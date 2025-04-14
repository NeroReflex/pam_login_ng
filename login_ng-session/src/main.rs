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

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use login_ng_session::dbus::SessionManagerDBus;
use login_ng_session::desc::NodeServiceDescriptor;
use login_ng_session::errors::SessionManagerError;
use login_ng_session::manager::SessionManager;
use login_ng_session::node::{SessionNode, SessionNodeRestart};
use nix::unistd::{getuid, User};
use tokio::sync::RwLock;
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), SessionManagerError> {
    let uid = getuid();
    let user = User::from_uid(uid)
        .expect("Failed to get user information")
        .unwrap();
    let load_directoried = vec![
        PathBuf::from(user.clone().dir)
            .join(".config")
            .join("login_ng-session"),
        PathBuf::from("/etc/login_ng-session/"),
    ];

    let default_service_name = String::from("default.service");

    let mut nodes = HashMap::new();
    let mut currently_loading = HashSet::new();
    match NodeServiceDescriptor::find_and_load(
        &mut nodes,
        &default_service_name,
        load_directoried.as_slice(),
        &mut currently_loading,
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
                    let shell = user.clone().shell.to_string_lossy().into_owned();

                    eprintln!(
                        "Definition for {default_service_name} not found: using shell {shell}"
                    );

                    nodes = HashMap::from([(
                        default_service_name.clone(),
                        Arc::new(RwLock::new(SessionNode::new(
                            shell.clone(),
                            &[],
                            nix::sys::signal::Signal::SIGINT,
                            SessionNodeRestart::no_restart(),
                            vec![],
                        ))),
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
        },
    };

    let manager = Arc::new(RwLock::new(SessionManager::new(nodes)));

    // This is the default user dbus address
    // DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
    // where /run/user/1000 is XDG_RUNTIME_DIR
    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Starting dbus service on socket {value}"),
        Err(err) => {
            eprintln!("Couldn't read dbus socket address: {err} - using default...");
            match std::env::var("XDG_RUNTIME_DIR") {
                Ok(xdg_runtime_dir) => std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={xdg_runtime_dir}/bus").as_str(),
                ),
                Err(err) => {
                    eprintln!("Unable to generate the default dbus address {err}")
                }
            }
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

    loop {
        let mut guard = manager.write().await;

        // here collect info on running stuff
        match guard
            .step(&default_service_name, Duration::from_millis(250))
            .await
        {
            Ok(is_stalled) => match is_stalled {
                Some(_) => break,
                None => tokio::time::sleep(Duration::from_nanos(100)).await,
            },
            Err(err) => return Err(err),
        }
    }

    drop(dbus_manager);

    manager
        .write()
        .await
        .wait_idle(&default_service_name, Duration::from_millis(250))
        .await?;

    Ok(())
}
