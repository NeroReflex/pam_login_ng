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

use login_ng::{
    pam::{
        disk::create_directory,
        mount::{MountAuthDBus, MountAuthOperations},
        session::Sessions,
        ServiceError,
    },
    users,
    zbus::connection,
};

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::RwLock;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    if users::get_current_uid() != 0 {
        eprintln!("🚫 Application started without root privileges: aborting...");
        return Err(ServiceError::MissingPrivilegesError);
    }

    let private_key_file_name_str = "private_key_pkcs1.pem";
    let authorization_file_name_str = "authorized_mounts.json";
    let dir_path_str = match std::fs::exists("/usr/lib/login_ng/").unwrap_or(false) {
        true => "/usr/lib/login_ng/",
        false => "/etc/login_ng/",
    };

    create_directory(PathBuf::from(dir_path_str)).await?;

    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(value) => println!("Connecting to dbus service on socket {value}"),
        Err(err) => {
            println!("🟠 Couldn't read dbus socket address: {err} - using default...");
            std::env::set_var(
                "DBUS_SESSION_BUS_ADDRESS",
                "unix:path=/run/dbus/system_bus_socket",
            );
        }
    }

    let mounts_auth = Arc::new(RwLock::new(MountAuthOperations::new(
        Path::new(dir_path_str).join(authorization_file_name_str),
    )));

    println!("🔧 Building the dbus object...");

    let dbus_mounts_auth_con = connection::Builder::session()
        .map_err(ServiceError::ZbusError)?
        .name("org.neroreflex.login_ng_mount")
        .map_err(ServiceError::ZbusError)?
        .serve_at(
            "/org/neroreflex/login_ng_mount",
            MountAuthDBus::new(mounts_auth.clone()),
        )
        .map_err(ServiceError::ZbusError)?
        .build()
        .await
        .map_err(ServiceError::ZbusError)?;

    let dbus_session_conn = connection::Builder::session()
        .map_err(ServiceError::ZbusError)?
        .name("org.neroreflex.login_ng_session")
        .map_err(ServiceError::ZbusError)?
        .serve_at(
            "/org/neroreflex/login_ng_session",
            Sessions::new(
                Path::new(dir_path_str).join(private_key_file_name_str),
                mounts_auth,
            ),
        )
        .map_err(ServiceError::ZbusError)?
        .build()
        .await
        .map_err(ServiceError::ZbusError)?;

    println!("🔄 Application running");

    // Create a signal listener for SIGTERM
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal handler");

    // Wait for a SIGTERM signal
    sigterm.recv().await;

    drop(dbus_session_conn);
    drop(dbus_mounts_auth_con);

    Ok(())
}
