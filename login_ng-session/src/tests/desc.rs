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

use std::{collections::HashMap, path::PathBuf};

use crate::desc::NodeServiceDescriptor;

#[tokio::test]
async fn test_not_found() {
    let load_path = PathBuf::from("../test_data/test_not_found");
    assert!(load_path.exists());

    let load_directoried = vec![load_path.clone()];

    let default_service_name = String::from("default.service");

    let mut nodes = HashMap::new();
    let load_res = NodeServiceDescriptor::load_tree(
        &mut nodes,
        &default_service_name,
        load_directoried.as_slice(),
    )
    .await
    .unwrap_err();

    match load_res {
        crate::errors::NodeLoadingError::FileNotFound(_) => (),
        _ => assert!(false),
    }
}

#[tokio::test]
async fn test_new() {
    let load_path = PathBuf::from("../test_data/test_cyclic_deps");
    assert!(load_path.exists());

    let load_directoried = vec![load_path.clone()];

    let default_service_name = String::from("default.service");

    let mut nodes = HashMap::new();
    let load_res = NodeServiceDescriptor::load_tree(
        &mut nodes,
        &default_service_name,
        load_directoried.as_slice(),
    )
    .await
    .unwrap_err();

    match load_res {
        crate::errors::NodeLoadingError::CyclicDependency(dep) => {
            assert_eq!(dep, String::from("default.service"))
        }
        crate::errors::NodeLoadingError::IOError(_) => assert_eq!(1, 4),
        crate::errors::NodeLoadingError::FileNotFound(_) => assert_eq!(2, 4),
        crate::errors::NodeLoadingError::JSONError(_) => assert_eq!(3, 4),
    }
}
