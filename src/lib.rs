/*
    pam_polyauth: A pam module written in rust that supports multiple
    authentication modes (including autologin).

    Copyright (C) 2024-2025  Denis Benato

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

pub mod auth;
pub mod command;
pub mod error;
pub mod mount;
pub mod pam;
pub mod storage;
pub mod user;

#[cfg(test)]
pub mod tests;

use hkdf::*;
use sha2::Sha256;
//use users::{os::unix::UserExt, User};

pub const LIBRARY_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn derive_key(input: &str, salt: &[u8]) -> [u8; 32] {
    // Create an HKDF instance with SHA-256 as the hash function
    let hkdf = Hkdf::<Sha256>::new(Some(salt), input.as_bytes());

    // Prepare a buffer for the derived key
    let mut okm = [0u8; 32]; // Output key material (32 bytes)

    // Extract the key material
    hkdf.expand(&[], &mut okm).expect("Failed to expand key");

    okm
}

pub(crate) fn password_to_vec(password: &String) -> Vec<u8> {
    password.as_str().into()
}

pub(crate) fn vec_to_password(vec: &Vec<u8>) -> String {
    String::from_utf8_lossy(vec.as_slice()).to_string()
}

// this MUST be implemented and used because entering invalid strings can be a security hole (see lossy_utf8)
pub(crate) fn is_valid_password(password: &String) -> bool {
    vec_to_password(password_to_vec(password).as_ref()) == password.clone()
}

/*
pub fn valid_users() -> Vec<User> {
    unsafe { crate::users::all_users() }
        .filter_map(|user| {
            if user.name() == "nobody" {
                return None;
            }

            if user.shell() == OsString::from("/bin/false") {
                return None;
            }

            let uid = user.uid();
            if uid == 0 || uid < 1000 || uid == crate::users::uid_t::MAX {
                return None;
            }

            Some(user)
        })
        .collect()
}
*/

use crate::{
    pam::{
        result::ServiceOperationResult, security::SessionPrelude, session::SessionsProxy,
        XDG_RUNTIME_DIR_PATH,
    },
    storage::{load_user_auth_data, StorageSource},
};

pub(crate) extern crate pam as pam_binding;

use pam_binding::{
    constants::{PamFlag, PamMessageStyle},
    conv::Conv,
    error::{PamErrorCode, PamResult},
    module::{PamHandle, PamHooks},
    pam_hooks,
};

use zbus::{Connection, Result as ZResult};

use users::{gid_t, uid_t};

use std::{borrow::Cow, ffi::CStr, path::PathBuf, sync::Once};
use tokio::runtime::Runtime;

static INIT: Once = Once::new();
static mut RUNTIME: Option<Runtime> = None;

struct PamQuickEmbedded;
pam_hooks!(PamQuickEmbedded);

impl PamQuickEmbedded {
    pub(crate) async fn open_session_for_user(
        user: &String,
        plain_main_password: String,
    ) -> ZResult<(ServiceOperationResult, uid_t, gid_t)> {
        let connection = Connection::session().await?;

        let proxy = SessionsProxy::new(&connection).await?;

        let pk = proxy.initiate_session().await?;

        // return an unknown error if the service was unable to serialize the RSA public key
        if pk.is_empty() {
            return Ok((ServiceOperationResult::EmptyPubKey, 0, 0));
        }

        let Ok(session_prelude) = serde_json::from_str::<SessionPrelude>(pk.as_str()) else {
            return Ok((ServiceOperationResult::SerializationError, 0, 0));
        };

        let Ok(encrypted_password) = session_prelude.encrypt(plain_main_password) else {
            return Ok((ServiceOperationResult::EncryptionError, 0, 0));
        };

        let reply = proxy
            .open_user_session(user.as_str(), encrypted_password)
            .await?;

        Ok((ServiceOperationResult::from(reply.0), reply.1, reply.2))
    }

