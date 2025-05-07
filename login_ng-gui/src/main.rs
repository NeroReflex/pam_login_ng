// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, error::Error, ffi::OsString, sync::{Arc, Mutex}};

use login_ng::users::os::unix::UserExt;
use login_ng_user_interactions::{cli::CommandLineLoginUserInteractionHandler, login::{LoginExecutor, SessionCommandRetrival}};
use slint::ModelRc;

slint::include_modules!();

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
        let _ui = ui_handle.unwrap();

        let maybe_username = Some(username.as_str().to_string());

        let prompter = Arc::new(Mutex::new(CommandLineLoginUserInteractionHandler::new(
            true,
            Some(username.as_str().to_string()),
            None,
        )));

        use login_ng_user_interactions::greetd::GreetdLoginExecutor;

        let mut login_executor = GreetdLoginExecutor::new(env::var("GREETD_SOCK").unwrap(), prompter);

        login_executor.execute(&maybe_username, &SessionCommandRetrival::AutodetectFromUserHome).unwrap();

        //let ui_handle = ui.as_weak();
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
