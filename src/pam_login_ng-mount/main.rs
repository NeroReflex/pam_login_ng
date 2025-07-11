/*
    login-ng A greeter written in rust that also supports autologin with systemd-homed
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

extern crate tokio;

use std::path::PathBuf;

use login_ng::{
    pam::{mount::MountAuthDBusProxy, result::ServiceOperationResult, ServiceError},
    storage::{load_user_mountpoints, StorageSource},
    zbus::Connection,
};

use argh::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// Command line tool for managing login-ng authentication methods
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
/// Subcommands for managing authentication methods
enum Command {
    Info(InfoCommand),
    Authorize(AuthorizeCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Print information about the software
#[argh(subcommand, name = "info")]
struct InfoCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Authorize a user to mount devices on each login
#[argh(subcommand, name = "authorize")]
struct AuthorizeCommand {
    #[argh(option, short = 'u')]
    /// username of the user target to the action
    username: String,

    #[argh(option, short = 'd')]
    /// force the use of a specific home directory
    directory: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let args: Args = argh::from_env();

    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Starting dbus service on socket {value}"),
        Err(err) => {
            println!("Couldn't read dbus socket address: {err} - using default...");
            std::env::set_var(
                "DBUS_SESSION_BUS_ADDRESS",
                "unix:path=/run/dbus/system_bus_socket",
            );
        }
    }

    let connection = Connection::session().await?;

    let proxy = MountAuthDBusProxy::new(&connection).await?;

    match args.command {
        Command::Info(_) => {
            let version = login_ng::LIBRARY_VERSION;
            println!("login-ng version {version}, Copyright (C) 2024 Denis Benato");
            println!("login-ng comes with ABSOLUTELY NO WARRANTY;");
            println!("This is free software, and you are welcome to redistribute it");
            println!("under certain conditions.");
            println!("\n");
        }
        Command::Authorize(auth_data) => {
            let storage_source = match auth_data.directory {
                Some(path) => StorageSource::Path(path),
                _ => StorageSource::Username(auth_data.username.clone()),
            };

            let user_mounts = match load_user_mountpoints(&storage_source) {
                Ok(existing_data) => existing_data,
                Err(err) => {
                    eprintln!("Error in loading user mounts data: {err}");
                    std::process::exit(-1)
                }
            };

            let Some(loaded_mounts) = user_mounts else {
                eprintln!("User does not have mounts configured");
                std::process::exit(-1)
            };

            let reply = proxy
                .authorize(auth_data.username.as_str(), loaded_mounts.hash())
                .await?;

            let result = ServiceOperationResult::from(reply);

            if result != ServiceOperationResult::Ok {
                eprintln!("Error in authorizing the user mouunt: {result}");
                std::process::exit(-1)
            }
        }
    };

    Ok(())
}
