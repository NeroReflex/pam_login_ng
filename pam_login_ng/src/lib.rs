extern crate pam;

use login_ng::storage::{load_user_auth_data, StorageSource};
use pam::constants::{PamFlag, PamResultCode, *};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
//use pam::pam_try;
use login_ng::users;
use std::ffi::CStr;

struct PamQuickEmbedded;
pam::pam_hooks!(PamQuickEmbedded);

impl PamHooks for PamQuickEmbedded {
    fn sm_close_session(pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        match pamh.get_item::<pam::items::User>() {
            Ok(Some(username)) => println!("{}", String::from(username.to_string_lossy())),
            Ok(None) => println!("B"),
            Err(err) => println!("E {:?}", err),
        };

        PamResultCode::PAM_IGNORE
    }

    fn sm_open_session(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        PamResultCode::PAM_IGNORE
    }

    fn sm_setcred(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        println!("set credentials");
        PamResultCode::PAM_SUCCESS
    }

    fn acct_mgmt(_pamh: &mut PamHandle, _args: Vec<&CStr>, _flags: PamFlag) -> PamResultCode {
        println!("account management");
        PamResultCode::PAM_SUCCESS
    }

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

        match username.as_str() {
            "" => return PamResultCode::PAM_USER_UNKNOWN,
            "root" => return PamResultCode::PAM_USER_UNKNOWN,
            _ => {}
        }

        let storage_source = match users::get_user_by_name(&username) {
            Some(user) => StorageSource::Username(user.name().to_string_lossy().to_string()),
            None => return PamResultCode::PAM_USER_UNKNOWN,
        };

        // load login-ng data and skip the user if it's not set
        let user_cfg = match load_user_auth_data(&storage_source) {
            Ok(load_res) => match load_res {
                Some(auth_data) => match auth_data.has_main() {
                    true => auth_data,
                    false => return PamResultCode::PAM_USER_UNKNOWN,
                },
                None => return PamResultCode::PAM_USER_UNKNOWN,
            },
            Err(_err) => return PamResultCode::PAM_USER_UNKNOWN,
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

        match pam::pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "Password: "))
            .map(|cstr| cstr.to_str().map(|s| s.to_string()))
        {
            Some(Ok(password)) => user_cfg
                .check_main(&password)
                .map(|password_matches| match password_matches {
                    true => PamResultCode::PAM_SUCCESS,
                    false => PamResultCode::PAM_AUTH_ERR,
                })
                .unwrap_or(PamResultCode::PAM_AUTH_ERR),
            Some(Err(_err)) => PamResultCode::PAM_CRED_INSUFFICIENT,
            None => PamResultCode::PAM_CRED_INSUFFICIENT,
        }
    }
}
