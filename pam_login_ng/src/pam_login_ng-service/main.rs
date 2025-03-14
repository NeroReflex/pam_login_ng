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

use rand::rngs::OsRng;

use sys_mount::{Mount, MountFlags, SupportedFilesystems, Unmount, UnmountFlags};

use login_ng::{
    storage::{load_user_auth_data, load_user_mountpoints},
    users::{self, get_user_by_name, os::unix::UserExt},
};

use thiserror::Error;

use std::{collections::HashMap, io};
use std::sync::Arc;
use tokio::sync::Mutex;

use std::future::pending;
use zbus::{connection, interface, Error as ZError};

use rsa::{
    pkcs1::EncodeRsaPublicKey, pkcs8::LineEnding, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Permission error: not running as the root user")]
    MissingPrivilegesError,

    #[error("DBus error: {0}")]
    ZbusError(#[from] ZError),
}

struct UserSession {}

struct Service {
    priv_key: RsaPrivateKey,
    pub_key: RsaPublicKey,
    pub_key_string: String,
    sessions: Arc<Mutex<HashMap<String, UserSession>>>,
}

impl Service {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 8192).expect("failed to generate a key");
        let pub_key = RsaPublicKey::from(&priv_key);
        let pub_key_string = pub_key.to_pkcs1_pem(LineEnding::CRLF).unwrap();

        let sessions = Arc::new(Mutex::new(HashMap::new()));

        Self {
            priv_key,
            pub_key,
            pub_key_string,
            sessions,
        }
    }
}

fn mount(data: (String, String, String, String)) -> io::Result<Mount> {
    match data.0.is_empty() {
        true => Mount::builder()
            .mount(data.2.as_str(), data.3.as_str()),
        false => Mount::builder()
            .fstype(data.0.as_str())
            .data(data.1.as_str())
            .mount(data.2.as_str(), data.3.as_str()
        )
    }
}

#[interface(name = "org.zbus.pam_login_ng")]
impl Service {
    async fn get_pubkey(&self) -> String {
        self.pub_key_string.clone()
    }

    async fn open_user_session(&mut self, user: &str, password: Vec<u8>) -> u32 {
        let source = login_ng::storage::StorageSource::Username(String::from(user));

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

        let mut mounted_devices = vec![];

        // mount every directory in order or throw an error
        match get_user_by_name(user) {
            Some(user) => {
                if let Some(mounts) = user_mounts {
                    let staged_mounts = mounts.foreach(|a, b| 
                        (b.fstype().clone(), b.flags().join(",").clone(), b.device().clone(), a.clone())
                    );
        
                    for m in staged_mounts {
                        match mount(m) {
                            Ok(mount) => {
                                // Make the mount temporary, so that it will be unmounted on drop.
                                mounted_devices.push(mount.into_unmount_drop(UnmountFlags::DETACH));
                            }
                            Err(err) => {
                                eprintln!("failed to mount device {} into {}: {}", m.2.as_str(), m.3.as_str(), err);
                                return 4u32;
                            }
                        }
                    }
                }
                user.home_dir();
            }
            None => {},
        }
        

        0u32 // OK
    }

    async fn close_user_session(&mut self, user: &str) -> u32 {
        0u32
    }
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    if users::get_current_uid() != 0 {
        return Err(ServiceError::MissingPrivilegesError);
    }

    let service = Service::new();

    let _conn = connection::Builder::session()
        .map_err(|err| ServiceError::ZbusError(err))?
        .name("org.zbus.pam_login_ng")
        .map_err(|err| ServiceError::ZbusError(err))?
        .serve_at("/org/zbus/pam_login_ng", service)
        .map_err(|err| ServiceError::ZbusError(err))?
        .build()
        .await
        .map_err(|err| ServiceError::ZbusError(err))?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
