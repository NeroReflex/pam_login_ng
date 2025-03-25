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

use sys_mount::{Mount, UnmountDrop};

use login_ng::{
    storage::load_user_mountpoints,
    users::{get_user_by_name, os::unix::UserExt},
};

use std::{collections::{HashMap, hash_map::DefaultHasher}, ffi::OsString, sync::Arc};
use std::hash::{Hash, Hasher};
use thiserror::Error;

use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};

use crate::{mount::mount_all, security::*, result::*};

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
    priv_key: Arc<RsaPrivateKey>,
    pub_key: RsaPublicKey,
    one_time_tokens: HashMap<u64, Vec<u8>>,
    sessions: HashMap<OsString, UserSession>,
}

impl Service {
    pub fn new(rsa_pkcs8: &str) -> Self {
        let priv_key = Arc::new(RsaPrivateKey::from_pkcs8_pem(rsa_pkcs8).unwrap());
        let pub_key = RsaPublicKey::from(priv_key.as_ref());
        let one_time_tokens = HashMap::new();
        let sessions = HashMap::new();

        Self {
            priv_key,
            pub_key,
            one_time_tokens,
            sessions,
        }
    }
}

#[interface(
    name = "org.zbus.login_ng1",
    proxy(
        default_service = "org.zbus.login_ng",
        default_path = "/org/zbus/login_ng"
    )
)]
impl Service {
    async fn initiate_session(&mut self) -> String {
        let pub_pkcs1_pem = match self.pub_key.to_pkcs1_pem(LineEnding::CRLF) {
            Ok(key) => key,
            Err(err) => {
                println!("failed to serialize the RSA key: {err}");
                return String::new();
            }
        };

        let session = SessionPrelude::new(pub_pkcs1_pem);

        let otp = session.one_time_token();

        let mut hasher = DefaultHasher::new();
        otp.hash(&mut hasher);
        let key = hasher.finish();

        self.one_time_tokens.insert(key, otp);

        session.to_string()
    }

    async fn open_user_session(&mut self, user: &str, password: Vec<u8>) -> u32 {
        println!("Requested session for user '{user}' to be opened");

        let source = login_ng::storage::StorageSource::Username(String::from(user));

        let Some(user) = get_user_by_name(user) else {
            return ServiceOperationResult::CannotIdentifyUser.into();
        };

        if self.sessions.contains_key(&user.name().to_os_string()) {
            return ServiceOperationResult::SessionAlreadyOpened.into();
        }

        let (otp, password) = match SessionPrelude::decrypt(self.priv_key.clone(), password) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Failed to decrypt data: {err}");
                return ServiceOperationResult::DataDecryptionFailed.into();
            }
        };

        // check the OTP to be available to defeat replay attacks
        let mut hasher = DefaultHasher::new();
        otp.hash(&mut hasher);
        match self.one_time_tokens.remove(&hasher.finish()) {
            Some(stored) => if stored != otp {
                return ServiceOperationResult::EncryptionError.into()
            },
            None => return ServiceOperationResult::EncryptionError.into(),
        }

        let user_mounts = match load_user_mountpoints(&source) {
            Ok(user_cfg) => user_cfg,
            Err(err) => {
                eprintln!("Failed to load user mount data: {err}");
                return ServiceOperationResult::CannotLoadUserMountError.into();
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

                    return ServiceOperationResult::MountError.into();
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

        self.sessions
            .insert(user.name().to_os_string(), user_session);

        println!(
            "Successfully opened session for user '{}'",
            user.name().to_string_lossy()
        );

        return ServiceOperationResult::Ok.into();
    }

    async fn close_user_session(&mut self, user: &str) -> u32 {
        println!("Requested session for user '{user}' to be closed");

        let Some(user) = get_user_by_name(user) else {
            return ServiceOperationResult::CannotIdentifyUser.into();
        };

        let username = user.name().to_string_lossy();

        // due to how directories are mounted discarding the session also umounts all mount points:
        // either remove the user session from the collection and destroy the session or
        // report to the caller that the requested session is already closed
        match self.sessions.remove(user.name()) {
            Some(user_session) => drop(user_session),
            None => return ServiceOperationResult::SessionAlreadyClosed.into(),
        };

        println!("Successfully closed session for user '{username}'");

        return ServiceOperationResult::Ok.into();
    }
}
