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

extern crate pam;
extern crate pam_login_ng_common;

use pam::{
    constants::{PamFlag, PamResultCode, *},
    conv::Conv,
    module::{PamHandle, PamHooks},
    pam_try,
};
use pam_login_ng_common::{
    login_ng::{
        storage::{load_user_auth_data, StorageSource},
        user::UserAuthData,
    },
    result::ServiceOperationResult,
    security::SessionPrelude,
};
use pam_login_ng_common::{
    session::SessionsProxy,
    zbus::{Connection, Result as ZResult},
};

use std::{ffi::CStr, sync::Once};
use tokio::runtime::Runtime;

static INIT: Once = Once::new();
static mut RUNTIME: Option<Runtime> = None;

struct PamQuickEmbedded;
pam::pam_hooks!(PamQuickEmbedded);

impl PamQuickEmbedded {
    pub(crate) fn load_user_auth_data_from_username(
        username: &String,
    ) -> Result<UserAuthData, PamResultCode> {
        match username.as_str() {
            "" => Err(PamResultCode::PAM_USER_UNKNOWN),
            "root" => Err(PamResultCode::PAM_USER_UNKNOWN),
            // load login-ng data and skip the user if it's not set
            _ => match load_user_auth_data(&StorageSource::Username(username.clone())) {
                Ok(load_res) => match load_res {
                    Some(auth_data) => match auth_data.has_main() {
                        true => Ok(auth_data),
                        false => Err(PamResultCode::PAM_USER_UNKNOWN),
                    },
                    None => Err(PamResultCode::PAM_USER_UNKNOWN),
                },
                Err(_err) => Err(PamResultCode::PAM_USER_UNKNOWN),
            },
        }
    }

    pub(crate) async fn open_session_for_user(
        user: &String,
        plain_main_password: String,
    ) -> ZResult<ServiceOperationResult> {
        let connection = Connection::session().await?;

        let proxy = SessionsProxy::new(&connection).await?;

        let pk = proxy.initiate_session().await?;

        // return an unknown error if the service was unable to serialize the RSA public key
        if pk.is_empty() {
            return Ok(ServiceOperationResult::EmptyPubKey);
        }

        let session_prelude = SessionPrelude::from_string(pk.as_str());

        let Ok(encrypted_password) = session_prelude.encrypt(plain_main_password) else {
            return Ok(ServiceOperationResult::EncryptionError);
        };

        let reply = proxy
            .open_user_session(user.as_str(), encrypted_password)
            .await?;

        Ok(ServiceOperationResult::from(reply))
    }

    pub(crate) async fn close_session_for_user(user: &String) -> ZResult<u32> {
        let connection = Connection::session().await?;

        let proxy = SessionsProxy::new(&connection).await?;
        let reply = proxy.close_user_session(user.as_str()).await?;

        Ok(reply)
    }

    pub(crate) async fn handle_env(pamh: &mut PamHandle) {}
}

