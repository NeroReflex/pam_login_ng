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

use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;

use crate::{
    errors::SessionManagerError,
    node::{SessionNode, SessionStalledReason},
};

pub struct ManagerStatus {
    running: Vec<String>,
}

impl ManagerStatus {
    pub fn is_idle(&self) -> bool {
        self.running.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct SessionManager {
    services: HashMap<String, Arc<RwLock<SessionNode>>>,
}

impl SessionManager {
    pub fn new(map: HashMap<String, Arc<RwLock<SessionNode>>>) -> Self {
        let services = map
            .into_iter()
            .map(|(name, node)| (name.clone(), node.clone()))
            .collect::<HashMap<String, Arc<RwLock<SessionNode>>>>();

        Self { services }
    }

    pub async fn is_running(&self, target: &String) -> Result<bool, SessionManagerError> {
        match self.services.get(target) {
            Some(node) => Ok(node.read().await.is_running().await),
            None => Err(SessionManagerError::NotFound(target.clone())),
        }
    }

    pub async fn wait_idle(
        &mut self,
        target: &String,
        minimum_step_delay: Duration,
    ) -> Result<(), SessionManagerError> {
        // await until the target goes stalled (loop while it is NOT stalled)
        while self.step(target, minimum_step_delay).await?.is_none() {}

        Ok(())
    }

    pub async fn step(
        &mut self,
        target: &String,
        minimum_step_delay: Duration,
    ) -> Result<Option<SessionStalledReason>, SessionManagerError> {
        let mut guard = self.services.get(target).unwrap().write().await;

        let (_sleep_res, stalled) =
            tokio::join!(tokio::time::sleep(minimum_step_delay), guard.poll());

        Ok(stalled)
    }
}
