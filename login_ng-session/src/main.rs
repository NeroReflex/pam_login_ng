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

use login_ng_session::dbus::SessionManagerDBus;
use login_ng_session::desc::NodeServiceDescriptor;
use login_ng_session::errors::SessionManagerError;
use login_ng_session::manager::SessionManager;
use login_ng_session::node::{SessionNode, SessionNodeRestart, SessionNodeType};
use login_ng::users::{get_user_by_name, os::unix::UserExt};
use std::time::{SystemTime, UNIX_EPOCH};
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), SessionManagerError> {
    let username = login_ng::users::get_current_username().unwrap();

    let user = get_user_by_name(username.as_os_str())
        .expect("Failed to get user information");
    let load_directoried = vec![
        user.clone().home_dir().join(".config").join("login_ng-session"),
        PathBuf::from("/etc/login_ng-session/"),
    ];

    let default_service_name = String::from("default.service");

    let mut nodes = HashMap::new();
    match NodeServiceDescriptor::load_tree(
        &mut nodes,
        &default_service_name,
        load_directoried.as_slice(),
    )
    .await
    {
        Ok(_) => {}
        Err(err) => match err {
            login_ng_session::errors::NodeLoadingError::IOError(err) => {
                eprintln!("File error: {err}");
                std::process::exit(-1)
            }
            login_ng_session::errors::NodeLoadingError::FileNotFound(filename) => {
                // if the default target is missing use the default user shell
                if filename == default_service_name {
                    let shell = user.shell().to_string_lossy().into_owned();

                    eprintln!(
                        "Definition for {default_service_name} not found: using shell {shell}"
                    );

                    nodes = HashMap::from([(
                        default_service_name.clone(),
                        Arc::new(SessionNode::new(
                            default_service_name.clone(),
                            SessionNodeType::Service,
                            shell.clone(),
                            vec![],
                            nix::sys::signal::Signal::SIGINT,
                            SessionNodeRestart::no_restart(),
                            vec![],
                        )),
                    )])
                } else {
                    eprintln!("Dependency not found: {filename}");
                    std::process::exit(-1)
                }
            }
            login_ng_session::errors::NodeLoadingError::CyclicDependency(filename) => {
                eprintln!("Cycle for target: {filename}");
                std::process::exit(-1)
            }
            login_ng_session::errors::NodeLoadingError::JSONError(err) => {
                eprintln!("JSON deserialization error: {err}");
                std::process::exit(-1)
            }
            login_ng_session::errors::NodeLoadingError::InvalidKind(err) => {
                eprintln!("JSON syntax error: unrecognised kind value {err}");
                std::process::exit(-1)
            }
        },
    };

    // the XDG_RUNTIME_DIR is required for generating the default dbus socket path
    // and also the runtime directory (hopefully /tmp mounted) to keep track of services
    let xdg_runtime_dir = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap());

    let manager_runtime_path = xdg_runtime_dir.join(format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    ));

    std::fs::create_dir(manager_runtime_path.clone()).unwrap();

    let manager = Arc::new(SessionManager::new(manager_runtime_path, nodes));

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

    let dbus_manager = connection::Builder::session()
        .map_err(SessionManagerError::ZbusError)?
        .name("org.neroreflex.login_ng_service")
        .map_err(SessionManagerError::ZbusError)?
        .serve_at(
            "/org/zbus/login_ng_service",
            SessionManagerDBus::new(manager.clone()),
        )
        .map_err(SessionManagerError::ZbusError)?
        .build()
        .await
        .map_err(SessionManagerError::ZbusError)?;

    println!("Running the session manager");

    manager.run(&default_service_name).await?;

    drop(dbus_manager);

    Ok(())
}
