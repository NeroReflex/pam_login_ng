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

use login_ng_session::errors::SessionManagerError;
use login_ng_session::login_ng::command::SessionCommand;
use login_ng_session::manager::SessionManager;
use login_ng_session::manager::SessionManagerDBus;
use tokio::sync::RwLock;
use zbus::connection;

#[tokio::main]
async fn main() -> Result<(), SessionManagerError> {

    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Starting dbus service on socket {value}"),
        Err(err) => {
            eprintln!("🟠 Couldn't read dbus socket address: {err} - using default...");
            std::env::set_var(
                "DBUS_SESSION_BUS_ADDRESS",
                "unix:path=/run/dbus/system_bus_socket",
            );
        }
    }

    let preloaded = HashMap::from([
        (String::from("desktop"), SessionCommand::new(String::from("Hyprland"), vec![]))
    ]);

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

        main_service_exited = guard.is_running("desktop").await?;
    }

    let mut guard = manager.write().await;
    guard.terminate().await?;

    drop(dbus_manager);

    Ok(())
}
