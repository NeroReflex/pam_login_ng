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
extern crate rand;
pub extern crate zbus;

use login_ng::storage::{load_user_auth_data, StorageSource};
use login_ng::user::UserAuthData;
use pam::constants::{PamFlag, PamResultCode, *};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
use pam::pam_try;
use rsa::pkcs1::DecodeRsaPublicKey;
use std::ffi::CStr;
use std::sync::Once;
use tokio::runtime::Runtime;
use zbus::{proxy, Connection, Result as ZResult};

use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

static INIT: Once = Once::new();
static mut RUNTIME: Option<Runtime> = None;

#[repr(C)]
enum ServiceOperationResult {
    Ok = 0,
    PubKeyError = 1,
    DataDecryptionFailed = 2,
    CannotLoadUserMountError = 3,
    MountError = 4,
    SessionAlreadyOpened = 5,
    SessionAlreadyClosed = 6,
    CannotIdentifyUser = 7,
    Unknown,
}

impl From<u32> for ServiceOperationResult {
    fn from(value: u32) -> Self {
        match value {
            0 => ServiceOperationResult::Ok,
            1 => ServiceOperationResult::PubKeyError,
            2 => ServiceOperationResult::DataDecryptionFailed,
            3 => ServiceOperationResult::CannotLoadUserMountError,
            4 => ServiceOperationResult::MountError,
            5 => ServiceOperationResult::SessionAlreadyOpened,
            6 => ServiceOperationResult::SessionAlreadyClosed,
            7 => ServiceOperationResult::CannotIdentifyUser,
            _ => ServiceOperationResult::Unknown,
        }
    }
}

#[proxy(
    interface = "org.zbus.login_ng",
    default_service = "org.zbus.login_ng",
    default_path = "/org/zbus/login_ng"
)]
trait Service {
    async fn get_pubkey(&self) -> ZResult<String>;

    async fn open_user_session(&self, user: &str, password: Vec<u8>) -> ZResult<u32>;

    async fn close_user_session(&self, user: &str) -> ZResult<u32>;
}

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
        plain_main_password: &String,
    ) -> ZResult<u32> {
        let connection = Connection::session().await?;

        let proxy = ServiceProxy::new(&connection).await?;

        let pk = proxy.get_pubkey().await?;

        // return an unknown error if the service was unable to serialize the RSA public key
        if pk.is_empty() {
            println!("login_ng: open_session: public key is empty");

            return Ok(u32::MAX);
        }

        match RsaPublicKey::from_pkcs1_pem(pk.as_str()) {
            Ok(pubkey) => {
                let mut rng = rand::thread_rng();

                let encrypted_password = pubkey
                    .encrypt(&mut rng, Pkcs1v15Encrypt, plain_main_password.as_bytes())
                    .unwrap();

                let reply = proxy
                    .open_user_session(user.as_str(), encrypted_password)
                    .await?;

                println!("login_ng: open_session: DBus service returned {reply}");

                Ok(reply)
            }
            Err(_) => {
                println!("login_ng: open_session: cannot import public RSA key");

                Ok(1u32)
            },
        }
    }

    pub(crate) async fn close_session_for_user(user: &String) -> ZResult<u32> {
        let connection = Connection::session().await?;

        let proxy = ServiceProxy::new(&connection).await?;
        let reply = proxy.close_user_session(user.as_str()).await?;

        Ok(reply)
    }

    pub(crate) async fn handle_env(pamh: &mut PamHandle) {}
}

impl PamHooks for PamQuickEmbedded {
    fn sm_close_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        INIT.call_once(|| {
            // Initialize the Tokio runtime
            unsafe {
                RUNTIME = Some(Runtime::new().unwrap());
            }
        });

        unsafe {
            match &RUNTIME {
                Some(runtime) => runtime.block_on(async {
                    match pamh.get_item::<pam::items::User>() {
                        Ok(Some(username)) => match PamQuickEmbedded::close_session_for_user(
                            &String::from(username.to_string_lossy()),
                        )
                        .await
                        {
                            Ok(result) => match ServiceOperationResult::from(result) {
                                ServiceOperationResult::Ok => PamResultCode::PAM_SUCCESS,
                                _ => PamResultCode::PAM_SERVICE_ERR,
                            },
                            Err(_) => PamResultCode::PAM_SERVICE_ERR,
                        },
                        Ok(None) => PamResultCode::PAM_SERVICE_ERR,
                        Err(_) => PamResultCode::PAM_SERVICE_ERR,
                    }
                }),
                None => return PamResultCode::PAM_SERVICE_ERR,
            }
        }
    }

    fn sm_open_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        println!("login_ng: open_session: enter");

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
                    Ok(Some(username)) => String::from(username.to_string_lossy()),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        println!("login_ng: open_session: user {username}");

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg = match PamQuickEmbedded::load_user_auth_data_from_username(&username) {
            Ok(user_cfg) => user_cfg,
            Err(pam_err_code) => return pam_err_code,
        };

        println!("login_ng: open_session: loaded data");
        // TODO: set environment variables

        unsafe {
            match &RUNTIME {
                Some(runtime) => runtime.block_on(async {
                    match pamh.get_item::<pam::items::User>() {
                        Ok(Some(username)) => {
                            println!("login_ng: open_session: pam_login_ng-service->{}", username.to_string_lossy());

                            match PamQuickEmbedded::open_session_for_user(
                                &String::from(username.to_string_lossy()),
                                &String::from(""), // TODO: fetch the real passowrd
                            )
                            .await
                            {
                                Ok(result) => {
                                    println!("login_ng: open_session: pam_login_ng-service returned {result}");

                                    match ServiceOperationResult::from(result) {
                                        ServiceOperationResult::Ok => PamResultCode::PAM_SUCCESS,
                                        _ => PamResultCode::PAM_SERVICE_ERR,
                                    }
                                },
                                Err(err) => {
                                    eprintln!("login_ng: open_session: pam_login_ng-service errored: {err}");
                                    PamResultCode::PAM_SERVICE_ERR
                                },
                            }
                        }
                        Ok(None) => PamResultCode::PAM_SERVICE_ERR,
                        Err(_) => PamResultCode::PAM_SERVICE_ERR,
                    }
                }),
                None => return PamResultCode::PAM_SERVICE_ERR,
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
                    Ok(Some(username)) => String::from(username.to_string_lossy()),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg = match PamQuickEmbedded::load_user_auth_data_from_username(&username) {
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
                    Ok(Some(username)) => String::from(username.to_string_lossy()),
                    Ok(None) => return PamResultCode::PAM_AUTH_ERR,
                    Err(err) => return err,
                }
            }
        };

        // try to load the user and return PAM_USER_UNKNOWN if it cannot be loaded
        let user_cfg = match PamQuickEmbedded::load_user_auth_data_from_username(&username) {
            Ok(user_cfg) => user_cfg,
            Err(pam_err_code) => return pam_err_code,
        };

        // first of all check if the empty password is valid
        if let Ok(main_password) = user_cfg.main_by_auth(&Some(String::new())) {
            match user_cfg.check_main(&main_password) {
                Ok(password_matches) => match password_matches {
                    true => return PamResultCode::PAM_SUCCESS,
                    false => {}
                },
                _ => {}
            }
        }

        // if the empty password was not valid then continue and ask for a password
        let conv = match pamh.get_item::<Conv>() {
            Ok(Some(conv)) => conv,
            Ok(None) => {
                unreachable!("No conv available");
            }
            Err(err) => {
                println!("Couldn't get pam_conv");
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
