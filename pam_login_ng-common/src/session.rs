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

use tokio::sync::RwLock;
use zbus::interface;

use sys_mount::{Mount, UnmountDrop};

use login_ng::{
    storage::load_user_mountpoints,
    users::{get_user_by_name, gid_t, os::unix::UserExt, uid_t},
};

use std::hash::{Hash, Hasher};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    ffi::OsString,
    sync::Arc,
};

use rsa::{
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};

use crate::{
    mount::{mount_all, MountAuth},
    result::*,
    security::*,
};

struct UserSession {
    _mounts: Vec<UnmountDrop<Mount>>,
}

pub struct Sessions {
    mounts_auth: Arc<RwLock<MountAuth>>,
    priv_key: Arc<RsaPrivateKey>,
    pub_key: RsaPublicKey,
    one_time_tokens: HashMap<u64, Vec<u8>>,
    sessions: HashMap<OsString, UserSession>,
}

impl Sessions {
    pub fn new(mounts_auth: Arc<RwLock<MountAuth>>, key_string: &str) -> Self {
        let priv_key = Arc::new(RsaPrivateKey::from_pkcs1_pem(key_string).unwrap());
        let pub_key = RsaPublicKey::from(priv_key.as_ref());
        let one_time_tokens = HashMap::new();
        let sessions = HashMap::new();

        Self {
            mounts_auth,
            priv_key,
            pub_key,
            one_time_tokens,
            sessions,
        }
    }
}

#[interface(
    name = "org.neroreflex.login_ng_session1",
    proxy(
        default_service = "org.neroreflex.login_ng_session",
        default_path = "/org/zbus/login_ng_session"
    )
)]
impl Sessions {
    async fn initiate_session(&mut self) -> String {
        println!("🔓 Requested initialization of a new session");

        let pub_pkcs1_pem = match self.pub_key.to_pkcs1_pem(LineEnding::CRLF) {
            Ok(key) => key,
            Err(err) => {
                println!("❌ Error serializing the RSA key: {err}");
                return String::new();
            }
        };

        let session = SessionPrelude::new(pub_pkcs1_pem);

        let otp = session.one_time_token();

        let mut hasher = DefaultHasher::new();
        otp.hash(&mut hasher);
        let key = hasher.finish();

        let serialized = match serde_json::to_string(&session) {
            Ok(serialized) => serialized,
            Err(err) => {
                println!("❌ Error serializing the session one time token: {err}");
                return String::new();
            }
        };

        self.one_time_tokens.insert(key, otp);

        println!("✅ Created one time token {key}");

        serialized
    }

    async fn open_user_session(
        &mut self,
        username: &str,
        password: Vec<u8>,
    ) -> (u32, uid_t, gid_t) {
        println!("👤 Requested session for user '{username}' to be opened");

        let source = login_ng::storage::StorageSource::Username(String::from(username));

        let Some(user) = get_user_by_name(username) else {
            return (ServiceOperationResult::CannotIdentifyUser.into(), 0, 0);
        };

        if self.sessions.contains_key(&user.name().to_os_string()) {
            return (ServiceOperationResult::SessionAlreadyOpened.into(), 0, 0);
        }

        let (otp, password) = match SessionPrelude::decrypt(self.priv_key.clone(), password) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("❌ Error in decrypting data: {err}");
                return (ServiceOperationResult::DataDecryptionFailed.into(), 0, 0);
            }
        };

        // check the OTP to be available to defeat replay attacks
        let mut hasher = DefaultHasher::new();
        otp.hash(&mut hasher);
        match self.one_time_tokens.remove(&hasher.finish()) {
            Some(stored) => {
                if stored != otp {
                    eprintln!("🚫 The provided temporary OTP key couldn't be verified");
                    return (ServiceOperationResult::EncryptionError.into(), 0, 0);
                }
            }
            None => {
                println!("❌ Error in finding the provided temporary OTP key");
                return (ServiceOperationResult::EncryptionError.into(), 0, 0);
            }
        }

        let user_mounts = match load_user_mountpoints(&source) {
            Ok(user_cfg) => user_cfg,
            Err(err) => {
                eprintln!("❌ Error loading user mount data: {err}");
                return (
                    ServiceOperationResult::CannotLoadUserMountError.into(),
                    0,
                    0,
                );
            }
        };

        // Check for the mount to be approved by root
        // otherwise the user might mount everything he wants to
        // with every dmask, potentially compromising the
        // security and integrity of the whole system.
        if let Some(mounts) = user_mounts.clone() {
            let hash_to_check = mounts.hash();
            if !self
                .mounts_auth
                .read()
                .await
                .authorized(username, hash_to_check)
            {
                eprintln!("🚫 User {username} attempted an unauthorized mount {hash_to_check}.");
                return (ServiceOperationResult::UnauthorizedMount.into(), 0, 0);
            }
        };

        let mounted_devices = mount_all(
            user_mounts,
            password,
            user.uid(),
            user.primary_group_id(),
            user.name().to_string_lossy().to_string(),
            user.home_dir().as_os_str().to_string_lossy().to_string(),
        );

        if mounted_devices.is_empty() {
            eprintln!("❌ Error mounting one or more devices for user {username}");

            return (ServiceOperationResult::MountError.into(), 0, 0);
        }

        let user_session = UserSession {
            _mounts: mounted_devices,
        };

        self.sessions
            .insert(user.name().to_os_string(), user_session);

        println!("✅ Successfully opened session for user {username}");

        (
            ServiceOperationResult::Ok.into(),
            user.uid(),
            user.primary_group_id(),
        )
    }

    async fn close_user_session(&mut self, user: &str) -> u32 {
        println!("👤 Requested session for user '{user}' to be closed");

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

        println!("✅ Successfully closed session for user '{username}'");

        ServiceOperationResult::Ok.into()
    }
}
