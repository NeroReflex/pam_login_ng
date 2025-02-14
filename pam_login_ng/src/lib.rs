extern crate pam;

use pam::constants::{PamFlag, PamResultCode, *};
use pam::conv::Conv;
use pam::module::{PamHandle, PamHooks};
//use pam::pam_try;
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
        let user = match pamh.get_user(None) {
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

        if user == "root" {
            return PamResultCode::PAM_USER_UNKNOWN;
        }

        if user != "stupido" {
            return PamResultCode::PAM_USER_UNKNOWN;
        }

        let _conv = match pamh.get_item::<Conv>() {
            Ok(Some(conv)) => conv,
            Ok(None) => {
                unreachable!("No conv available");
            }
            Err(err) => {
                println!("Couldn't get pam_conv");
                return err;
            }
        };

        //let _ = conv.send(PAM_TEXT_INFO, format!("logging in as {}", user).as_str());

        //let password = pam_try!(conv.send(PAM_PROMPT_ECHO_OFF, "Password: "));

        PamResultCode::PAM_SUCCESS
    }
}
