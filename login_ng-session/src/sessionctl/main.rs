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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use argh::FromArgs;
use login_ng_session::dbus::{SessionManagerDBus, SessionManagerDBusProxy};
use zbus::Connection;

#[derive(FromArgs, PartialEq, Debug)]
/// Command line tool for managing login_ng-session
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
/// Subcommands for managing login_ng-session
enum Command {
    Stop(StopCommand),
    Restart(RestartCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Stop a target from within login_ng-session
#[argh(subcommand, name = "stop")]
struct StopCommand {
    #[argh(option)]
    /// the target to be stopped
    target: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Restart a target from within login_ng-session
#[argh(subcommand, name = "restart")]
struct RestartCommand {
    #[argh(option)]
    /// the target to be restarted
    target: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // the XDG_RUNTIME_DIR is required for generating the default dbus socket path
    // and also the runtime directory (hopefully /tmp mounted) to keep track of services
    let xdg_runtime_dir = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap());

    // This is the default user dbus address
    // DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
    // where /run/user/1000 is XDG_RUNTIME_DIR
    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Starting dbus service on socket {value}"),
        Err(err) => {
            println!("Couldn't read dbus socket address: {err} - using default...");
            std::env::set_var(
                "DBUS_SESSION_BUS_ADDRESS",
                format!(
                    "unix:path={}/bus",
                    xdg_runtime_dir.as_os_str().to_string_lossy()
                )
                .as_str(),
            )
        }
    }

    let connection = Connection::session().await?;
    let proxy = SessionManagerDBusProxy::new(&connection).await?;

    let args: Args = argh::from_env();

    match &args.command {
        Command::Stop(stop_command) => {
            proxy.stop(stop_command.target.clone()).await.unwrap();

            Ok(())
        },
        Command::Restart(restart_command) => {
            proxy.restart(restart_command.target.clone()).await.unwrap();

            Ok(())
        },
    }
}