    pub(crate) async fn close_session_for_user(user: &String) -> ZResult<u32> {
        let connection = Connection::session().await?;

        let proxy = SessionsProxy::new(&connection).await?;
        let reply = proxy.close_user_session(user.as_str()).await?;

        Ok(reply)
    }

    pub(crate) async fn is_user_polyauth_enabled(user: &String) -> ZResult<u32> {
        let connection = Connection::session().await?;

        let proxy = SessionsProxy::new(&connection).await?;
        let reply = proxy.is_user_polyauth_enabled(user.as_str()).await?;

        Ok(reply)
    }
}

impl PamHooks for PamQuickEmbedded {
    fn sm_close_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResult<()> {
        pamh.log(
            pam_binding::module::LogLevel::Debug,
            "polyauth: sm_close_session: enter".to_string(),
        );

        match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            Ok(value) => pamh.log(
                pam_binding::module::LogLevel::Debug,
                format!("Starting dbus service on socket {value}"),
            ),
            Err(err) => {
                pamh.log(
                    pam_binding::module::LogLevel::Debug,
                    format!("Couldn't read dbus socket address: {err} - using default..."),
                );
                std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    "unix:path=/run/dbus/system_bus_socket",
                );
            }
        }

        INIT.call_once(|| {
            // Initialize the Tokio runtime
            unsafe {
                RUNTIME = Some(Runtime::new().unwrap());
            }
        });

        let username = match pamh.get_user(None) {
            Ok(Some(res)) => res,
            Ok(None) => match pamh.get_item::<pam_binding::items::User>() {
                Ok(Some(username)) => username.to_string_lossy().to_string(),
                Ok(None) => return Err(PamErrorCode::AUTH_ERR),
                Err(err) => return Err(err),
            },
            Err(err) => {
                pamh.log(
                    pam_binding::module::LogLevel::Error,
                    format!("polyauth: open_session: get_user failed: {err}"),
                );
                return Err(err);
            }
        };

        unsafe {
            let runtime_ptr = &raw const RUNTIME;
            match &*runtime_ptr {
                Some(runtime) => runtime.block_on(async {
                    let Ok(result) =
                        PamQuickEmbedded::close_session_for_user(&String::from(username)).await
                    else {
                        return Err(PamErrorCode::SERVICE_ERR);
                    };

                    match ServiceOperationResult::from(result) {
                        ServiceOperationResult::Ok => Ok(()),
                        _ => Err(PamErrorCode::SERVICE_ERR),
                    }
                }),
                None => Err(PamErrorCode::SERVICE_ERR),
            }
        }
    }

    fn sm_open_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResult<()> {
        pamh.log(
            pam_binding::module::LogLevel::Debug,
            "polyauth: sm_open_session: enter".to_string(),
        );

        match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            Ok(value) => pamh.log(
                pam_binding::module::LogLevel::Info,
                format!("Starting dbus service on socket {value}"),
            ),
            Err(err) => {
                pamh.log(
                    pam_binding::module::LogLevel::Debug,
                    format!("Couldn't read dbus socket address: {err} - using default..."),
                );
                std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    "unix:path=/run/dbus/system_bus_socket",
                );
            }
        }

        INIT.call_once(|| {
            // Initialize the Tokio runtime
            unsafe {
                RUNTIME = Some(Runtime::new().unwrap());
            }
        });

        let username = match pamh.get_user(None) {
            Ok(Some(res)) => res,
            Ok(None) => match pamh.get_item::<pam_binding::items::User>() {
                Ok(Some(username)) => username.to_string_lossy().to_string(),
                Ok(None) => {
                    pamh.log(
                        pam_binding::module::LogLevel::Warning,
                        "polyauth: sm_open_session: get_item<User> returned nothing but did not fail".to_string(),
                    );

                    return Err(PamErrorCode::AUTH_ERR);
                }
                Err(err) => {
                    pamh.log(
                        pam_binding::module::LogLevel::Warning,
                        format!("polyauth: sm_open_session: get_item<User> failed {err}"),
                    );

                    return Err(err);
                }
            },
            Err(err) => {
                pamh.log(
                    pam_binding::module::LogLevel::Error,
                    "polyauth: sm_open_session: get_user failed".to_string(),
                );
                return Err(err);
            }
        };

        pamh.log(
            pam_binding::module::LogLevel::Debug,
            format!("polyauth: sm_open_session: user {username}"),
        );

        unsafe {
            let runtime_ptr = &raw const RUNTIME;
            let runtime = (&*runtime_ptr).as_ref().ok_or(PamErrorCode::SERVICE_ERR)?;
            runtime.block_on(async {
                let cred_data = format!("{}-polyauth", username);
                let main_password = pamh.get_data::<String>(cred_data.as_str()).map_err(|err| {
                    pamh.log(
                        pam_binding::module::LogLevel::Error,
                        format!(
                            "polyauth: sm_open_session: get_data error: {err}"
                        ),
                    );

                    err
                })?;

                let (result, uid, gid) = PamQuickEmbedded::open_session_for_user(
                    &String::from(username),
                    main_password.clone(),
                )
                .await
                .map_err(|err| {
                    pamh.log(
                        pam_binding::module::LogLevel::Error,
                        format!(
                            "polyauth: sm_open_session: pam_polyauth-service dbus error: {err}"
                        ),
                    );

                    PamErrorCode::SERVICE_ERR
                })?;

                match result {
                    ServiceOperationResult::Ok => {
                        pamh.log(
                            pam_binding::module::LogLevel::Info,
                            "polyauth: sm_open_session: pam_polyauth-service was successful".to_string(),
                        );

                        let uid = uid;
                        let _gid = gid;

                        let xdg_user_path = PathBuf::from(XDG_RUNTIME_DIR_PATH).join(format!("{uid}"));
                        match pamh.env_set(Cow::from("XDG_RUNTIME_DIR"), xdg_user_path.to_string_lossy()) {
                            Ok(_) => pamh.log(
                                    pam_binding::module::LogLevel::Info,
                                    "polyauth: sm_open_session: session opened and XDG_RUNTIME_DIR set".to_string(),
                                ),
                            Err(err) => pamh.log(
                                    pam_binding::module::LogLevel::Warning,
                                    format!("polyauth: sm_open_session: could not set XDG_RUNTIME_DIR: {err}"),
                                ),
                        }

                        Ok(())
                    },
                    err => {
                        pamh.log(
                            pam_binding::module::LogLevel::Error,
                            format!(
                                "polyauth: sm_open_session: pam_polyauth-service errored: {err}"
                            ),
                        );

                        Err(PamErrorCode::SERVICE_ERR)
                    },
                }
            })
        }
    }

    fn sm_setcred(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResult<()> {
        pamh.log(
            pam_binding::module::LogLevel::Debug,
            format!("polyauth: sm_setcred: enter"),
        );

        match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            Ok(value) => pamh.log(
                pam_binding::module::LogLevel::Debug,
                format!("Using dbus service on socket {value}"),
            ),
            Err(err) => {
                pamh.log(
                    pam_binding::module::LogLevel::Debug,
                    format!("Couldn't read dbus socket address: {err} - using default..."),
                );
                std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    "unix:path=/run/dbus/system_bus_socket",
                );
            }
        }

        INIT.call_once(|| {
            // Initialize the Tokio runtime
            unsafe {
                RUNTIME = Some(Runtime::new().unwrap());
            }
        });

        let username = match pamh.get_user(None)? {
            Some(res) => res,
            None => match pamh.get_item::<pam_binding::items::User>()? {
                Some(username) => username.to_string_lossy().to_string(),
                None => {
                    pamh.log(
                        pam_binding::module::LogLevel::Error,
                        "polyauth: sm_setcred: get_item<User> returned nothing but did not fail"
                            .to_string(),
                    );

                    return Err(PamErrorCode::AUTH_ERR);
                }
            },
        };

        // Check if the user is polyauth-enabled by asking the service
        unsafe {
            let runtime_ptr = &raw const RUNTIME;
            match &*runtime_ptr {
                Some(runtime) => runtime.block_on(async {
                    let Ok(result) =
                        PamQuickEmbedded::is_user_polyauth_enabled(&String::from(username)).await
                    else {
                        return Err(PamErrorCode::SERVICE_ERR);
                    };

                    match ServiceOperationResult::from(result) {
                        ServiceOperationResult::Ok => Ok(()),
                        _ => Err(PamErrorCode::USER_UNKNOWN),
                    }
                }),
                None => Err(PamErrorCode::SERVICE_ERR),
            }
        }
    }

    /*
        fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
            println!("account management");
            PamResultCode::PAM_SUCCESS
        }
    */

    fn sm_authenticate(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResult<()> {
        pamh.log(
            pam_binding::module::LogLevel::Error,
            format!("polyauth: sm_authenticate: enter"),
        );

        let username = match pamh.get_user(None).map_err(|err| {
            pamh.log(
                pam_binding::module::LogLevel::Error,
                format!("polyauth: open_session: get_user failed: {err}"),
            );

            err
        })? {
            Some(username) => username,
            None => pamh
                .get_item::<pam_binding::items::User>()?
                .ok_or({
                    pamh.log(
                        pam_binding::module::LogLevel::Error,
                        format!("polyauth: sm_authenticate: get_item<User> returned nothing"),
                    );

                    PamErrorCode::AUTH_ERR
                })?
                .to_string_lossy()
                .to_string(),
        };

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg = match load_user_auth_data(&StorageSource::Username(username.to_string())) {
            Ok(Some(auth_data)) if auth_data.has_main() => auth_data,
            _ => return Err(PamErrorCode::USER_UNKNOWN),
        };

        let cred_data = format!("{}-polyauth", username);

        // NOTE: if main_by_auth returns a main password the authentication was successful:
        // there is no need to check if the returned main password is the same as the stored one.
        // This will also used below for the user-provided string.
        if let Ok(main_password) = user_cfg.main_by_auth(&Some(String::new())) {
            pamh.set_data(cred_data.as_str(), Box::new(main_password))
                .map_err(|err| {
                    pamh.log(
                        pam_binding::module::LogLevel::Error,
                        format!("polyauth: sm_authenticate: set_data error {err}"),
                    );

                    err
                })?;

            return Ok(());
        }

        // if the empty password was not valid then continue and ask for a password
        let conv = pamh
            .get_item::<Conv>()
            .map_err(|err| {
                pamh.log(
                    pam_binding::module::LogLevel::Error,
                    format!("Couldn't get pam_conv: pam error {err}"),
                );

                err
            })?
            .ok_or({
                pamh.log(
                    pam_binding::module::LogLevel::Critical,
                    "No conv available".to_string(),
                );

                PamErrorCode::SERVICE_ERR
            })?;

        let password = conv
            .send(PamMessageStyle::PAM_PROMPT_ECHO_OFF, "Password: ")
            .map(|cstr| cstr.map(|a| a.to_string_lossy()).map(|s| s.to_string()))?
            .ok_or(PamErrorCode::CRED_INSUFFICIENT)?;

        let main_password = user_cfg.main_by_auth(&Some(password)).map_err(|err| {
            pamh.log(
                pam_binding::module::LogLevel::Error,
                format!("polyauth: sm_authenticate: authentication error: {err}"),
            );

            PamErrorCode::AUTH_ERR
        })?;
        pamh.set_data(cred_data.as_str(), Box::new(main_password))
            .map_err(|err| {
                pamh.log(
                    pam_binding::module::LogLevel::Error,
                    format!("polyauth: sm_authenticate: set_data error {err}"),
                );

                err
            })
    }
}
