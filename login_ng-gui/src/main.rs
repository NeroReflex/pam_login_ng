// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, error::Error, ffi::OsString, sync::{Arc, Mutex}};

use login_ng::{storage::{load_user_auth_data, StorageSource}, user::UserAuthData, users::os::unix::UserExt};
use login_ng_user_interactions::login::{LoginExecutor, LoginUserInteractionHandler, SessionCommandRetrival};
use slint::ModelRc;

slint::include_modules!();

#[derive(Default)]
pub struct GUILoginUserInteractionHandler {
    attempt_autologin: bool,

    maybe_user: Option<UserAuthData>,

    maybe_username: Option<String>,
}

impl GUILoginUserInteractionHandler {
    pub fn new(
        attempt_autologin: bool,
        maybe_username: Option<String>,
    ) -> Self {
        let maybe_user = match &maybe_username {
            Some(username) => {
                load_user_auth_data(&StorageSource::Username(username.clone())).map_or(None, |a| a)
            }
            None => None,
        };

        Self {
            attempt_autologin,
            maybe_user,
            maybe_username,
        }
    }
}

impl LoginUserInteractionHandler for GUILoginUserInteractionHandler {
    fn provide_username(&mut self, username: &String) {
        self.maybe_user =
            load_user_auth_data(&StorageSource::Username(username.clone())).map_or(None, |a| a)
    }

    fn prompt_secret(&mut self, msg: &String) -> Option<String> {
        if self.attempt_autologin {
            if let Some(user_cfg) = &self.maybe_user {
                if let Ok(main_password) = user_cfg.main_by_auth(&Some(String::new())) {
                    return Some(main_password);
                }
            }
        }

        todo!()
    }

    fn prompt_plain(&mut self, msg: &String) -> Option<String> {
        match &self.maybe_username {
            Some(username) => Some(username.clone()),
            None => todo!(),
        }
    }

    fn print_info(&mut self, msg: &String) {
        println!("{}", msg)
    }

    fn print_error(&mut self, msg: &String) {
        eprintln!("{}", msg)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    // Fetch the list of users (this is just a placeholder; replace with your actual user fetching logic)
    let users = slint::VecModel::<slint::SharedString>::default();

    for user in unsafe { login_ng::users::all_users() } {
        if user.name() == OsString::from("nobody") {
            continue;
        }

        if user.shell() == OsString::from("/bin/false") {
            continue;
        }

        let uid = user.uid();
        if uid == 0 || uid < 1000 || uid == login_ng::users::uid_t::MAX {
            continue;
        }

        users.push(slint::SharedString::from(user.name().to_string_lossy().to_string()));
    }

    let users = ModelRc::new(users);

    // Set the user list in the UI
    ui.set_userList(users);

    let ui_handle = ui.as_weak();
    ui.on_request_login(move |username| {
        let ui = ui_handle.unwrap();

        let maybe_username = Some(username.as_str().to_string());

        let prompter = Arc::new(Mutex::new(GUILoginUserInteractionHandler::new(
            true,
            Some(username.as_str().to_string()),
        )));

        use login_ng_user_interactions::greetd::GreetdLoginExecutor;

        let mut login_executor = GreetdLoginExecutor::new(env::var("GREETD_SOCK").unwrap(), prompter);

        login_executor.execute(&maybe_username, &SessionCommandRetrival::AutodetectFromUserHome).unwrap();

        /*move || {
            let ui = ui_handle.unwrap();
            let selected_user = ui.get_selectedUser();
            // Here you can handle the login logic for the selected user
            println!("Logging in as: {}", selected_user);
            // Add your login logic here
        }*/
    });

    ui.run()?;

    Ok(())
}
