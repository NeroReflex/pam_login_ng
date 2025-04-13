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

use std::{ops::Deref, process::ExitStatus, sync::Arc, time::Duration, u64};

use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};

use tokio::{
    process::{Child, Command},
    sync::RwLock,
    time::{self, Instant},
};

#[derive(Debug)]
pub struct SessionNodeRestart {
    times: u64,
    delay: Duration,
}

impl SessionNodeRestart {
    pub fn new(times: u64, delay: Duration) -> Self {
        Self { times, delay }
    }

    pub fn no_restart() -> Self {
        Self {
            times: u64::MIN,
            delay: Duration::from_secs(5),
        }
    }
}

impl Default for SessionNodeRestart {
    fn default() -> Self {
        Self {
            times: u64::MAX,
            delay: Duration::from_secs(5),
        }
    }
}

#[derive(Debug)]
pub enum SessionNodeStopReason {
    StartError(std::io::Error),
    Completed(ExitStatus),
    Stopped(std::io::Error),
    Manual,
}

#[derive(Debug, Clone)]
pub enum SessionNodeStatus {
    Ready,
    Running(Arc<RwLock<Child>>),
    Stopped {
        time: time::Instant,
        reason: Arc<SessionNodeStopReason>,
    },
}

#[derive(Debug)]
pub struct SessionNode {
    stop_signal: Signal,
    restart: SessionNodeRestart,
    restarted: u64,
    command: Command,
    status: SessionNodeStatus,
    dependencies: Vec<Arc<RwLock<SessionNode>>>,
}

impl SessionNode {
    pub fn new(
        cmd: String,
        args: &[String],
        stop_signal: Signal,
        restart: SessionNodeRestart,
        dependencies: Vec<Arc<RwLock<SessionNode>>>,
    ) -> Self {
        let mut command = Command::new(cmd);
        command.args(args);
        let restarted = 0u64;
        let status = SessionNodeStatus::Ready;

        Self {
            restarted,
            command,
            status,
            restart,
            stop_signal,
            dependencies,
        }
    }

    pub async fn is_running(&self) -> bool {
        if let SessionNodeStatus::Running(_) = self.status {
            return true;
        }

        for dep in self.dependencies.iter() {
            let dep_guard = dep.read().await;
            if Box::pin(dep_guard.is_running()).await {
                return false;
            }
        }

        false
    }

    pub async fn issue_manual_stop(&mut self) {
        if let SessionNodeStatus::Running(proc) = &self.status {
            let mut proc_guard = proc.write().await;

            match proc_guard.id() {
                Some(pid) => {
                    match signal::kill(Pid::from_raw(pid.try_into().unwrap()), self.stop_signal) {
                        Ok(_) => match proc_guard.wait().await {
                            Ok(exit_status) => todo!(),
                            Err(err) => todo!(),
                        },
                        Err(err) => todo!(),
                    }
                }
                None => match proc_guard.kill().await {
                    Ok(_) => todo!(),
                    Err(err) => todo!(),
                },
            }
        }

        todo!()
    }

    pub async fn poll(&mut self) -> bool {
        let mut stalled = false;

        self.status = match &self.status {
            SessionNodeStatus::Ready => match self.command.spawn() {
                Ok(child) => SessionNodeStatus::Running(Arc::new(RwLock::new(child))),
                Err(err) => SessionNodeStatus::Stopped {
                    time: time::Instant::now(),
                    reason: Arc::new(SessionNodeStopReason::StartError(err)),
                },
            },
            SessionNodeStatus::Running(proc) => match proc.write().await.try_wait() {
                Ok(possible_exit_status) => match possible_exit_status {
                    Some(exit_status) => SessionNodeStatus::Stopped {
                        time: time::Instant::now(),
                        reason: Arc::new(SessionNodeStopReason::Completed(exit_status)),
                    },
                    None => SessionNodeStatus::Running(proc.clone()),
                },
                Err(err) => SessionNodeStatus::Stopped {
                    time: time::Instant::now(),
                    reason: Arc::new(SessionNodeStopReason::Stopped(err)),
                },
            },
            SessionNodeStatus::Stopped { time, reason } => {
                stalled = match reason.deref() {
                    SessionNodeStopReason::StartError(_) => self.restart.times < self.restarted,
                    SessionNodeStopReason::Completed(exit_status) => {
                        exit_status.success() || self.restart.times < self.restarted
                    }
                    SessionNodeStopReason::Stopped(_) => self.restart.times < self.restarted,
                    SessionNodeStopReason::Manual => true,
                };

                match time.checked_add(self.restart.delay) {
                    Some(restart_time) => match Instant::now() >= restart_time {
                        true => {
                            if !stalled {
                                SessionNodeStatus::Ready
                            } else {
                                self.status.clone()
                            }
                        }
                        false => self.status.clone(),
                    },
                    None => self.status.clone(),
                }
            }
        };

        stalled
    }
}
