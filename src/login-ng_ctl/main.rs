use std::sync::Arc;
use std::sync::Mutex;

use login_ng::cli::TrivialCommandLineConversationPrompter;
use login_ng::conversation::*;
use login_ng::storage::{
    load_user_auth_data,
    remove_user_auth_data,
    save_user_auth_data
};
use login_ng::storage::StorageSource;
use login_ng::cli::*;
use login_ng::prompt_password;

use login_ng::user::UserAuthData;
use pam_client2::{Context, Flag};

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Command line tool for managing login-ng authentication methods
struct Args {
    #[argh(option, short = 'u')]
    /// username
    user: Option<String>,

    #[argh(option, short = 'p')]
    /// main password for authentication (the one accepted by PAM)
    password: Option<String>,

    #[argh(option)]
    /// force update of the user configuration if required
    update_as_needed: Option<bool>,

    #[argh(option)]
    /// ignore the failure about the user running this software and the target user not being the same
    ignore_user: Option<bool>,

    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
/// Subcommands for managing authentication methods
enum Command {
    Reset(ResetCommand),
    Inspect(InspectCommand),
    Add(AddAuthCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Reset additional authentication data also destroying the intermediate key
#[argh(subcommand, name = "reset")]
struct ResetCommand {
    
}

#[derive(FromArgs, PartialEq, Debug)]
/// Inspects user login settings
#[argh(subcommand, name = "inspect")]
struct InspectCommand {
    
}

#[derive(FromArgs, PartialEq, Debug)]
/// Add a new authentication method
#[argh(subcommand, name = "add")]
struct AddAuthCommand {
    #[argh(option)]
    /// intermediate key (the key used to unlock the main password)
    intermediate: Option<String>,

    #[argh(subcommand)]
    method: AddAuthMethod,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
/// Subcommands for adding an authentication method
enum AddAuthMethod {
    Password(AddAuthPasswordCommand)
}

#[derive(FromArgs, PartialEq, Debug)]
/// Command to add a new authentication method
#[argh(subcommand, name = "password")]
struct AddAuthPasswordCommand {
    #[argh(option)]
    /// secondary password for authentication
    secondary_pw: Option<String>,
}

fn main() {
    let args: Args = argh::from_env();

    let user_prompt = Some("username: ");

    let answerer = Arc::new(
        Mutex::new(
            TrivialCommandLineConversationPrompter::new(
                args.user.clone(),
                args.password.clone(),
            )
        )
    );

    let interaction_recorder = Arc::new(
        Mutex::new(
            SimpleConversationRecorder::new()
        )
    );

    let mut context = Context::new(
        "system-login",
        args.user.as_deref(),
        CommandLineConversation::new(Some(answerer), Some(interaction_recorder.clone()))
    ).expect("Failed to initialize PAM context");

    context.set_user_prompt(user_prompt).unwrap();

    // Authenticate the user (ask for password, 2nd-factor token, fingerprint, etc.)
    context.authenticate(Flag::NONE).expect("Authentication failed");

    // Validate the account (is not locked, expired, etc.)
    context.acct_mgmt(Flag::NONE).expect("Account validation failed");

    let username = args.user.clone().unwrap_or_else(|| {
        interaction_recorder.lock().unwrap().recorded_username(&user_prompt).unwrap()
    });

    let storage_source = StorageSource::Username(username.clone());
    let mut user_cfg = match load_user_auth_data(&storage_source) {
        Ok(load_res) => match load_res {
            Some(auth_data) => auth_data,
            None => UserAuthData::new()
        },
        Err(err) => {
            eprintln!("There is a problem loading your configuration file: {}.\nAborting.", err);
            std::process::exit(-1)
        }
    };

    let mut write_file = args.update_as_needed;
    match args.command {
        Command::Reset(_) => {
            match remove_user_auth_data(&storage_source) {
                Ok(_) => {},
                Err(err) => {
                    eprintln!("Error in resetting user additional athentication methods: {}", err);
                    std::process::exit(-1)
                }
            }
            
            // Do NOT rewrite the User structure that was created while authenticating the user
            write_file = Some(false)
        },
        Command::Inspect(_) => {
            match load_user_auth_data(&storage_source) {
                Ok(user) => {},
                Err(err) => {
                    eprintln!("Error in fetching user additional athentication methods: {}", err);
                    std::process::exit(-1)
                }
            }
        },
        Command::Add(add_cmd) => {
            let intermediate_password = add_cmd.intermediate.clone().unwrap_or_else(|| {
                prompt_password("Intermediate key:").expect("Failed to read intermediate key")
            });

            if user_cfg.has_main() {
                if let Err(err) = user_cfg.main_by_auth(&Some(intermediate_password.clone())) {
                    eprintln!("Could not verify the correctness of the intermediate key: {}", err);
                    std::process::exit(-1)
                }
            }

            // if the main password is accepted update the stored one
            if let Some(main_password) = interaction_recorder.lock().unwrap().recorded_password() {
                user_cfg.set_main(&main_password, &intermediate_password).expect("Error handling main password");
            }

            match add_cmd.method {
                AddAuthMethod::Password(add_auth_password_command) =>  {
                    let secondary_password = match add_auth_password_command.secondary_pw {
                        Some(secondary_password) => secondary_password,
                        None => {
                            let secondary_password = prompt_password("Secondary password:").expect("Failed to read secondary password");
    
                            let repeat = prompt_password("Secondary password (repeat):").expect("Failed to read secondary password (repeat)");
                            if secondary_password != repeat {
                                eprintln!("Passwords do not match.\nAborting.");
                                std::process::exit(-1)
                            }
    
                            secondary_password
                        }
                    };

                    if !user_cfg.has_main() {
                        eprintln!("Cannot add a secondary password for an account with no main password.\nAborting.");
                        std::process::exit(-1);
                    }
    
                    match user_cfg.add_secondary_password(&intermediate_password, &secondary_password) {
                        Ok(_) => {
                            write_file = Some(true);
                            println!("Secondary password added.");
                        },
                        Err(err) => {
                            eprintln!("Error adding a secondary password: {}.\nAborting.", err);
                            std::process::exit(-1);
                        }
                    }
                },
            }
        }
    }

    let selected_user = users::get_user_by_name(&username).expect("Could not identify the specified user by its username.\nAborting.");

    let uid = selected_user.uid();

    if write_file.unwrap_or_default() {
        let current_uid = users::get_current_uid();

        if uid != current_uid {
            eprintln!("Configuration is not relevant to the user invoking the command.\nAborting.");
            std::process::exit(-1);
        }

        save_user_auth_data(user_cfg, &storage_source).expect("Error saving the updated configuration.\nAborting.");
    }
}
