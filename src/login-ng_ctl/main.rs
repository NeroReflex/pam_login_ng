use std::env;
use std::sync::Arc;
use std::sync::Mutex;

use login_ng::conversation::*;
use login_ng::user::*;

use login_ng::prompt_password;

use pam_client2::{Context, Flag};

use std::path::Path;

fn main() {
    let cmd = clap::Command::new("login-ng_ctl")
        .bin_name("login-ng_ctl")
        .arg(
            clap::arg!(--"user" <USER>)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            clap::arg!(--"password" <PASSWORD>)
                .help("Main password for authentication (the one accepted by PAM)")
                .value_parser(clap::value_parser!(String))
        )
        //.styles(CLAP_STYLING)
        .subcommand_required(true)
        .subcommand(
            clap::command!("add")
            .subcommand_required(true)
            .subcommand(clap::command!("secondary")
                .about("Add a secondary password as an authentication method")
                .arg(
                    clap::arg!(--"secondary" <SECONDARY>)
                        .help("Secondary password for authentication")
                        .value_parser(clap::value_parser!(String))
                )
            )
            .about("Add a new authentication method to unlock the intermediate key that will in turn unlock the main password")
            .arg(
                clap::arg!(--"intermediate" <INTERMEDIATE>)
                .help("Intermediate key (the key used to unlock the main password)")
                    .value_parser(clap::value_parser!(String)
            )
        )
    );
    let matches = cmd.get_matches();

    let maybe_username = matches.get_one::<String>("user");
    let user_prompt = Option::Some("username: ");

    let answerer = Arc::new(
        Mutex::new(
            SimpleConversationPromptAnswerer::new(
                match maybe_username {
                    Some(username) => Some(username.clone()),
                    None => None
                },
                match matches.get_one::<String>("password") {
                    Some(username) => Some(username.clone()),
                    None => None
                }
            )
        )
    );

    let interaction_recorder = Arc::new(
        Mutex::new(
            SimpleConversationRecorder::new()
        )
    );

    let conversation = Conversation::new(Some(answerer), Some(interaction_recorder.clone()));

    let mut context = Context::new(
        "system-login",
        maybe_username.map(|user| user.as_str()),
        conversation
    ).expect("Failed to initialize PAM context");

    context.set_user_prompt(user_prompt.map(|prompt| prompt)).unwrap();

    // Authenticate the user (ask for password, 2nd-factor token, fingerprint, etc.)
    context.authenticate(Flag::NONE).expect("Authentication failed");

    // Validate the account (is not locked, expired, etc.)
    context.acct_mgmt(Flag::NONE).expect("Account validation failed");

    let username = match maybe_username {
        Some(username) => username.clone(),
        None => interaction_recorder.clone().lock().unwrap().recorded_username(&user_prompt).unwrap()
    };

    let file_path = format!("/etc/login-ng/{}.json", &username);
    let mut user_cfg = match Path::new(&file_path).exists() {
        true => match User::load_from_file(&file_path) {
                Ok(user_cfg) => user_cfg,
                Err(err) => {
                    eprintln!("There is a problem loading your configuration file: {}.\nAborting.", err);
                    std::process::exit(-1)
                }
            }
        false => {
            User::new()
        }
    };

    let _matches = match matches.subcommand() {
        Some(("add", matches)) => {
            let intermediate_password = match matches.get_one::<String>("intermediate") {
                Some(intermediate_password) => intermediate_password.clone(),
                None => prompt_password("Intermediate key:").expect("Failed to read intermediate key")
            };
        
            // if the main password is accepted update the stored one
            match interaction_recorder.clone().lock().unwrap().recorded_password() {
                Some(main_password) => match user_cfg.set_main(&main_password, &intermediate_password) {
                    Ok(_) => {}
                    Err(err) => println!("Error handling main password: {}.\nAborting.", err)
                },
                None => {}
            }

            match matches.subcommand() {
                Some(("secondary", matches)) => {
                    if !user_cfg.has_main() {
                        println!("Cannot add a secondary password for an account with no main password.\nAborting.");
                        std::process::exit(-1)
                    }

                    let secondary_password = match matches.get_one::<String>("secondary") {
                        Some(secondary_password) => secondary_password.clone(),
                        None => {
                            let secondary_password = prompt_password("Secondary password:").expect("Failed to read secondary password");

                            let repeat = prompt_password("Secondary password (repeat):").expect("Failed to read secondary password (repeat)");
                            if secondary_password != repeat {
                                println!("Passwords do not match.\nAborting.");
                                std::process::exit(-1)
                            }

                            secondary_password
                        }
                    };

                    match user_cfg.add_secondary_password(&intermediate_password, &secondary_password) {
                        Ok(_) => match user_cfg.store_to_file(Path::new(&file_path)) {
                            Ok(_) => println!("Secondary password added."),
                            Err(err) => println!("Error saving the updated configuration: {}.\nAborting.", err)
                        }
                        Err(err) => println!("Error adding a secondary password: {}.\nAborting.", err)
                    }

                    match user_cfg.store_to_file(Path::new(&file_path)) {
                        Ok(_) => println!("Authentication method added."),
                        Err(err) => eprintln!("Error saving the updated configuration: {}.\nAborting.", err)
                    }
                },
                _ => eprintln!("No additional authentication method provided"),
            }
        }
        _ => unreachable!("clap should ensure we don't get here"),
    };
}
/*
// See also `clap_cargo::style::CLAP_STYLING`
pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
*/