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
use std::sync::Arc;

use login_ng_session::dbus::SessionManagerDBus;
use login_ng_session::errors::SessionManagerError;
use login_ng_session::login_ng::command::SessionCommand;
use login_ng_session::manager::SessionManager;
use tokio::sync::RwLock;
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), SessionManagerError> {
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

    let default_service_name = String::from("default");

    let preloaded = HashMap::from([(
        default_service_name.clone(),
        SessionCommand::new(String::from("Hyprland"), vec![]),
    )]);

    let manager = Arc::new(RwLock::new(SessionManager::new(preloaded)));

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

    let mut main_service_exited = false;
    while !main_service_exited {
        let mut guard = manager.write().await;

        // here collect info on running stuff
        let _ = tokio::join!(
            guard.step(tokio::time::Duration::from_millis(1)),
            tokio::time::sleep(tokio::time::Duration::from_millis(250))
        );

        main_service_exited = guard.is_running(&default_service_name).await?;
    }

    let mut guard = manager.write().await;
    guard.wait_idle().await?;

    drop(dbus_manager);

    Ok(())
}
