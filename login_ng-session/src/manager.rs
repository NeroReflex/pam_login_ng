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

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use tokio::task::{self, JoinSet};

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
    runtime_dir: PathBuf,
    services: HashMap<String, Arc<SessionNode>>,
}

impl SessionManager {
    pub fn new(runtime_dir: PathBuf, map: HashMap<String, Arc<SessionNode>>) -> Self {
        let services = map
            .into_iter()
            .map(|(name, node)| (name.clone(), node.clone()))
            .collect::<HashMap<String, Arc<SessionNode>>>();

        Self {
            runtime_dir,
            services,
        }
    }

    pub async fn is_running(&self, target: &String) -> Result<bool, SessionManagerError> {
        match self.services.get(target) {
            Some(node) => Ok(node.is_running().await),
            None => Err(SessionManagerError::NotFound(target.clone())),
        }
    }

    pub async fn run(&self, target: &String) -> Result<(), SessionManagerError> {
        let mut other_nodes = vec![];
        let mut main_node = None;

        for (node_name, node_value) in self.services.iter() {
            if *target == *node_name {
                main_node = Some(node_value.clone())
            } else {
                other_nodes.push(node_value.clone());
            }
        }

        let Some(main_node) = main_node else {
            return Err(SessionManagerError::NotFound(target.clone()))
        };

        // start all services and let those sync themselves
        let node_run_tasks = other_nodes.iter().map(|node| {
            let n = node.clone();
            let runtime_dir = self.runtime_dir.clone();
            async move {
                SessionNode::run(runtime_dir, n).await
            }
        }).collect::<JoinSet<_>>();

        // wait for the target run to exit
        let runtime_dir = self.runtime_dir.clone();
        let (main_node_res, other_nodes_res) = tokio::join!(task::spawn(async move {
            SessionNode::run(runtime_dir, main_node).await
        }), node_run_tasks.join_all());

        Ok(())
    }

    pub async fn step(
        &mut self,
        target: &String,
        minimum_step_delay: Duration,
    ) -> Result<Option<SessionStalledReason>, SessionManagerError> {
        let node = self.services.get(target).unwrap();

        let (_sleep_res, stalled) =
            tokio::join!(tokio::time::sleep(minimum_step_delay), node.poll());

        Ok(stalled)
    }
}
