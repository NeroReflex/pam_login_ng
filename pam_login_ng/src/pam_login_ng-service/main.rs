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

use login_ng::users;

use thiserror::Error;

use std::future::pending;
use zbus::{connection, interface, Error as ZError};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Permission error: not running as the root user")]
    MissingPrivilegesError,

    #[error("DBus error: {0}")]
    ZbusError(#[from] ZError),
}

struct Service {}

impl Service {
    pub fn new() -> Self {
        Self {}
    }
}

#[interface(name = "org.zbus.pam_login_ng")]
impl Service {
    fn open_user_session(&mut self, user: &str) -> u32 {
        0u32
    }

    fn close_user_session(&mut self, user: &str) -> u32 {
        0u32
    }
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    if users::get_current_uid() != 0 {
        return Err(ServiceError::MissingPrivilegesError);
    }

    let service = Service::new();

    let _conn = connection::Builder::session()
        .map_err(|err| ServiceError::ZbusError(err))?
        .name("org.zbus.pam_login_ng")
        .map_err(|err| ServiceError::ZbusError(err))?
        .serve_at("/org/zbus/pam_login_ng", service)
        .map_err(|err| ServiceError::ZbusError(err))?
        .build()
        .await
        .map_err(|err| ServiceError::ZbusError(err))?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
