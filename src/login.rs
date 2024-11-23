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

use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoginResult {
    Success,
    Failure,
}

#[derive(Debug, Error)]
pub enum LoginError {
    #[cfg(feature = "greetd")]
    #[error("Error with greetd: {0}")]
    GreetdError(#[from] crate::greetd::GreetdLoginError),

    #[error("Error with pam: {0}")]
    PamError(#[from] crate::pam::PamLoginError),

    #[error("Username not recognised")]
    UserDiscoveryError,
}

pub trait LoginUserInteractionHandler {
    fn provide_username(&mut self, username: &String);

    fn prompt_secret(&mut self, msg: &String) -> Option<String>;

    fn prompt_plain(&mut self, msg: &String) -> Option<String>;

    fn print_info(&mut self, msg: &String);

    fn print_error(&mut self, msg: &String);
}

/// Interface that allows a user to authenticate and perform actions
pub trait LoginExecutor {
    fn prompt(&self) -> Arc<Mutex<dyn LoginUserInteractionHandler>>;

    /// Authenticate the user and execute the given command, or launch shell if one is not being provided.
    fn execute(
        &mut self,
        maybe_username: &Option<String>,
        cmd: &Option<String>,
    ) -> Result<LoginResult, LoginError>;
}