impl PamHooks for PamQuickEmbedded {
    fn sm_close_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            Ok(value) => pamh.log(
                pam::module::LogLevel::Debug,
                format!("Starting dbus service on socket {value}"),
            ),
            Err(err) => {
                pamh.log(
                    pam::module::LogLevel::Error,
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
            Ok(res) => res,
            Err(err) => {
                // If the error is PAM_SUCCESS, we should not return an error
                if err != PamResultCode::PAM_SUCCESS {
                    eprintln!("login_ng: open_session: get_user failed");
                    return err;
                }

                // Attempt to get the user item
                match pamh.get_item::<pam::items::User>() {
                    Ok(Some(username)) => username.to_string_lossy(),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        unsafe {
            match &RUNTIME {
                Some(runtime) => runtime.block_on(async {
                    match PamQuickEmbedded::close_session_for_user(&String::from(username)).await {
                        Ok(result) => match ServiceOperationResult::from(result) {
                            ServiceOperationResult::Ok => PamResultCode::PAM_SUCCESS,
                            _ => PamResultCode::PAM_SERVICE_ERR,
                        },
                        Err(_) => PamResultCode::PAM_SERVICE_ERR,
                    }
                }),
                None => PamResultCode::PAM_SERVICE_ERR,
            }
        }
    }

    fn sm_open_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        pamh.log(
            pam::module::LogLevel::Debug,
            "login_ng: open_session: enter".to_string(),
        );

        match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
            Ok(value) => pamh.log(
                pam::module::LogLevel::Info,
                format!("Starting dbus service on socket {value}"),
            ),
            Err(err) => {
                pamh.log(
                    pam::module::LogLevel::Error,
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
            Ok(res) => res,
            Err(err) => {
                // If the error is PAM_SUCCESS, we should not return an error
                if err != PamResultCode::PAM_SUCCESS {
                    pamh.log(
                        pam::module::LogLevel::Error,
                        "login_ng: open_session: get_user failed".to_string(),
                    );
                    return err;
                }

                // Attempt to get the user item
                match pamh.get_item::<pam::items::User>() {
                    Ok(Some(username)) => username.to_string_lossy(),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        pamh.log(
            pam::module::LogLevel::Debug,
            format!("login_ng: open_session: user {username}"),
        );

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg =
            match PamQuickEmbedded::load_user_auth_data_from_username(&username.to_string()) {
                Ok(user_cfg) => user_cfg,
                Err(pam_err_code) => return pam_err_code,
            };

        pamh.log(
            pam::module::LogLevel::Debug,
            format!("login_ng: open_session: loaded data for user {username}"),
        );
        // TODO: set environment variables

        unsafe {
            match &RUNTIME {
                Some(runtime) => runtime.block_on(async {
                    match PamQuickEmbedded::open_session_for_user(
                        &String::from(username),
                        String::from(""), // TODO: fetch the real passowrd
                    )
                    .await
                    {
                        Ok(result) => {
                            match result {
                                ServiceOperationResult::Ok => {
                                    pamh.log(
                                        pam::module::LogLevel::Info,
                                        "login_ng: open_session: pam_login_ng-service was successful".to_string(),
                                    );

                                    PamResultCode::PAM_SUCCESS
                                },
                                err => {
                                    pamh.log(
                                        pam::module::LogLevel::Error,
                                        format!(
                                            "login_ng: open_session: pam_login_ng-service errored: {err}"
                                        ),
                                    );

                                    PamResultCode::PAM_SERVICE_ERR
                                },
                            }
                        }
                        Err(err) => {
                            pamh.log(
                                pam::module::LogLevel::Error,
                                format!(
                                    "login_ng: open_session: pam_login_ng-service dbus error: {err}"
                                ),
                            );

                            PamResultCode::PAM_SERVICE_ERR
                        }
                    }
                }),
                None => PamResultCode::PAM_SERVICE_ERR,
            }
        }
    }

    fn sm_setcred(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let username = match pamh.get_user(None) {
            Ok(res) => res,
            Err(err) => {
                // If the error is PAM_SUCCESS, we should not return an error
                if err != PamResultCode::PAM_SUCCESS {
                    return err;
                }

                // Attempt to get the user item
                match pamh.get_item::<pam::items::User>() {
                    Ok(Some(username)) => username.to_string_lossy(),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg =
            match PamQuickEmbedded::load_user_auth_data_from_username(&username.to_string()) {
                Ok(user_cfg) => user_cfg,
                Err(pam_err_code) => return pam_err_code,
            };

        // TODO: set environment variables

        PamResultCode::PAM_SUCCESS
    }

    /*
        fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
            println!("account management");
            PamResultCode::PAM_SUCCESS
        }
    */
    fn sm_authenticate(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        let username = match pamh.get_user(None) {
            Ok(res) => res,
            Err(err) => {
                // If the error is PAM_SUCCESS, we should not return an error
                if err != PamResultCode::PAM_SUCCESS {
                    return err;
                }

                // Attempt to get the user item
                match pamh.get_item::<pam::items::User>() {
                    Ok(Some(username)) => username.to_string_lossy(),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg =
            match PamQuickEmbedded::load_user_auth_data_from_username(&username.to_string()) {
                Ok(user_cfg) => user_cfg,
                Err(pam_err_code) => return pam_err_code,
            };

        // first of all check if the empty password is valid
        if let Ok(main_password) = user_cfg.main_by_auth(&Some(String::new())) {
            if let Ok(password_matches) = user_cfg.check_main(&main_password) {
                if password_matches {
                    return PamResultCode::PAM_SUCCESS;
                }
            }
        }

        // if the empty password was not valid then continue and ask for a password
        let conv = match pamh.get_item::<Conv>() {
            Ok(Some(conv)) => conv,
            Ok(None) => {
                pamh.log(
                    pam::module::LogLevel::Critical,
                    "No conv available".to_string(),
                );

                return PamResultCode::PAM_SERVICE_ERR;
            }
            Err(err) => {
                pamh.log(
                    pam::module::LogLevel::Error,
                    format!("Couldn't get pam_conv: pam error {err}"),
                );

                return err;
            }
        };

        // NOTE: if main_by_auth returns a main passowrd the authentication was successful:
        // there is no need to check if the returned main password is the same as the stored one.
        match pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "Password: "))
            .map(|cstr| cstr.to_str().map(|s| s.to_string()))
        {
            Some(Ok(password)) => user_cfg
                .main_by_auth(&Some(password))
                .map(|_| PamResultCode::PAM_SUCCESS)
                .unwrap_or(PamResultCode::PAM_AUTH_ERR),
            Some(Err(_err)) => PamResultCode::PAM_CRED_INSUFFICIENT,
            None => PamResultCode::PAM_CRED_INSUFFICIENT,
        }
    }
}
