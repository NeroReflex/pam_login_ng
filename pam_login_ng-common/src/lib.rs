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

pub extern crate login_ng;
pub extern crate rand;
pub extern crate rsa;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate zbus;

#[cfg(test)]
pub(crate) mod tests;

pub mod disk;
pub mod mount;
pub mod result;
pub mod security;
pub mod service;
pub mod session;

pub const XDG_RUNTIME_DIR_PATH: &str = "/tmp/xdg/";
