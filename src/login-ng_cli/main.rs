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
    cmd: &Option<String>
) -> Result<LoginResult, LoginError> {
    use login_ng::greetd::GreetdLoginExecutor;

    let mut login_executor = GreetdLoginExecutor::new(greetd_sock, prompter);

    login_executor.execute(maybe_username, cmd)
}

fn login_pam(
    prompter: Arc<Mutex<dyn LoginUserInteractionHandler>>,
    maybe_username: &Option<String>,
    cmd: &Option<String>
) -> Result<LoginResult, LoginError> {
    let conversation = ProxyLoginUserInteractionHandlerConversation::new(prompter);

    let mut login_executer = PamLoginExecutor::new(conversation);

    login_executer.execute(maybe_username, cmd)
}

fn main() {
    let args: Args = argh::from_env();

    let allow_autologin = args.autologin.unwrap_or(false);

    let max_failures = args.failures.unwrap_or(5);

    let prompter = Arc::new(
        Mutex::new(
            CommandLineLoginUserInteractionHandler::new(
                allow_autologin,
                args.user.clone()
            )
        )
    );

    'login_attempt: for attempt in 0..max_failures {

        let login_result = {
            #[cfg(not(feature = "greetd"))]
            {
                if let Ok(_) = env::var("GREETD_SOCK") {
                    println!("Running over greetd, but greetd support has been compile-time disabled.")
                }

                login_pam(prompter.clone(), &args.user, &args.cmd)
            }

            #[cfg(feature = "greetd")]
            {
                match env::var("GREETD_SOCK") {
                    Ok(greetd_sock) => login_greetd(greetd_sock, prompter.clone(), &args.user, &args.cmd),
                    Err(_) => login_pam(prompter.clone(), &args.user, &args.cmd)
                }
            }
        };

        match login_result {
            Ok(succeeded) => match succeeded {
                LoginResult::Success => break 'login_attempt,
                LoginResult::Failure => eprintln!("Login attempt {attempt}/{max_failures} failed.")
            },
            Err(err) => eprintln!("Login attempt {attempt}/{max_failures} errored: {}", err)
        };
    }
}
