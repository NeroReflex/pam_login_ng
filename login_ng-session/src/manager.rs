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

use std::{collections::HashMap, process::ExitStatus, time::Duration};

use tokio::{
    process::{Child, Command},
    select,
    time::timeout,
};

use login_ng::command::SessionCommand;

use crate::errors::SessionManagerError;

#[derive(Debug)]
pub enum ServiceStatus {
    Ready(Command),
    Running(Child),
    StoppedSuccessfully(ExitStatus),
    StoppedErrored(std::io::Error),
    Errored(std::io::Error),
}

#[derive(Debug, Default)]
pub struct SessionManager {
    services: HashMap<String, ServiceStatus>,
}

pub struct ManagerStatus {
    running: Vec<String>
}

impl ManagerStatus {
    pub fn is_idle(&self) -> bool {
        self.running.is_empty()
    }
}

impl SessionManager {
    pub fn new(map: HashMap<String, SessionCommand>) -> Self {
        let services = map
            .into_iter()
            .map(|(name, cmd)| {
                (name.clone(), {
                    let mut ready_cmd = Command::new(cmd.command());
                    ready_cmd.args(cmd.args().as_slice());
                    ServiceStatus::Ready(ready_cmd)
                })
            })
            .collect::<HashMap<String, ServiceStatus>>();

        Self { services }
    }

    pub async fn is_running(&self, target: &String) -> Result<bool, SessionManagerError> {
        let target_string = String::from(target);
        match self.services.get(&target_string) {
            Some(status) => match status {
                ServiceStatus::Running(_) => Ok(true),
                _ => Ok(false),
            },
            None => Err(SessionManagerError::NotFound(target_string)),
        }
    }

    pub async fn load(&mut self, target: &String, cmd: &String, args: &[String]) -> Result<(), SessionManagerError> {
        let mut command = Command::new(cmd);
        command.args(args);

        match self.services.get(target) {
            Some(status) => match status {
                _ => {
                    eprintln!("");
                    todo!()
                }
            },
            None => {
                let _ = self.services.insert(target.clone(), ServiceStatus::Ready(command));
                Ok(())
            },
        }
    }

    pub async fn wait_idle(&mut self) -> Result<(), SessionManagerError> {
        // await until everything goes idle
        loop {
            if self.step(Duration::from_secs(30)).await?.is_idle() {
                break;
            }
        }

        Ok(())
    }

    pub async fn step(
        &mut self,
        process_await_delay: Duration,
    ) -> Result<ManagerStatus, SessionManagerError> {
        let mut running = Vec::new();

        for task in self.services.iter_mut() {
            let tast_status = task.1;
            let target = task.0;
            match tast_status {
                ServiceStatus::Ready(proc) => {
                    *tast_status = {
                        match proc.spawn() {
                            Ok(child) => ServiceStatus::Running(child),
                            Err(err) => {
                                eprintln!("Service errored starting {target}: {err}");
                                ServiceStatus::Errored(err)
                            }
                        }
                    }
                }
                ServiceStatus::Running(proc) => select! {
                    wait_proc_res = timeout(process_await_delay, proc.wait()) => {
                        if let Ok(proc_res) = wait_proc_res {
                            match proc_res {
                                Ok(exit_status) => {
                                    *tast_status = ServiceStatus::StoppedSuccessfully(exit_status)
                                },
                                Err(exit_err) => {
                                    eprintln!("Service errored awaiting termination {target}: {exit_err}");
                                    *tast_status = ServiceStatus::StoppedErrored(exit_err)
                                },
                            }
                        } else {
                            running.push(target.clone());
                        }
                    },
                },
                _ => {}
            }
        }

        Ok(ManagerStatus {
            running
        })
    }
}
