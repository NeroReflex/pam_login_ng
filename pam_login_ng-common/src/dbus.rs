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

use zbus::{interface, Error as ZError};

use tokio::sync::Mutex;

use sys_mount::{Mount, UnmountDrop};

use login_ng::{
    storage::load_user_mountpoints,
    users::{get_user_by_name, os::unix::UserExt},
};

use std::{collections::HashMap, ffi::OsString};
use std::sync::Arc;
use thiserror::Error;

use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, LineEnding},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

use crate::mount::mount_all;

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
    _mounts: Vec<UnmountDrop<Mount>>,
}

pub struct Service {
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

#[interface(name = "org.zbus.login_ng1",
proxy(
    default_service = "org.zbus.login_ng",
    default_path = "/org/zbus/login_ng"
)

)]
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
                    "Successfully mounted {} device for user '{}'",
                    mounted_devices.len(),
                    user.name().to_string_lossy()
                );

                mounted_devices
            }
            None => vec![],
        };

        let user_session = UserSession {
            _mounts: mounted_devices,
        };

        let mut guard = self.sessions.lock().await;
        guard.insert(user.name().to_os_string(), user_session);

        println!(
            "Successfully opened session for user '{}'",
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
