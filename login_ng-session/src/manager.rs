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

use std::{collections::HashMap, sync::Arc};

use tokio::{process::Command, sync::RwLock};

use zbus::interface;

#[derive(Debug, Clone)]
pub struct SessionCommand {
    cmd: String,
    args: Vec<String>,
}

#[derive(Debug)]
pub enum SessionStatus {
    Running(Command),
    Stopped,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Default)]
pub struct SessionManager {
    services_cmd: HashMap<String, SessionCommand>,
    services_status: HashMap<String, SessionStatus>,
}

impl SessionManager {
    pub fn new(map: HashMap<String, SessionCommand>) -> Self {
        let services_status = map
            .iter()
            .map(|(name, _)| (name.clone(), SessionStatus::Stopped))
            .collect::<HashMap<String, SessionStatus>>();

        let services_cmd = map.clone();        

        Self {
            services_cmd,
            services_status,
        }
    }

    pub fn is_running(&self, target: &str) -> Result<bool, i32> {
        todo!()
    }

    pub fn load(&self) -> Result<(), i32> {
        todo!()
    }
}

#[derive(Debug)]
pub struct SessionManagerDBus {
    manager: Arc<RwLock<SessionManager>>,
}

impl SessionManagerDBus {
    pub fn new(manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { manager }
    }
}

#[interface(
    name = "org.neroreflex.login_ng_mount1",
    proxy(
        default_service = "org.neroreflex.login_ng_mount",
        default_path = "/org/zbus/login_ng_mount"
    )
)]
impl SessionManagerDBus {
    pub async fn start(&self, target: &str) -> u32 {
        todo!()
    }

    pub async fn stop(&self, target: &str) -> u32 {
        todo!()
    }

    pub async fn change(&self, target: &str, cmd: String, args: Vec<String>) -> u32 {
        todo!()
    }
}
