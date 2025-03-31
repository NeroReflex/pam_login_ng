/*
    login-ng A greeter written in rust that also supports autologin with systemd-homed
    Copyright (C) 2024  Denis Benato

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

use std::fs::{self, create_dir, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use pam_login_ng_common::mount::{MountAuth, MountAuthDBus};
use pam_login_ng_common::rsa::pkcs1::EncodeRsaPrivateKey;
use pam_login_ng_common::rsa::pkcs8::LineEnding;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::RwLock;

use std::os::unix::fs::PermissionsExt;

use pam_login_ng_common::{
    login_ng::users, service::ServiceError, session::Sessions, zbus::connection,
};

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    if users::get_current_uid() != 0 {
        eprintln!("🚫 Application started without root privileges: aborting...");
        return Err(ServiceError::MissingPrivilegesError);
    }

    let file_name_str = "private_key_pkcs1.pem";
    let dir_path_str = "/etc/login_ng/";
    let dir_path = Path::new(dir_path_str);

    if !dir_path.exists() {
        match create_dir(dir_path) {
            Ok(_) => {
                println!("📁 Directory {dir_path_str} created");

                let mut permissions = fs::metadata(dir_path)?.permissions();
                permissions.set_mode(0o700);

                fs::set_permissions(dir_path, permissions)?;
            }
            Err(err) => {
                eprintln!("❌ Could not create directory {dir_path_str}: {err}");
            }
        }
    }

    let file_path = dir_path.join(file_name_str);

    let contents = match file_path.exists() {
        true => {
            let mut contents = String::new();

            let mut file = File::open(file_path)?;
            let read = file.read_to_string(&mut contents)?;
            println!("📖 Read private key file of {read} bytes");

            contents
        }
        false => {
            eprintln!(
                "🖊️ File {dir_path_str}/{file_name_str} not found: a new one will be generated..."
            );

            let mut rng = pam_login_ng_common::rand::thread_rng();
            let priv_key = pam_login_ng_common::rsa::RsaPrivateKey::new(&mut rng, 4096)
                .expect("failed to generate a key");

            let contents = priv_key.to_pkcs1_pem(LineEnding::CRLF)?.to_string();

            match File::create(&file_path) {
                Ok(mut file) => {
                    let metadata = file.metadata()?;
                    let mut perm = metadata.permissions();
                    perm.set_mode(0o700);

                    fs::set_permissions(file_path, perm)?;
                    match file.write_all(contents.to_string().as_bytes()) {
                        Ok(_) => {
                            println!(
                                "✅ Generated key has been saved to {dir_path_str}/{file_name_str}"
                            )
                        }
                        Err(err) => {
                            eprintln!("❌ Failed to write the generated key to {dir_path_str}/{file_name_str}: {err}")
                        }
                    };
                }
                Err(err) => {
                    eprintln!("Failed to create the file {dir_path_str}/{file_name_str}: {err}")
                }
            };

            contents
        }
    };

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

    let mounts_auth = Arc::new(RwLock::new(
        MountAuth::default(), /*load_from_file("").unwrap()*/
    ));

    println!("🔧 Building the dbus object...");

    let dbus_mounts_auth_con = connection::Builder::session()
        .map_err(ServiceError::ZbusError)?
        .name("org.neroreflex.login_ng_mount")
        .map_err(ServiceError::ZbusError)?
        .serve_at(
            "/org/zbus/login_ng_mount",
            MountAuthDBus::new(dir_path.join("authorized_mounts.json"), mounts_auth.clone()),
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
            "/org/zbus/login_ng_session",
            Sessions::new(mounts_auth, contents.as_str()),
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
