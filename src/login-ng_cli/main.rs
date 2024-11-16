use std::{
    env,
    io::{self, BufRead},
};

use getopts::Options;

use login_ng::login::*;

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
    opts.optopt("c", "cmd", "command to run", "COMMAND");
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

    let cmd = match matches.opt_default("cmd", "login_cmd") {
        Some(cmd) => cmd,
        None => String::from("sh")
    };

    let max_failures: usize = match matches.opt_get("max-failures") {
        Ok(v) => v.unwrap_or(5),
        Err(e) => {
            eprintln!("unable to parse max failures: {}", e);
            std::process::exit(1)
        }
    };

    let interactive_prompt = |str: &String| -> Result<String, Box<dyn std::error::Error>> {
        prompt_stderr(str.as_str())
    };

    //let uts = uname().unwrap();
    'login_attempt: for _ in 0..max_failures {

        let username = match prompt_stderr(&format!("login: ")) {
            Ok(typed_username) => {
                typed_username
            },
            Err(err) => {
                println!("Login failed: {}\n", err);
                continue 'login_attempt
            }
        };

        let login_data = Login::new(username, cmd.clone(), interactive_prompt);

        match login_data.execute() {
            Ok(LoginResult::Success) => break,
            Ok(LoginResult::Failure) => eprintln!("Login incorrect\n"),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
