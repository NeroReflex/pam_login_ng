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

use std::{collections::HashMap, process::ExitStatus, sync::Arc, time::Duration};

use tokio::{
    process::{Child, Command},
    select,
    sync::RwLock,
    time::timeout,
};

use zbus::interface;

use login_ng::command::SessionCommand;

use crate::errors::SessionManagerError;

#[derive(Debug)]
pub enum SessionStatus {
    Ready(Command),
    Running(Child),
    StoppedSuccessfully(ExitStatus),
    StoppedErrored(std::io::Error),
    Errored(std::io::Error),
}

#[derive(Debug, Default)]
pub struct SessionManager {
    services: HashMap<String, SessionStatus>,
}

impl SessionManager {
    pub fn new(map: HashMap<String, SessionCommand>) -> Self {
        let services = map
            .into_iter()
            .map(|(name, cmd)| {
                (name.clone(), {
                    let mut ready_cmd = Command::new(cmd.command());
                    ready_cmd.args(cmd.args().as_slice());
                    SessionStatus::Ready(ready_cmd)
                })
            })
            .collect::<HashMap<String, SessionStatus>>();

        Self { services }
    }

    pub async fn is_running(&self, target: &String) -> Result<bool, SessionManagerError> {
        let target_string = String::from(target);
        match self.services.get(&target_string) {
            Some(status) => match status {
                SessionStatus::Running(_) => Ok(true),
                _ => Ok(false),
            },
            None => Err(SessionManagerError::NotFound(target_string)),
        }
    }

    pub async fn load(&mut self) -> Result<(), SessionManagerError> {
        todo!()
    }

    pub async fn wait_idle(&mut self) -> Result<(), SessionManagerError> {
        // await until everything goes idle
        loop {
            if self.step(Duration::from_secs(30)).await? == 0 {
                break;
            }
        }

        Ok(())
    }

    pub async fn step(
        &mut self,
        process_await_delay: Duration,
    ) -> Result<usize, SessionManagerError> {
        let mut still_running = 0;

        for task in self.services.iter_mut() {
            let tast_status = task.1;
            let target = task.0.as_str();
            match tast_status {
                SessionStatus::Ready(proc) => {
                    *tast_status = {
                        match proc.spawn() {
                            Ok(child) => SessionStatus::Running(child),
                            Err(err) => {
                                eprintln!("Error starting {target}: {err}");
                                SessionStatus::Errored(err)
                            }
                        }
                    }
                }
                SessionStatus::Running(proc) => select! {
                    wait_proc_res = timeout(process_await_delay, proc.wait()) => {
                        if let Ok(proc_res) = wait_proc_res {
                            match proc_res {
                                Ok(exit_status) => {
                                    *tast_status = SessionStatus::StoppedSuccessfully(exit_status)
                                },
                                Err(exit_err) => {
                                    eprintln!("Service errored {target}: {exit_err}");
                                    *tast_status = SessionStatus::StoppedErrored(exit_err)
                                },
                            }
                        } else {
                            still_running += 1;
                        }
                    },
                },
                _ => {}
            }
        }

        Ok(still_running)
    }
}

#[derive(Debug, Clone)]
pub struct SessionManagerDBus {
    manager: Arc<RwLock<SessionManager>>,
}

impl SessionManagerDBus {
    pub fn new(manager: Arc<RwLock<SessionManager>>) -> Self {
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
        let guard = self.manager.read().await;

        match guard.is_running(&target).await {
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
