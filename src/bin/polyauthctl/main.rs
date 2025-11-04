/*
    polyauth A greeter written in rust that also supports autologin with systemd-homed
    Copyright (C) 2024-2025  Denis Benato

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

use std::fmt::Debug;
use std::path::PathBuf;

use chrono::Local;
use chrono::TimeZone;
use pam_polyauth::command::SessionCommand;
use pam_polyauth::mount::MountParams;
use pam_polyauth::storage::load_user_mountpoints;
use pam_polyauth::storage::load_user_session_command;
use pam_polyauth::storage::store_user_mountpoints;
use pam_polyauth::storage::store_user_session_command;
use pam_polyauth::storage::StorageSource;
use pam_polyauth::storage::{load_user_auth_data, remove_user_data, store_user_auth_data};
use pam_polyauth::user::UserAuthData;

use rpassword::prompt_password;

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Command line tool for managing polyauth authentication methods
struct Args {
    #[argh(option, short = 'u')]
    /// username to be used, if unspecified it will be autodetected: if that fails it will be prompted for
    username: Option<String>,

    #[argh(option, short = 'c')]
    /// force the use of a specific configuration file
    config_file: Option<PathBuf>,

    #[argh(option, short = 'p')]
    /// main password for authentication (the one accepted by PAM)
    password: Option<String>,

    #[argh(switch)]
    /// force update of the user configuration if required
    update_as_needed: Option<bool>,

    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
/// Subcommands for managing authentication methods
enum Command {
    Info(InfoCommand),
    Setup(SetupCommand),
    Reset(ResetCommand),
    Inspect(InspectCommand),
    Add(AddAuthCommand),
    SetSession(SetSessionCommand),
    ChangeMainMount(ChangeMainMountCommand),
    ChangeSecondaryMount(ChangeSecondaryMountCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Print information about the software
#[argh(subcommand, name = "info")]
struct InfoCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Set the mount command that has to be used to mount the user home directory
#[argh(subcommand, name = "set-pre-mount")]
struct ChangeSecondaryMountCommand {
    #[argh(option)]
    /// directory to mount the device into
    dir: String,

    #[argh(option)]
    /// device to mount
    device: String,

    #[argh(option)]
    /// filesystem type (corresponds to -t flag in mount)
    fstype: String,

    #[argh(option)]
    /// mount options relative to the filesystem type (corresponds to -o flag in mount)
    flags: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Set the mount command that has to be used to mount the user home directory
#[argh(subcommand, name = "set-home-mount")]
struct ChangeMainMountCommand {
    #[argh(option)]
    /// device to mount
    device: String,

    #[argh(option)]
    /// filesystem type (corresponds to -t flag in mount)
    fstype: String,

    #[argh(option)]
    /// mount options relative to the filesystem type (corresponds to -o flag in mount)
    flags: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Setup initial authentication data also creating a new intermediate key
#[argh(subcommand, name = "setup")]
struct SetupCommand {
    #[argh(option, short = 'i')]
    /// the intermediate key
    intermediate: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Set the default session command to be executed when a user login if nothing else is being specified
#[argh(subcommand, name = "set-session")]
struct SetSessionCommand {
    #[argh(option)]
    /// command to execute
    cmd: String,

    #[argh(option)]
    /// additional arguments for the command
    args: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Reset additional authentication data also destroying the intermediate key
#[argh(subcommand, name = "reset")]
struct ResetCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Inspects user login settings
#[argh(subcommand, name = "inspect")]
struct InspectCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Add a new authentication method
#[argh(subcommand, name = "add")]
struct AddAuthCommand {
    #[argh(option)]
    /// name of the authentication method
    name: String,

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
    Password(AddAuthPasswordCommand),
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

    let (storage_source, maybe_main_password) = match args.config_file {
        Some(path) => (StorageSource::File(path), args.password),
        None => (StorageSource::Username(users::get_current_username().unwrap().to_string_lossy().to_string()), args.password),
    };

    let mut user_cfg = match load_user_auth_data(&storage_source) {
        Ok(load_res) => match load_res {
            Some(auth_data) => auth_data,
            None => UserAuthData::new(),
        },
        Err(err) => {
            eprintln!("❌ There is a problem loading your configuration file: {err}.\nAborting.");
            std::process::exit(-1)
        }
    };

    let mut user_mounts = match load_user_mountpoints(&storage_source) {
        Ok(existing_data) => existing_data,
        Err(err) => {
            eprintln!("❌ Error in loading user mounts data: {err}");
            std::process::exit(-1)
        }
    };

    let mut write_file = args.update_as_needed;
    match args.command {
        Command::Info(_) => {
            let version = pam_polyauth::LIBRARY_VERSION;
            println!("polyauth version {version}, Copyright (C) 2024-2025 Denis Benato");
            println!("polyauth comes with ABSOLUTELY NO WARRANTY;");
            println!("This is free software, and you are welcome to redistribute it");
            println!("under certain conditions.");
            println!("\n");
        }
        Command::ChangeSecondaryMount(mount_data) => {
            let Some(new_data) = user_mounts else {
                eprintln!("❌ Error in changing user mounts: a main mount has not been defined");
                std::process::exit(-1)
            };

            user_mounts = Some(new_data.with_premount(
                &mount_data.dir,
                &MountParams::new(mount_data.device, mount_data.fstype, mount_data.flags),
            ));

            write_file = Some(true)
        }
        Command::ChangeMainMount(mount_data) => {
            user_mounts = Some(
                user_mounts
                    .unwrap_or_default()
                    .with_mount(&MountParams::new(
                        mount_data.device,
                        mount_data.fstype,
                        mount_data.flags,
                    )),
            );

            write_file = Some(true)
        }
        Command::SetSession(session_data) => {
            let command = SessionCommand::new(session_data.cmd);

            match store_user_session_command(&command, &storage_source) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("❌ Error in changing the user default session: {err}");
                    std::process::exit(-1)
                }
            }
        }
        Command::Setup(s) => {
            if user_cfg.has_main() {
                eprintln!("⚠️  User already has an intermediate key present: use reset if you want to delete the old one");
                std::process::exit(-1)
            }

            let intermediate_key = match s.intermediate {
                Some(ik) => ik.clone(),
                None => {
                    let ik = prompt_password("intermediate key:").unwrap();
                    let ikc = prompt_password("intermediate key (confirm):").unwrap();

                    if ik != ikc {
                        eprintln!("❌ Intermediate key and confirmation not matching");
                        std::process::exit(-1)
                    }

                    ik
                }
            };

            let password = match &maybe_main_password {
                Some(password) => password.clone(),
                None => prompt_password("main password:").unwrap(),
            };

            user_cfg = UserAuthData::new();
            match user_cfg.set_main(&password, &intermediate_key) {
                Ok(_) => {
                    // Force the write of the populated User structure
                    write_file = Some(true);
                }
                Err(err) => {
                    eprintln!("❌ Error in initializing the user authentication data: {err}");
                    std::process::exit(-1)
                }
            };
        }
        Command::Reset(_) => {
            match remove_user_data(&storage_source) {
                Ok(_) => {
                    // Do NOT rewrite the User structure that was created while authenticating the user
                    write_file = Some(false)
                }
                Err(err) => {
                    eprintln!("❌ Error in resetting user additional authentication methods: {err}");
                    std::process::exit(-1)
                }
            }
        }
        Command::Inspect(_) => {
            match &storage_source {
                StorageSource::Username(username) => {
                    println!("-----------------------------------------------------------");
                    println!("👤 User: {username}");
                    println!("-----------------------------------------------------------");
                }
                StorageSource::File(path) => {
                    println!("-----------------------------------------------------------");
                    println!("📁 Path: {}", path.to_string_lossy());
                    println!("-----------------------------------------------------------");
                }
            }

            match user_mounts {
                Some(ref mount_info) => {
                    let hash = mount_info.hash();
                    println!("🔑 hash: {hash}");

                    let primary_mount = mount_info.mount();
                    println!("💾 device: {}", primary_mount.device());
                    if !primary_mount.fstype().is_empty() {
                        println!("📂 filesystem: {}", primary_mount.fstype());
                    }

                    println!("⚙️  args: {}", primary_mount.flags().join(","));

                    mount_info.foreach(|a, b| {
                        println!("***********************************************************");
                        println!("    📁 directory: {}", a.clone());
                        println!("    💾 device: {}", b.device().clone());
                        println!("    📂 filesystem: {}", b.fstype().clone());
                        println!("    ⚙️  args: {}", b.flags().join(","))
                    });
                }
                None => println!("ℹ️  No user-defined mounts"),
            }

            println!("-----------------------------------------------------------");

            match load_user_session_command(&storage_source) {
                Ok(maybe_data) => match maybe_data {
                    Some(data) => {
                        let cmd = data.command();
                        println!("🖥️  Default session command: {cmd}")
                    }
                    None => println!("ℹ️  No default session set."),
                },
                Err(err) => {
                    eprintln!("❌ Error in reading the user default session: {err}");
                    std::process::exit(-1)
                }
            };

            println!("-----------------------------------------------------------");

            let methods_count = user_cfg.secondary().len();
            match methods_count {
                0 => {
                    println!("ℹ️  No authentication methods configured.");
                }
                1 => {
                    println!("🔐 There is 1 authentication method: ");
                    println!("-----------------------------------------------------------");
                }
                _ => {
                    println!(
                        "🔐 There are {} authentication methods: ",
                        user_cfg.secondary().len()
                    );
                    println!("-----------------------------------------------------------");
                }
            }

            for s in user_cfg.secondary() {
                println!("🏷️  name: {}", s.name());
                println!(
                    "    📅 created at: {:?}",
                    Local
                        .timestamp_opt(s.creation_date() as i64, 0)
                        .unwrap()
                        .to_string()
                );
                println!("    🔑 type: {}", s.type_name());
                println!("-----------------------------------------------------------");
            }
        }
        Command::Add(add_cmd) => {
            let intermediate_password = match user_cfg.has_main() {
                false => add_cmd.intermediate.clone().unwrap_or_else(|| {
                    let intermediate_password = prompt_password("Intermediate key:")
                        .expect("Failed to read intermediate key");

                    let intermediate_password_repeat =
                        prompt_password("Intermediate key (repeat):")
                            .expect("Failed to read intermediate key (repeat)");

                    if intermediate_password != intermediate_password_repeat {
                        eprintln!("❌ Intermediate key and Intermediate (repeat) do not match!");

                        std::process::exit(-1)
                    }

                    intermediate_password
                }),
                true => add_cmd.intermediate.clone().unwrap_or_else(|| {
                    prompt_password("Intermediate key:").expect("Failed to read intermediate key")
                }),
            };

            if user_cfg.has_main() {
                if let Err(err) = user_cfg.main_by_auth(&Some(intermediate_password.clone())) {
                    eprintln!("❌ Could not verify the correctness of the intermediate key: {err}");
                    std::process::exit(-1)
                }
            }

            // if the main password is accepted update the stored one
            if let Some(main_password) = maybe_main_password {
                user_cfg
                    .set_main(&main_password, &intermediate_password)
                    .expect("Error handling main password");
            }

            match add_cmd.method {
                AddAuthMethod::Password(add_auth_password_command) => {
                    let secondary_password = match add_auth_password_command.secondary_pw {
                        Some(secondary_password) => secondary_password,
                        None => {
                            let secondary_password = prompt_password("Secondary password:")
                                .expect("Failed to read secondary password");

                            let repeat = prompt_password("Secondary password (repeat):")
                                .expect("Failed to read secondary password (repeat)");
                            if secondary_password != repeat {
                                eprintln!("❌ Passwords do not match.\nAborting.");
                                std::process::exit(-1)
                            }

                            secondary_password
                        }
                    };

                    if !user_cfg.has_main() {
                        eprintln!("❌ Cannot add a secondary password for an account with no main password.\nAborting.");
                        std::process::exit(-1);
                    }

                    match user_cfg.add_secondary_password(
                        &add_cmd.name,
                        &intermediate_password,
                        &secondary_password,
                    ) {
                        Ok(_) => {
                            write_file = Some(true);
                            println!("✅ Secondary password added.");
                        }
                        Err(err) => {
                            eprintln!("❌ Error adding a secondary password: {err}.\nAborting.");
                            std::process::exit(-1);
                        }
                    }
                }
            }
        }
    }

    if write_file.unwrap_or_default() {
        store_user_auth_data(user_cfg, &storage_source)
            .expect("Error saving the updated user auth data.\nAborting.");

        store_user_mountpoints(user_mounts, &storage_source)
            .expect("Error saving the updated user mount data.\nAborting.");
    }
}
