use std::env;

use login_ng::user::*;

use rpassword::prompt_password;

use std::path::Path;

fn main() {
    let cmd = clap::Command::new("login-ng_ctl")
        .bin_name("login-ng_ctl")
        .arg(
            clap::arg!(--"user" <USER>)
                .value_parser(clap::value_parser!(String)),
        )
        //.styles(CLAP_STYLING)
        .subcommand_required(true)
        .subcommand(
            clap::command!("set_main")
            .arg(
                clap::arg!(--"main" <MAIN>)
                    .value_parser(clap::value_parser!(String))
            )
            .arg(
                clap::arg!(--"intermediate" <INTERMEDIATE>)
                    .value_parser(clap::value_parser!(String))
            )
            ,
        )
        .subcommand(
            clap::command!("add_secondary_password")
            .arg(
                clap::arg!(--"intermediate" <INTERMEDIATE>)
                    .value_parser(clap::value_parser!(String))
            )
            .arg(
                clap::arg!(--"secondary" <SECONDARY>)
                    .value_parser(clap::value_parser!(String))
            )
            ,
        );
    let matches = cmd.get_matches();

    match matches.get_one::<String>("user") {
        Some(username) => {
            let file_path = format!("/etc/login-ng/{}.json", &username);
            let mut user_cfg = match Path::new(&file_path).exists() {
                true => match User::load_from_file(&file_path) {
                        Ok(user_cfg) => user_cfg,
                        Err(err) => {
                            println!("There is a problem loading your configuration file: {}.\nAborting.", err);
                            std::process::exit(-1)
                        }
                    }
                false => {
                    User::new()
                }
            };

            let _matches = match matches.subcommand() {
                Some(("add_secondary_password", matches)) => {
                    if !user_cfg.has_main() {
                        println!("Cannot add a secondary password for an account with no main password.\nAborting.");
                        std::process::exit(-1)
                    }

                    let intermediate_password = match matches.get_one::<String>("intermediate") {
                        Some(intermediate_password) => intermediate_password.clone(),
                        None => {
                            let intermediate_password = prompt_password("Intermediate password:").expect("Failed to read intermediate password");

                            if !login_ng::is_valid_password(&intermediate_password) {
                                println!("Intermediate password is not valid.\nAborting.");
                                std::process::exit(-1)
                            }

                            intermediate_password
                        }
                    };

                    let secondary_password = match matches.get_one::<String>("secondary") {
                        Some(secondary_password) => secondary_password.clone(),
                        None => {
                            let secondary_password = prompt_password("Secondary password:").expect("Failed to read secondary password");
                            if !login_ng::is_valid_password(&secondary_password) {
                                println!("Secondary password is not valid.\nAborting.");
                                std::process::exit(-1)
                            }

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
                },
                Some(("set_main", matches)) => {
                    let main_password = match matches.get_one::<String>("main") {
                        Some(main_password) => main_password.clone(),
                        None => {
                            let main_password = prompt_password("Main password:").expect("Failed to read main password");
                            if !login_ng::is_valid_password(&main_password) {
                                println!("Main password is not valid.\nAborting.");
                                std::process::exit(-1)
                            }

                            let repeat = prompt_password("Main password (repeat):").expect("Failed to read main password (repeat)");
                            if main_password != repeat {
                                println!("Passwords do not match.\nAborting.");
                                std::process::exit(-1)
                            }

                            main_password
                        }
                    };

                    let intermediate_password = match matches.get_one::<String>("intermediate") {
                        Some(intermediate_password) => intermediate_password.clone(),
                        None => {
                            let intermediate_password = prompt_password("Intermediate password:").expect("Failed to read intermediate password");

                            if !login_ng::is_valid_password(&intermediate_password) {
                                println!("Intermediate password is not valid.\nAborting.");
                                std::process::exit(-1)
                            }

                            if !user_cfg.has_main() {
                                let repeat = prompt_password("Intermediate password (repeat):").expect("Failed to read intermediate password (repeat)");
                                if intermediate_password != repeat {
                                    println!("Passwords do not match.\nAborting.");
                                    std::process::exit(-1)
                                }
                            }

                            intermediate_password
                        }
                    };

                    match user_cfg.set_main(&main_password, &intermediate_password) {
                        Ok(_) => match user_cfg.store_to_file(Path::new(&file_path)) {
                            Ok(_) => println!("Main password changed."),
                            Err(err) => println!("Error saving the updated configuration: {}.\nAborting.", err)
                        }
                        Err(err) => println!("Error changing main password: {}.\nAborting.", err)
                    }
                },
                _ => unreachable!("clap should ensure we don't get here"),
            };
        },
        None => {
            println!("No username provided.\nAborting.");
            std::process::exit(-1)
        }
    }
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