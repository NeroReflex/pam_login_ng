use std::{
    env,
    io::{self, BufRead},
};

use getopts::Options;

use login_ng::login::*;
use login_ng::user::*;

use rpassword::prompt_password;

fn prompt_stderr(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdin_iter = stdin.lock().lines();
    eprint!("{}", prompt);
    Ok(stdin_iter.next().ok_or("no input")??)
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("u", "username", "username to force", "USERNAME");
    opts.optopt("c", "cmd", format!("command to run, defaults to login-ng_cmd").as_str(), "COMMAND");
    opts.optflag("a", "autologin", "allow autologin");
    opts.optopt(
        "f",
        "max-failures",
        "maximum number of accepted failures",
        "FAILURES",
    );
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f);
            print_usage(&program, opts);
            std::process::exit(1);
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        std::process::exit(0);
    }

    let allow_autologin = matches.opt_present("a");

    let cmd = match matches.opt_default("cmd", "login-ng_cmd") {
        Some(cmd) => cmd,
        None => String::from("login-ng_cmd")
    };

    let max_failures: usize = match matches.opt_get("max-failures") {
        Ok(v) => v.unwrap_or(5),
        Err(e) => {
            eprintln!("unable to parse max failures: {}", e);
            std::process::exit(1)
        }
    };

    //let uts = uname().unwrap();
    'login_attempt: for attempt in 0..max_failures {

        let username = match matches.opt_str("username") {
            Some(account) => account,
            None => match prompt_stderr(&format!("login: ")) {
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
