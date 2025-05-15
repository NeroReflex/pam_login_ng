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

use std::{
    io::Error as IOError, ops::Deref, path::PathBuf, process::ExitStatus, sync::Arc,
    time::Duration, u64,
};

use nix::sys::signal::Signal;

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

#[derive(Debug)]
pub enum SessionNodeStopReason {
    Completed(ExitStatus),
    Errored(IOError),
    Manual,
}

#[derive(Debug, Clone)]
pub enum SessionNodeStatus {
    Ready,
    Running,
    Stopped {
        time: time::Instant,
        restart: bool,
        reason: Arc<SessionNodeStopReason>,
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

    pub async fn run(runtime_dir: PathBuf, node: Arc<SessionNode>) {
        assert_send_sync::<Arc<SessionNode>>();

        let name = node.name.clone();

        let mut restarted: u64 = 0;

        loop {
            // wait for dependencies to be up and running or failed for good
            if node
                .dependencies
                .iter()
                .map(|a| {
                    let dep = a.clone();
                    let runtime_dir = runtime_dir.clone();
                    tokio::spawn(async move {
                        Self::wait_for_dependency_satisfied(runtime_dir, dep).await
                    })
                })
                .collect::<JoinSet<_>>()
                .join_all()
                .await
                .iter()
                .any(|dep_res| dep_res.is_err())
            {}

            let mut command = Command::new(node.cmd.as_str());
            command.args(node.args.as_slice());
            restarted += 1;
            let will_restart_if_failed = restarted <= node.restart.max_times();

            match command.spawn() {
                Ok(mut child) => {
                    if let Some(pidfile) = &node.pidfile {
                        match child.id() {
                            Some(id) => {
                                match File::create(pidfile)
                                    .await
                                {
                                    Ok(mut pidfile) => {
                                        match pidfile.write_all(format!("{id}").as_bytes()).await {
                                            Ok(_) => {}
                                            Err(err) => {
                                                eprintln!(
                                                    "Error writing pidfile for {name}: {err}"
                                                );
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Error creating pidfile for {name}: {err}");
                                    }
                                }
                            }
                            None => {
                                eprintln!("Error fetching pid for {name}");
                            }
                        }
                    }

                    // the process is now runnig: update the status and notify waiters
                    *node.status.write().await = SessionNodeStatus::Running;
                    node.status_notify.notify_waiters();

                    // here wait for child to exit or for the command to kill the process
                    // in the case user has requested program to exit use wait_for_dependency_stopped
                    // to wait until all dependencies are stopped
                    tokio::select! {
                        result = child.wait() =>
                            match result {
                                Ok(result) => SessionNodeStatus::Stopped { time: Instant::now(), restart: !result.success() && will_restart_if_failed, reason: Arc::new(SessionNodeStopReason::Completed(result)) },
                                Err(err) => SessionNodeStatus::Stopped { time: Instant::now(), restart: will_restart_if_failed, reason: Arc::new(SessionNodeStopReason::Errored(err)) }
                            },
                        // TODO: here await for the termination signal
                    };

                    if let Some(pidfile) = &node.pidfile {
                        let _ = std::fs::remove_file(pidfile);
                    }

                    // the status has been changed: notify waiters
                    node.status_notify.notify_waiters();
                }
                Err(err) => {
                    eprintln!("Error spawning the child process: {}", err);

                    *node.status.write().await = SessionNodeStatus::Stopped {
                        time: Instant::now(),
                        restart: will_restart_if_failed,
                        reason: Arc::new(SessionNodeStopReason::Errored(err)),
                    };
                    node.status_notify.notify_waiters();
                }
            };

            // node exited (either successfully or with an error)
            // attempt to sleep before restarting it
            if will_restart_if_failed {
                sleep(node.restart.delay()).await;
                continue;
            } else {
                // TODO: here return the run result
                break;
            }
        }
    }

    pub(crate) async fn wait_for_dependency_satisfied(
        runtime_dir: PathBuf,
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
                    SessionNodeStatus::Running => return Ok(()),
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

    pub(crate) async fn wait_for_dependency_stopped(
        runtime_dir: PathBuf,
        dependency: Arc<SessionNode>,
    ) {
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
            SessionNodeStatus::Running => true,
            _ => false,
        }
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

}
