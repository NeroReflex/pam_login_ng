use std::env;

use login_ng::user::*;

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Command line tool for managing login-ng authentication methods
struct Args {
    #[argh(option, short = 'u')]
    /// username to authenticate
    user: Option<String>,

    #[argh(option, short = 'p')]
    /// password for authentication: main, intermediate key or secondary are accepted
    password: Option<String>,

    #[argh(option, short = 'a')]
    /// attempt to autologin attempting to use the empty password (not trying the empty password vastly reduces authentication times)
    autologin: Option<bool>,

    #[argh(option, short = 'c')]
    /// command to run as the logged in user
    cmd: Option<String>,

    #[argh(option, short = 'f')]
    /// maximum number of accepted failures before the login gets aborted (defaults to 5)
    failures: Option<usize>,
}


use login_ng::prompt_password;



fn main() {
    let args: Args = argh::from_env();

    let allow_autologin = args.autologin.unwrap_or(false);
/*
    let cmd = match matches.opt_default("cmd", "login-ng_cmd") {
        Some(cmd) => cmd,
        None => String::from("login-ng_cmd")
    };
*/
    let max_failures = args.failures.unwrap_or(5);

    //let uts = uname().unwrap();
    'login_attempt: for attempt in 0..max_failures {

        #[cfg(not(feature = "greetd"))]
        {
            // Code that runs when the greeter feature is not enabled
            println!("Greeter feature is not enabled. Running in default mode.");
        }

        #[cfg(feature = "greetd")]
        {
            use login_ng::login::*;

            let username = match args.user {
                Some(account) => account,
                None => match login_ng::prompt_stderr(&format!("login: ")) {
                    Ok(typed_username) => {
                        typed_username
                    },
                    Err(err) => {
                        println!("Login failed: {}\n", err);
                        continue 'login_attempt
                    }
                }
            };

            let login_data = Login::new(
                username.clone(),
                cmd.clone(),
                move |str: &String, param: (String, usize)| -> Result<String, Box<dyn std::error::Error>> {
                    let (username, attempt) = param;
                    
                    let file_path = format!("/etc/login-ng/{}.conf", username.clone());

                    // try to autologin searching for a secondary password that is the empty string
                    if attempt == 0 && allow_autologin {
                        if let Ok(user_cfg) = User::load_from_file(file_path) {
                            let empty_password = Some(String::new());
                            if let Ok(main_password) = user_cfg.main_by_auth(&empty_password) {
                                return Ok(main_password)
                            }
                        }
                    }

                    Ok(prompt_password(format!("{}", str)).map_err(|err| Box::new(err))?)
                }
            );

            match login_data.execute((username.clone(), attempt)) {
                Ok(LoginResult::Success) => break,
                Ok(LoginResult::Failure) => eprintln!("Login incorrect\n"),
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
