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
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use tokio::sync::RwLock;

use nix::sys::signal::Signal;
use serde::{Deserialize, Serialize};

use crate::{
    errors::{NodeLoadingError, NodeLoadingResult},
    node::{SessionNode, SessionNodeRestart},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeServiceDescriptor {
    cmd: String,
    args: Vec<String>,
    max_restarts: u64,
    restart_delay_secs: u64,
    dependencies: Vec<String>,
}

impl NodeServiceDescriptor {
    pub async fn load_tree(
        hashmap: &mut HashMap<String, Arc<SessionNode>>,
        filename: &String,
        directories: &[PathBuf],
    ) -> NodeLoadingResult<()> {
        let mut currently_loading = HashSet::new();

        Self::find_and_load(hashmap, filename, directories, &mut currently_loading).await
    }

    /// Attempts to find and load a session node from a specified file, checking for cyclic dependencies.
    ///
    /// This function searches for a file with the given `filename` in the provided `directories`.
    /// If the file is found, it reads its contents, deserializes it into a `NodeServiceDescriptor`,
    /// and creates a new `SessionNode`. The newly created node is then inserted into the provided
    /// `hashmap`. The function also recursively loads any dependencies specified in the
    /// `NodeServiceDescriptor`.
    ///
    /// # Parameters
    ///
    /// - `hashmap`: A mutable reference to a `HashMap` that stores loaded session nodes. The keys
    ///   are filenames, and the values are `Arc<RwLock<SessionNode>>` instances representing the
    ///   loaded nodes.
    /// - `filename`: A reference to a `String` that specifies the name of the file to load.
    /// - `directories`: A slice of `PathBuf` representing the directories to search for the file.
    /// - `currently_loading`: A mutable reference to a `HashSet<String>` that tracks the filenames
    ///   currently being loaded to detect cyclic dependencies.
    ///
    /// # Returns
    ///
    /// This function returns a `NodeLoadingResult<()>`. On success, it returns `Ok(())`. If an error
    /// occurs, it returns a `NodeLoadingError` variant, which can indicate issues such as:
    /// - `CyclicDependency`: If a cyclic dependency is detected during loading.
    /// - `FileNotFound`: If the specified file cannot be found in the provided directories.
    /// - `IOError`: If an I/O error occurs while opening or reading the file.
    /// - `JSONError`: If the file contents cannot be deserialized into a `NodeServiceDescriptor`.
    ///
    /// # Safety
    ///
    /// This function is not `unsafe`, but care should be taken to ensure that the `currently_loading`
    /// set is properly managed to avoid memory leaks or deadlocks in a multi-threaded context.
    async fn find_and_load(
        hashmap: &mut HashMap<String, Arc<SessionNode>>,
        filename: &String,
        directories: &[PathBuf],
        currently_loading: &mut HashSet<String>,
    ) -> NodeLoadingResult<()> {
        // Check for cyclic dependency
        if currently_loading.contains(filename) {
            return Err(NodeLoadingError::CyclicDependency(filename.clone()));
        }

        // Add the current filename to the loading set
        currently_loading.insert(filename.clone());

        // Check if the file is already loaded
        if hashmap.contains_key(filename) {
            // Remove from loading set before returning
            currently_loading.remove(filename);
            return Err(NodeLoadingError::CyclicDependency(filename.clone()));
        }

        let mut chosen = None;

        for dir in directories.iter() {
            let file = dir.join(filename);
            if file.exists() {
                chosen = Some(file);
            }
        }

        let value = match chosen {
            Some(filepath) => {
                let mut file = File::open(filepath).map_err(NodeLoadingError::IOError)?;
                let mut value = String::new();
                file.read_to_string(&mut value)
                    .map_err(NodeLoadingError::IOError)?;
                value
            }
            None => {
                currently_loading.remove(filename); // Clean up before returning
                return Err(NodeLoadingError::FileNotFound(filename.clone()));
            }
        };

        let main = serde_json::from_str::<NodeServiceDescriptor>(value.as_str())
            .map_err(NodeLoadingError::JSONError)?;

        let mut dependencies = vec![];

        // Parse all dependencies and then register those as part of node
        for dep in main.dependencies().iter() {
            Box::pin(Self::find_and_load(
                hashmap,
                dep,
                directories,
                currently_loading,
            ))
            .await?;

            let just_loaded = hashmap.get(dep).unwrap();
            dependencies.push(just_loaded.clone());
        }

        let node = SessionNode::new(
            main.cmd(),
            main.args(),
            Signal::SIGABRT,
            SessionNodeRestart::new(main.max_restarts(), main.delay()),
            dependencies,
        );

        hashmap.insert(filename.clone(), Arc::new(node));

        // Remove the filename from the loading set after processing
        currently_loading.remove(filename);

        Ok(())
    }

    pub fn cmd(&self) -> String {
        self.cmd.clone()
    }

    pub fn args(&self) -> Vec<String> {
        self.args.clone()
    }

    pub fn max_restarts(&self) -> u64 {
        self.max_restarts
    }

    pub fn delay(&self) -> Duration {
        Duration::from_secs(self.restart_delay_secs)
    }

    pub fn dependencies(&self) -> &[String] {
        self.dependencies.as_slice()
    }
}
