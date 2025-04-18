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

use serde_json::error::Error as JSONError;
use std::io::Error as IOError;
use thiserror::Error;
use zbus::Error as ZError;

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("DBus error: {0}")]
    ZbusError(#[from] ZError),

    #[error("Service name not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Error)]
pub enum NodeLoadingError {
    #[error("I/O error: {0}")]
    IOError(#[from] IOError),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Cyclic dependency found: {0}")]
    CyclicDependency(String),

    #[error("JSON error: {0}")]
    JSONError(#[from] JSONError),

    #[error("Invalid service kind: {0}")]
    InvalidKind(String),
}

pub type NodeLoadingResult<T> = Result<T, NodeLoadingError>;

#[derive(Debug, Error)]
pub(crate) enum NodeDependencyError {
    #[error("I/O error: {0}")]
    IOError(#[from] IOError),


}

pub(crate) type NodeDependencyResult<T> = Result<T, NodeLoadingError>;