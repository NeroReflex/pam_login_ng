/*
    login-ng A greeter written in rust that also supports autologin with systemd-homed
    Copyright (C) 2024  Denis Benato

    This program is free software; you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation; either version 2 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License along
    with this program; if not, write to the Free Software Foundation, Inc.,
    51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

extern crate rand;
extern crate sys_mount;
extern crate tokio;

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;

use sys_mount::{Mount, MountFlags, SupportedFilesystems, Unmount, UnmountDrop, UnmountFlags};

use login_ng::{
    mount::MountPoints,
    storage::{load_user_auth_data, load_user_mountpoints},
    users::{self, get_user_by_name, os::unix::UserExt},
};

use std::{collections::HashMap, ffi::OsString, io};
use std::{fs::create_dir, path::Path, sync::Arc};
use thiserror::Error;
use zbus::{connection, interface, Error as ZError};

use std::fs::File;
use std::io::Read;

use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, LineEnding},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Permission error: not running as the root user")]
    MissingPrivilegesError,

    #[error("DBus error: {0}")]
    ZbusError(#[from] ZError),

    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),
}

struct UserSession {
    mounts: Vec<UnmountDrop<Mount>>,
}

struct Service {
    priv_key: RsaPrivateKey,
    pub_key: RsaPublicKey,
    sessions: Arc<Mutex<HashMap<OsString, UserSession>>>,
}

impl Service {
    pub fn new(rsa_pkcs8: &str) -> Self {
        let priv_key = RsaPrivateKey::from_pkcs8_pem(rsa_pkcs8).unwrap();
        let pub_key = RsaPublicKey::from(&priv_key);

        let sessions = Arc::new(Mutex::new(HashMap::new()));

        Self {
            priv_key,
            pub_key,
            sessions,
        }
    }
}

/// Mounts a filesystem at the specified path.
///
/// This function takes a tuple containing information necessary for mounting a filesystem.
/// It checks if the specified mount path exists and is a directory. If the path does not exist,
/// it attempts to create it. Depending on whether the filesystem type is provided, it constructs
/// a mount operation accordingly.
///
/// # Parameters
///
/// - `data`: A tuple of four `String` values:
///   - `data.0`: The filesystem type (e.g., "ext4", "nfs"). If this is an empty string, the mount
///     operation will be performed without specifying a filesystem type.
///   - `data.1`: Additional data required for the mount operation (e.g., options for the mount).
///   - `data.2`: The source of the filesystem to mount (e.g., a device or remote filesystem).
///   - `data.3`: The target directory where the filesystem should be mounted.
///
/// # Returns
///
/// Returns a `Result<Mount, io::Error>`. On success, it returns a `Mount` object representing
/// the mounted filesystem. On failure, it returns an `io::Error` indicating what went wrong,
/// which could include issues with directory creation or mounting the filesystem.
///
/// # Errors
///
/// This function may fail if:
/// - The specified mount path does not exist and cannot be created due to permission issues.
/// - The mount operation fails due to invalid parameters or system errors.
///
/// # Example
///
/// ```rust
/// let mount_data = (String::from("ext4"), String::from("rw"), String::from("/dev/sda1"), String::from("/mnt/my_mount"));
/// match mount(mount_data) {
///     Ok(mount) => println!("Mounted successfully: {:?}", mount),
///     Err(e) => eprintln!("Failed to mount: {}", e),
/// }
/// ```
///
fn mount(data: (String, String, String, String)) -> io::Result<Mount> {
    let mount_path = Path::new(data.3.as_str());
    if !mount_path.exists() || !mount_path.is_dir() {
        // if the path is a file this will fail
        create_dir(mount_path)?;
    }

    match data.0.is_empty() {
        true => Mount::builder().mount(data.2.as_str(), mount_path.as_os_str()),
        false => Mount::builder()
            .fstype(data.0.as_str())
            .data(data.1.as_str())
            .mount(data.2.as_str(), data.3.as_str()),
    }
}

fn mount_all(mounts: MountPoints, username: String, homedir: String) -> Vec<UnmountDrop<Mount>> {
    let mut mounted_devices = vec![];

    for m in mounts
        .foreach(|a, b| {
            (
                b.fstype().clone(),
                b.flags().join(",").clone(),
                b.device().clone(),
                a.clone(),
            )
        })
        .iter()
    {
        match mount(m.clone()) {
            Ok(mount) => {
                println!(
                    "Mounted device {} into {} for user '{username}'",
                    m.2.as_str(),
                    m.3.as_str(),
                );

                // Make the mount temporary, so that it will be unmounted on drop.
                mounted_devices.push(mount.into_unmount_drop(UnmountFlags::DETACH));
            }
            Err(err) => {
                eprintln!(
                    "failed to mount device {} into {}: {}",
                    m.2.as_str(),
                    m.3.as_str(),
                    err
                );

                return vec![];
            }
        }
    }

    match mount((
        mounts.mount().fstype().clone(),
        mounts.mount().flags().join(","),
        mounts.mount().device().clone(),
        homedir,
    )) {
        Ok(mount) => {
            println!(
                "Mounted device {} on home directory for user '{username}'",
                mounts.mount().device().as_str(),
            );

            // Make the mount temporary, so that it will be unmounted on drop.
            mounted_devices.push(mount.into_unmount_drop(UnmountFlags::DETACH));
        }
        Err(err) => {
            eprintln!("failed to mount user directory: {err}");
            return vec![];
        }
    }

    mounted_devices
}

#[interface(name = "org.zbus.login_ng1")]
impl Service {
    async fn get_pubkey(&self) -> String {
        match self.pub_key.to_pkcs1_pem(LineEnding::CRLF) {
            Ok(key) => key,
            Err(err) => {
                println!("failed to serialize the RSA key: {err}");
                String::new()
            }
        }
    }

    async fn open_user_session(&mut self, user: &str, password: Vec<u8>) -> u32 {
        println!("Requested session for user '{user}' to be opened");

        let source = login_ng::storage::StorageSource::Username(String::from(user));

        let Some(user) = get_user_by_name(user) else {
            // cannot identify user
            return 7u32;
        };

        let password = match self.priv_key.decrypt(Pkcs1v15Encrypt, &password) {
            Ok(password) => {
                // TODO: defeat replay attacks!!!

                password
            }
            Err(err) => {
                eprintln!("Failed to decrypt data: {err}");
                return 2u32;
            }
        };

        let user_mounts = match load_user_mountpoints(&source) {
            Ok(user_cfg) => user_cfg,
            Err(err) => {
                eprintln!("Failed to load user mount data: {err}");
                return 3u32;
            }
        };

        // TODO: check for the mount to be approved by root
        // otherwise the user might mount everything he wants to
        // with every dmask, potentially compromising the
        // security and integrity of the whole system.

        // mount every directory in order or throw an error
        let mounted_devices = match user_mounts {
            Some(mounts) => {
                let mounted_devices = mount_all(
                    mounts,
                    user.name().to_string_lossy().to_string(),
                    user.home_dir().as_os_str().to_string_lossy().to_string(),
                );

                if mounted_devices.is_empty() {
                    eprintln!(
                        "Failed to mount one or more devices for user '{}'",
                        user.name().to_string_lossy()
                    );

                    return 4u32;
                }

                println!(
                    "Successfilly mounted {} device for user '{}'",
                    mounted_devices.len(),
                    user.name().to_string_lossy()
                );

                mounted_devices
            }
            None => vec![],
        };

        let user_session = UserSession {
            mounts: mounted_devices,
        };

        let mut guard = self.sessions.lock().await;
        guard.insert(user.name().to_os_string(), user_session);

        println!(
            "Successfilly opened session for user '{}'",
            user.name().to_string_lossy()
        );

        0u32 // OK
    }

    async fn close_user_session(&mut self, user: &str) -> u32 {
        println!("Requested session for user '{user}' to be closed");

        let Some(user) = get_user_by_name(user) else {
            // cannot identify user
            return 7u32;
        };

        let username = user.name().to_string_lossy();

        let mut guard = self.sessions.lock().await;

        // due to how directories are mounted discarding the session also umounts all mount points:
        // either remove the user session from the collection and destroy the session or
        // report to the caller that the requested session is already closed
        match guard.remove(user.name()) {
            Some(user_session) => drop(user_session),
            None => return 6u32,
        };

        println!("Successfully closed session for user '{username}'");

        0u32
    }
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    println!("Reading the private key...");
    let file_path = "/etc/login_ng/private_key_pkcs8.pem";
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    let read = file.read_to_string(&mut contents)?;
    println!("Read private key file of {read} bytes");

    if users::get_current_uid() != 0 {
        eprintln!("Application started without root privileges: aborting...");
        return Err(ServiceError::MissingPrivilegesError);
    }

    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Starting dbus service on socket {value}"),
        Err(err) => {
            eprintln!("Couldn't read dbus socket address: {err} - using default...");
            std::env::set_var(
                "DBUS_SESSION_BUS_ADDRESS",
                "unix:path=/run/dbus/system_bus_socket",
            );
        }
    }

    println!("Building the dbus object...");

    let dbus_conn = connection::Builder::session()
        .map_err(|err| ServiceError::ZbusError(err))?
        .name("org.zbus.login_ng")
        .map_err(|err| ServiceError::ZbusError(err))?
        .serve_at("/org/zbus/login_ng", Service::new(contents.as_str()))
        .map_err(|err| ServiceError::ZbusError(err))?
        .build()
        .await
        .map_err(|err| ServiceError::ZbusError(err))?;

    println!("Application running");

    // Create a signal listener for SIGTERM
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal handler");

    // Wait for a SIGTERM signal
    sigterm.recv().await;

    Ok(drop(dbus_conn))
}
