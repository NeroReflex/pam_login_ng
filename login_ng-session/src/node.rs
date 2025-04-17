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

use std::{ops::Deref, path::PathBuf, process::ExitStatus, sync::Arc, time::Duration, u64};

use nix::{
    sys::signal::{self, Signal},
    unistd::Pid,
};

use tokio::{
    process::{Child, Command},
    sync::RwLock,
    task::JoinSet,
    time::{self, sleep, Instant},
};

#[derive(Debug)]
pub struct SessionNodeRestart {
    max_times: u64,
    delay: Duration,
}

impl SessionNodeRestart {
    pub fn new(max_times: u64, delay: Duration) -> Self {
        Self { max_times, delay }
    }

    pub fn no_restart() -> Self {
        Self {
            max_times: u64::MIN,
            delay: Duration::from_secs(5),
        }
    }

    pub fn max_times(&self) -> u64 {
        self.max_times
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }
}

impl Default for SessionNodeRestart {
    fn default() -> Self {
        Self {
            max_times: u64::MAX,
            delay: Duration::from_secs(5),
        }
    }
}

#[derive(Debug)]
pub enum SessionNodeStopReason {
    Completed(ExitStatus),
    Errored(std::io::Error),
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

pub enum SessionStalledReason {
    RestartedTooManyTimes,
    TerminatedSuccessfully,
    StalledDependency,
    UserRequested,
}

#[derive(Debug)]
pub struct SessionNode {
    stop_signal: Signal,
    restart: SessionNodeRestart,
    cmd: String,
    args: Vec<String>,
    dependencies: Vec<Arc<SessionNode>>,
}

fn assert_send_sync<T: Send + Sync>() {}



impl SessionNode {
    pub fn new(
        cmd: String,
        args: Vec<String>,
        stop_signal: Signal,
        restart: SessionNodeRestart,
        dependencies: Vec<Arc<SessionNode>>,
    ) -> Self {
        Self {
            cmd,
            args,
            restart,
            stop_signal,
            dependencies,
        }
    }

    pub async fn run(runtime_dir: PathBuf, node: Arc<SessionNode>) -> () {
        assert_send_sync::<Arc<SessionNode>>();
    
        let mut restarted: u64 = 0;
    
        loop {
            // wait for dependencies to be up and running
            let dependencies = node
                .dependencies
                .iter()
                .map(|a| {
                    let dep = a.clone();
                    let runtime_dir = runtime_dir.clone();
                    tokio::spawn(async move { Self::wait_for_dependency_satisfied(runtime_dir, dep).await })
                })
                .collect::<JoinSet<_>>().join_all().await;
    
            let mut command = Command::new(node.cmd.as_str());
            command.args(node.args.as_slice());
            match command.spawn() {
                Ok(mut child) => {
                    // here wait for child to exit or for the command to kill the process
                    // in the case user has requested program to exit use wait_for_dependency_stopped
                    // to wait until all dependencies are stopped
                    child.wait().await.unwrap();
                },
                Err(err) => {
                    eprintln!("Error spawning the child process: {}", err);
                },
            }
    
            // node exited (either successfully or with an error)
            // attempt to sleep before restarting it
            restarted += 1;
            if restarted <= node.restart.max_times() {
                sleep(node.restart.delay()).await;
                continue;
            } else {
                // TODO: here return the run result
                break;
            }
        }
    
        //node.poll().await
    }
    
    pub(crate) async fn wait_for_dependency_satisfied(runtime_dir: PathBuf, dependency: Arc<SessionNode>) {
        assert_send_sync::<Arc<SessionNode>>();

        // TODO: wait for the dependency to be present
    }

    pub(crate) async fn wait_for_dependency_stopped(runtime_dir: PathBuf, dependency: Arc<SessionNode>) {
        assert_send_sync::<Arc<SessionNode>>();

        // TODO: wait for the dependency to be stopped in order to exit cleanly
    }

    pub async fn is_running(&self) -> bool {
        /*
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
        */

        todo!()
    }

    pub async fn issue_manual_stop(&mut self) {
        /*
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
        */
        todo!()
    }

    pub async fn poll(&self) -> Option<SessionStalledReason> {
        /*
        let mut stall_reason = None;

        self.status = match &self.status {
            SessionNodeStatus::Ready => {
                // Check for each dependency to be NOT stalled
                let mut stalled = false;
                for dep in self.dependencies.iter() {
                    let mut guard = dep.write().await;

                    if Box::pin(guard.poll()).await.is_some() {
                        stalled = true;
                    }
                }

                // here dependency node is either stalled or running.
                // if it is running it might NOT have completed what this node requires
                // I do not give any fuck (yet?) because programs can wait for what they
                // need, or fail and will be restarted.
                match stalled {
                    true => SessionNodeStatus::Ready,
                    false => match self.command.spawn() {
                        Ok(child) => SessionNodeStatus::Running(Arc::new(RwLock::new(child))),
                        Err(err) => SessionNodeStatus::Stopped {
                            time: time::Instant::now(),
                            reason: Arc::new(SessionNodeStopReason::Errored(err)),
                        },
                    },
                }
            }
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
                    reason: Arc::new(SessionNodeStopReason::Errored(err)),
                },
            },
            SessionNodeStatus::Stopped { time, reason } => {
                stall_reason = match reason.deref() {
                    SessionNodeStopReason::Errored(_) => {
                        match self.restarted >= self.restart.max_times {
                            true => Some(SessionStalledReason::RestartedTooManyTimes),
                            false => None,
                        }
                    }
                    SessionNodeStopReason::Completed(exit_status) => {
                        if exit_status.success() {
                            Some(SessionStalledReason::TerminatedSuccessfully)
                        } else if self.restarted >= self.restart.max_times {
                            Some(SessionStalledReason::RestartedTooManyTimes)
                        } else {
                            None
                        }
                    }
                    SessionNodeStopReason::Manual => Some(SessionStalledReason::UserRequested),
                };

                match time.checked_add(self.restart.delay) {
                    None => self.status.clone(),
                    Some(restart_time) => match Instant::now() >= restart_time {
                        false => self.status.clone(),
                        true => match stall_reason.is_none() {
                            true => SessionNodeStatus::Ready,
                            false => self.status.clone(),
                        },
                    },
                }
            }
        };

        stall_reason
        */
        todo!()
    }
}
