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

use std::env;
use std::sync::Arc;
use std::sync::Mutex;

use login_ng::cli::CommandLineLoginUserInteractionHandler;
use login_ng::conversation::ProxyLoginUserInteractionHandlerConversation;
use login_ng::login::*;

use argh::FromArgs;
use login_ng::pam::PamLoginExecutor;

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

#[cfg(feature = "greetd")]
fn login_greetd(
    greetd_sock: String,
    prompter: Arc<Mutex<dyn LoginUserInteractionHandler>>,
    maybe_username: &Option<String>,
    cmd: &Option<String>,
) -> Result<LoginResult, LoginError> {
    use login_ng::greetd::GreetdLoginExecutor;

    let mut login_executor = GreetdLoginExecutor::new(greetd_sock, prompter);

    login_executor.execute(maybe_username, cmd)
}

fn login_pam(
    allow_autologin: bool,
    prompter: Arc<Mutex<dyn LoginUserInteractionHandler>>,
    maybe_username: &Option<String>,
    cmd: &Option<String>,
) -> Result<LoginResult, LoginError> {
    let conversation = ProxyLoginUserInteractionHandlerConversation::new(prompter);

    let mut login_executer = PamLoginExecutor::new(conversation, allow_autologin);

    login_executer.execute(maybe_username, cmd)
}

fn main() {
    println!("login-ng version 0.1.0, Copyright (C) 2024 Denis Benato");
    println!("login-ng comes with ABSOLUTELY NO WARRANTY;");
    println!("This is free software, and you are welcome to redistribute it");
    println!("under certain conditions.");
    println!("");

    let args: Args = argh::from_env();

    let allow_autologin = args.autologin.unwrap_or(false);

    let max_failures = args.failures.unwrap_or(5);

    let prompter = Arc::new(Mutex::new(CommandLineLoginUserInteractionHandler::new(
        allow_autologin,
        args.user.clone(),
        args.password.clone(),
    )));

    'login_attempt: for attempt in 0..max_failures {
        let login_result = {
            #[cfg(not(feature = "greetd"))]
            {
                if let Ok(_) = env::var("GREETD_SOCK") {
                    println!(
                        "Running over greetd, but greetd support has been compile-time disabled."
                    )
                }

                login_pam(allow_autologin, prompter.clone(), &args.user, &args.cmd)
            }

            #[cfg(feature = "greetd")]
            {
                match env::var("GREETD_SOCK") {
                    Ok(greetd_sock) => {
                        login_greetd(greetd_sock, prompter.clone(), &args.user, &args.cmd)
                    }
                    Err(_) => login_pam(allow_autologin, prompter.clone(), &args.user, &args.cmd),
                }
            }
        };

        match login_result {
            Ok(succeeded) => match succeeded {
                LoginResult::Success => break 'login_attempt,
                LoginResult::Failure => eprintln!("Login attempt {}/{max_failures} failed.", attempt+1),
            },
            Err(err) => eprintln!("Login attempt {}/{max_failures} errored: {}", attempt+1, err),
        };
    }
}
