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

use std::sync::Arc;

use zbus::interface;

use crate::manager::SessionManager;

#[derive(Debug, Clone)]
pub struct SessionManagerDBus {
    manager: Arc<SessionManager>,
}

impl SessionManagerDBus {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        Self { manager }
    }
}

#[interface(
    name = "org.neroreflex.login_ng_service1",
    proxy(
        default_service = "org.neroreflex.login_ng_service",
        default_path = "/org/zbus/login_ng_service"
    )
)]
impl SessionManagerDBus {
    pub async fn start(&self, target: String) -> u32 {
        todo!()
    }

    pub async fn stop(&self, target: String) -> u32 {
        todo!()
    }

    pub async fn is_running(&self, target: String) -> (u32, bool) {
        match self.manager.is_running(&target).await {
            Ok(response) => (0, response),
            Err(err) => {
                eprintln!("Error in fetching the running status of {target}: {err}");
                (1, false)
            }
        }
    }

    pub async fn change(&self, target: String, cmd: String, args: Vec<String>) -> u32 {
        todo!()
    }

    pub async fn terminate(&self) -> u32 {
        todo!()
    }
}
