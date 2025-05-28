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
    errno::Errno,
    libc::pid_t,
    sys::signal::{self, Signal},
    unistd::Pid,
};

use thiserror::Error;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    process::Command,
    sync::{Notify, RwLock},
    task::JoinSet,
    time::{self, sleep, Instant},
};

use crate::errors::{NodeDependencyError, NodeDependencyResult};

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

#[derive(Debug, Copy, Clone)]
pub enum SessionNodeStopReason {
    Completed(ExitStatus),
    Errored, /*(IOError)*/
    ManuallyStopped,
    ManuallyRestarted,
}

#[derive(Debug, Clone)]
pub enum SessionNodeStatus {
    Ready,
    Running {
        pid: pid_t,
        pending: Option<ManualAction>,
    },
    Stopped {
        time: time::Instant,
        restart: bool,
        reason: SessionNodeStopReason,
    },
}

pub enum SessionStalledReason {
    RestartedTooManyTimes,
    TerminatedSuccessfully,
    StalledDependency,
    UserRequested,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SessionNodeType {
    OneShot,
    Service,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ManualAction {
    Restart,
    Stop,
}

pub enum RunResult {
    NeverRun,
    Exited(ExitStatus),
    Error,
}

#[derive(Error, Copy, Clone, PartialEq, Debug)]
pub enum ManualActionIssueError {
    #[error("Error performing the requested action: action pending already")]
    AlreadyPendingAction,

    #[error("Error sending the termination signal: {0}")]
    CannotSendSignal(Errno),
}

#[derive(Debug)]
pub struct SessionNode {
    name: String,
    kind: SessionNodeType,
    pidfile: Option<PathBuf>,
    stop_signal: Signal,
    restart: SessionNodeRestart,
    cmd: String,
    args: Vec<String>,
    dependencies: Vec<Arc<SessionNode>>,
    status: Arc<RwLock<SessionNodeStatus>>,
    status_notify: Arc<Notify>,
}

fn assert_send_sync<T: Send + Sync>() {}

impl SessionNode {
    pub fn new(
        name: String,
        kind: SessionNodeType,
        pidfile: Option<PathBuf>,
        cmd: String,
        args: Vec<String>,
        stop_signal: Signal,
        restart: SessionNodeRestart,
        dependencies: Vec<Arc<SessionNode>>,
    ) -> Self {
        let status = Arc::new(RwLock::new(SessionNodeStatus::Ready));
        let status_notify = Arc::new(Notify::new());

        Self {
            name,
            kind,
            pidfile,
            cmd,
            args,
            restart,
            stop_signal,
            dependencies,
            status,
            status_notify,
        }
    }

    pub async fn run(node: Arc<SessionNode>, main: bool) -> RunResult {
        assert_send_sync::<Arc<SessionNode>>();

        // Store environments at the beginning and reuse them later to ensure no bad env is carried over
        let environment = std::env::vars()
            .map(|(key, val)| (key, val))
            .collect::<Vec<_>>();

        let name = node.name.clone();

        let mut restarted: u64 = 0;

        loop {
            restarted += 1;
            let will_restart_if_failed = restarted <= node.restart.max_times();

            // wait for dependencies to be up and running or failed for good
            if node
                .dependencies
                .iter()
                .map(|a| {
                    let dep = a.clone();
                    tokio::spawn(async move { Self::wait_for_dependency_satisfied(dep).await })
                })
                .collect::<JoinSet<_>>()
                .join_all()
                .await
                .iter()
                .any(|dep_res| dep_res.is_err())
            {
                // TODO: what if there is an error?
            }

            // Prepare the command to execute: use the old set of environment variables
            let mut command = Command::new(node.cmd.as_str());
            command.args(node.args.as_slice());
            command.env_clear();
            for (key, val) in environment.iter() {
                command.env(key, val);
            }

            let mut node_status = node.status.write().await;

            let spawn_res = command.spawn();
            let Ok(mut child) = spawn_res else {
                eprintln!(
                    "Error spawning the child process: {}",
                    spawn_res.unwrap_err()
                );

                *node_status = SessionNodeStatus::Stopped {
                    time: Instant::now(),
                    restart: will_restart_if_failed,
                    reason: SessionNodeStopReason::Errored, /*(err)*/
                };
                node.status_notify.notify_waiters();

                continue;
            };

            let Some(pid) = child.id() else {
                // The PID cannot be found: kill the process by its handle
                eprintln!("Error fetching pid for {name}");
                child.kill().await.unwrap();

                *node_status = SessionNodeStatus::Stopped {
                    time: Instant::now(),
                    restart: will_restart_if_failed,
                    reason: SessionNodeStopReason::Errored, /*(err)*/
                };
                node.status_notify.notify_waiters();

                continue;
            };

            if let Some(pidfile) = &node.pidfile {
                match File::create(pidfile).await {
                    Ok(mut pidfile) => match pidfile.write_all(format!("{pid}").as_bytes()).await {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("Error writing pidfile for {name}: {err}");
                        }
                    },
                    Err(err) => {
                        eprintln!("Error creating pidfile for {name}: {err}");
                    }
                }
            }

            // the process is now runnig: update the status and notify waiters
            *node_status = SessionNodeStatus::Running {
                pid: pid.try_into().unwrap(),
                pending: None,
            };
            node.status_notify.notify_waiters();

            // while the process is awaited allows for other parts to get a hold of the status
            // so that a stop or restart command can be issued
            drop(node_status);

            enum ForcedAction {
                ForcefullyRestart,
                ForcefullyStop,
            }

            let mut end_loop_action = None;
            let mut success = false;

            // here wait for child to exit or for the command to kill the process
            // in the case user has requested program to exit use wait_for_dependency_stopped
            // to wait until all dependencies are stopped
            let mut last_exec_result = RunResult::NeverRun;
            tokio::select! {
                result = child.wait() => {
                    last_exec_result = match result {
                        Ok(result) => RunResult::Exited(result),
                        Err(_err) => RunResult::Error,
                    };
                    let mut new_status = node.status.write().await;
                    *new_status = match *(new_status) {
                        SessionNodeStatus::Running { pid: _, pending } => match pending {
                            Some(pending_action) => match pending_action {
                                ManualAction::Restart => {
                                    end_loop_action = Some(ForcedAction::ForcefullyRestart);
                                    SessionNodeStatus::Stopped { time: Instant::now(), restart: will_restart_if_failed, reason: SessionNodeStopReason::Errored /*(err)*/ }
                                },
                                ManualAction::Stop => {
                                    end_loop_action = Some(ForcedAction::ForcefullyStop);
                                    SessionNodeStatus::Stopped { time: Instant::now(), restart: will_restart_if_failed, reason: SessionNodeStopReason::Errored /*(err)*/ }
                                },
                            },
                            None => match &last_exec_result {
                                RunResult::Exited(result) => {
                                    success = result.success();
                                    SessionNodeStatus::Stopped { time: Instant::now(), restart: !result.success() && will_restart_if_failed, reason: SessionNodeStopReason::Completed(*result) }
                                },
                                RunResult::Error => {
                                    SessionNodeStatus::Stopped { time: Instant::now(), restart: will_restart_if_failed, reason: SessionNodeStopReason::Errored /*(err)*/ }
                                },
                                RunResult::NeverRun => unreachable!()
                            }
                        },
                        _ => unreachable!(),
                    }
                },
                // TODO: here await for the termination signal
            };

            if let Some(pidfile) = &node.pidfile {
                let _ = std::fs::remove_file(pidfile);
            }

            // the status has been changed: notify waiters
            node.status_notify.notify_waiters();

            match end_loop_action {
                Some(todo) => match todo {
                    ForcedAction::ForcefullyRestart => {
                        // clear out the restart count to be coherent
                        // with a restarted node that was halted due
                        // to too many restarts.
                        restarted = 0;
                        continue;
                    }
                    ForcedAction::ForcefullyStop => {
                        if main {
                            // TODO: flag the outcome: user has requested the
                            // node to be stopped, and this is the main node
                            // to program must now be closed
                            return Self::terminate_run(node.clone(), last_exec_result).await;
                        }

                        // trap the logic in an endless wait that
                        // can only be escaped by restarting the node
                        // or by the program termination (when main exits)
                        todo!()
                    }
                },
                None => {
                    // node exited (either successfully or with an error)
                    // attempt to sleep before restarting it
                    if will_restart_if_failed && !success {
                        sleep(node.restart.delay()).await;
                        continue;
                    }

                    if main {
                        // if we are here the main node has exited:
                        // it also means the program has to exit
                        // and therefore every service has to be stopped
                        return Self::terminate_run(node.clone(), last_exec_result).await;
                    }

                    // trap the logic in an endless wait that
                    // can only be escaped by restarting the node
                    // or by the program termination (when main exits)
                    todo!()
                }
            }
        }
    }

    async fn terminate_run(node: Arc<SessionNode>, result: RunResult) -> RunResult {
        node.dependencies
            .iter()
            .map(|a| {
                let dep = a.clone();
                tokio::spawn(async move { Self::wait_for_dependency_stopped(dep).await })
            })
            .collect::<JoinSet<_>>()
            .join_all()
            .await;

        result
    }

    pub(crate) async fn wait_for_dependency_satisfied(
        dependency: Arc<SessionNode>,
    ) -> NodeDependencyResult<()> {
        assert_send_sync::<Arc<SessionNode>>();

        loop {
            match dependency.kind {
                SessionNodeType::OneShot => {
                    // TODO: here wait for it to be stopped
                    // return OK(()) on success, Err() otherwise.
                }
                SessionNodeType::Service => match dependency.status.read().await.deref() {
                    SessionNodeStatus::Ready => {}
                    SessionNodeStatus::Running { pid: _, pending: _ } => return Ok(()),
                    SessionNodeStatus::Stopped {
                        time: _,
                        restart,
                        reason: _,
                    } => {
                        if !*restart {
                            return Err(NodeDependencyError::ServiceWontRestart);
                        }
                    }
                },
            }

            // wait for a signal to arrive to re-check or wait the timeout:
            // it is possible to lose a signal of status changed, so it is
            // imperative to query it sporadically
            tokio::select! {
                _ = sleep(Duration::from_millis(250)) => {},
                _ = dependency.status_notify.notified() => {},
            };
        }
    }

    pub(crate) async fn wait_for_dependency_stopped(dependency: Arc<SessionNode>) {
        assert_send_sync::<Arc<SessionNode>>();

        // TODO: wait for the dependency to be stopped in order to exit cleanly
    }

    pub async fn is_running(&self) -> bool {
        /*
        for dep in self.dependencies.iter() {
            let dep_guard = dep.read().await;
            if Box::pin(dep_guard.is_running()).await {
                return false;
            }
        }

        false
        */

        match *self.status.read().await {
            SessionNodeStatus::Running { pid: _, pending: _ } => true,
            _ => false,
        }
    }

    pub async fn issue_manual_action(
        node: Arc<SessionNode>,
        action: ManualAction,
    ) -> Result<(), ManualActionIssueError> {
        let mut status_guard = node.status.write().await;

        match *status_guard {
            SessionNodeStatus::Ready => match &action {
                ManualAction::Restart => todo!(),
                ManualAction::Stop => todo!(),
            },
            SessionNodeStatus::Running { pid, pending } => match pending {
                Some(_) => Err(ManualActionIssueError::AlreadyPendingAction),
                None => {
                    *status_guard = SessionNodeStatus::Running {
                        pid,
                        pending: Some(action),
                    };

                    match signal::kill(Pid::from_raw(pid.try_into().unwrap()), node.stop_signal) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(ManualActionIssueError::CannotSendSignal(err)),
                    }
                }
            },
            SessionNodeStatus::Stopped {
                time,
                restart,
                reason,
            } => todo!(),
        }
    }
}
