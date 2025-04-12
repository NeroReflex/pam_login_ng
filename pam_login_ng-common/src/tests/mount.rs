/*
    login-ng A greeter written in rust that also supports autologin with systemd-homed
    Copyright (C) 2024  Denis Benato

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

use crate::mount::{MountAuthDBus, MountAuthOperations};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const AUTHORIZATION_TESTFILE: &str = "auth_test.json";

#[tokio::test]
async fn test_new() {
    let filepath = Path::new("./").join(AUTHORIZATION_TESTFILE);

    if std::fs::exists(filepath.clone()).unwrap() {
        std::fs::remove_file(filepath.clone()).unwrap();
    }

    let mounts_auth_op = Arc::new(RwLock::new(MountAuthOperations::new(filepath.clone())));

    let mounts_auth = MountAuthDBus::new(mounts_auth_op.clone());

    assert_eq!(mounts_auth.check("username", 0x4E421u64).await, false);
}

#[tokio::test]
async fn test_authorize() {
    let filepath = Path::new("./").join(AUTHORIZATION_TESTFILE);

    if std::fs::exists(filepath.clone()).unwrap() {
        std::fs::remove_file(filepath.clone()).unwrap();
    }

    let mounts_auth_op = Arc::new(RwLock::new(MountAuthOperations::new(filepath.clone())));

    let mut mounts_auth = MountAuthDBus::new(mounts_auth_op.clone());

    assert_eq!(mounts_auth.check("username", 0x4E421u64).await, false);
    assert_eq!(mounts_auth.authorize("username", 0x4E421u64).await, 0u32);
    assert_eq!(mounts_auth.check("username", 0x4E421u64).await, true);
}

#[tokio::test]
async fn test_authorize_different_users() {
    let filepath = Path::new("./").join(AUTHORIZATION_TESTFILE);

    if std::fs::exists(filepath.clone()).unwrap() {
        std::fs::remove_file(filepath.clone()).unwrap();
    }

    let mounts_auth_op = Arc::new(RwLock::new(MountAuthOperations::new(filepath.clone())));

    let mut mounts_auth = MountAuthDBus::new(mounts_auth_op.clone());

    const NUM1: u64 = 0x2913787u64;
    const NUM2: u64 = 0x4E42142u64;

    assert_eq!(mounts_auth.check("username", NUM1).await, false);
    assert_eq!(mounts_auth.check("test", NUM2).await, false);
    assert_eq!(mounts_auth.authorize("test", NUM2).await, 0u32);
    assert_eq!(mounts_auth.authorize("username", NUM1).await, 0u32);
    assert_eq!(mounts_auth.check("username", NUM1).await, true);
    assert_eq!(mounts_auth.check("test", NUM2).await, true);
    assert_eq!(mounts_auth.check("test", NUM1).await, false);
    assert_eq!(mounts_auth.check("username", NUM2).await, false);
}
