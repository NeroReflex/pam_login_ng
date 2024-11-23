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

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    auth::{SecondaryAuth, SecondaryAuthMethod, SecondaryPassword},
    user::{MainPassword, UserAuthData},
};

use bytevec2::errors;
use errors::ByteVecError;
use thiserror::Error;
use users::{get_user_by_name, os::unix::UserExt};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Uhandled data version")]
    UnhandledVersion,

    #[error("Username not recognised")]
    UserDiscoveryError,

    #[error("Home directory not found")]
    HomeDirNotFound(OsString),

    #[error("Error with xattrs: {0}")]
    XAttrError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] ByteVecError),

    #[error("Deserialization error")]
    DeserializationError,
}

/// Represents a source of user authentication data
pub enum StorageSource {
    /// Load/Store operations will be performed on the autodetected home directory
    Username(String),

    /// Load/Store operations will be performed on the given path
    Path(PathBuf),
}

use bytevec2::*;

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    struct AuthDataManifest {
        version: u32
    }
}

impl AuthDataManifest {
    fn new() -> Self {
        Self { version: 0 }
    }
}

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Clone)]
    struct AuthDataSerialized {
        name: String,
        creation_date: u64,
        auth_type: u32,
        auth_data: Vec<u8>
    }
}

impl TryFrom<&SecondaryAuth> for AuthDataSerialized {
    type Error = StorageError;

    fn try_from(value: &SecondaryAuth) -> Result<Self, Self::Error> {
        let name = String::from(value.name());
        let creation_date = value.creation_date();

        let (auth_type, auth_data) = match value.data() {
            SecondaryAuthMethod::Password(secondary_password) => (
                0,
                secondary_password
                    .encode::<u16>()
                    .map_err(|err| Self::Error::SerializationError(err))?,
            ),
        };

        Ok(Self {
            name,
            creation_date,
            auth_data,
            auth_type,
        })
    }
}

impl TryInto<SecondaryAuth> for AuthDataSerialized {
    type Error = StorageError;

    fn try_into(self) -> Result<SecondaryAuth, Self::Error> {
        match self.auth_type {
            0 => Ok(SecondaryAuth::new_password(
                self.name.as_str(),
                Some(self.creation_date),
                SecondaryPassword::decode::<u16>(self.auth_data.as_slice())
                    .map_err(|err| StorageError::SerializationError(err))?,
            )),
            _ => Err(StorageError::DeserializationError),
        }
    }
}

fn homedir_by_username(username: &String) -> Result<OsString, StorageError> {
    let user = get_user_by_name(&username).ok_or(StorageError::UserDiscoveryError)?;

    let systemd_homed_str: OsString = format!("/home/{}.homedir", username).into();
    let systemd_homed_path = Path::new(systemd_homed_str.as_os_str());

    let home_dir_path = match systemd_homed_path.exists() {
        true => systemd_homed_str,
        false => user.home_dir().as_os_str().into(),
    };

    match Path::new(home_dir_path.as_os_str()).exists() {
        true => Ok(home_dir_path),
        false => Err(StorageError::HomeDirNotFound(home_dir_path)),
    }
}

pub fn load_user_auth_data(source: &StorageSource) -> Result<Option<UserAuthData>, StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string(),
    };

    let manifest = xattr::get_deref(
        home_dir_path.as_os_str(),
        format!("{}.manifest", crate::DEFAULT_XATTR_NAME),
    )
    .map_err(|err| StorageError::XAttrError(err))?;
    if let None = manifest {
        return Ok(None);
    }

    let main = xattr::get_deref(
        home_dir_path.as_os_str(),
        format!("{}.main", crate::DEFAULT_XATTR_NAME),
    )
    .map_err(|err| StorageError::XAttrError(err))?;
    if let None = main {
        return Ok(None);
    }

    let mut auth_data = UserAuthData::new();

    match main {
        Some(a) => {
            let main = MainPassword::decode::<u16>(a.as_slice())
                .map_err(|err| StorageError::SerializationError(err))?;
            auth_data.push_main(main);
        }
        None => return Ok(None),
    };

    let xattrs = xattr::list_deref(home_dir_path.as_os_str())
        .map_err(|err| StorageError::XAttrError(err))?;
    for attr in xattrs.into_iter() {
        match attr.to_str() {
            Some(s) => {
                if s.starts_with(format!("{}.auth.", crate::DEFAULT_XATTR_NAME).as_str()) {
                    let raw_data = xattr::get_deref(home_dir_path.as_os_str(), s)
                        .map_err(|err| StorageError::XAttrError(err))?
                        .unwrap();
                    let serialized_data = AuthDataSerialized::decode::<u32>(raw_data.as_slice())?;

                    let secondary_auth: SecondaryAuth = serialized_data.try_into()?;

                    auth_data.push_secondary(secondary_auth);
                }
            }
            None => {}
        }
    }

    Ok(Some(auth_data))
}

pub fn remove_user_auth_data(source: &StorageSource) -> Result<(), StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string(),
    };

    let xattrs = xattr::list_deref(home_dir_path.as_os_str())
        .map_err(|err| StorageError::XAttrError(err))?;
    for attr in xattrs.into_iter() {
        if attr
            .to_string_lossy()
            .starts_with(crate::DEFAULT_XATTR_NAME)
        {
            xattr::remove_deref(home_dir_path.as_os_str(), crate::DEFAULT_XATTR_NAME)
                .map_err(|err| StorageError::XAttrError(err))?
        }
    }

    Ok(())
}

pub fn save_user_auth_data(
    auth_data: UserAuthData,
    source: &StorageSource,
) -> Result<(), StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string(),
    };

    // this is used in case a future format will be required
    let manifest = AuthDataManifest::new();
    let manifest_serialization = manifest
        .encode::<u16>()
        .map_err(|err| StorageError::SerializationError(err))?;

    let maybe_main_password_serialization = match auth_data.main_password() {
        Some(m) => Some(
            m.encode::<u16>()
                .map_err(|err| StorageError::SerializationError(err))?,
        ),
        None => None,
    };

    // remove everything that was already present
    remove_user_auth_data(source)?;

    // once everything is serialized perform the writing
    xattr::set(
        home_dir_path.as_os_str(),
        format!("{}.manifest", crate::DEFAULT_XATTR_NAME),
        manifest_serialization.as_slice(),
    )
    .map_err(|err| StorageError::XAttrError(err))?;

    Ok(match &maybe_main_password_serialization {
        Some(data) => {
            // save the main password first so that if something bad happens after one or more secondary auth may be usable
            xattr::set(
                home_dir_path.as_os_str(),
                format!("{}.main", crate::DEFAULT_XATTR_NAME),
                data.as_slice(),
            )
            .map_err(|err| StorageError::XAttrError(err))?;

            for (index, val) in auth_data.secondary().enumerate() {
                let serialized_data: AuthDataSerialized = val.try_into()?;
                let raw_data = serialized_data
                    .encode::<u32>()
                    .map_err(|err| StorageError::SerializationError(err))?;

                xattr::set(
                    home_dir_path.as_os_str(),
                    format!("{}.auth.{}", crate::DEFAULT_XATTR_NAME, index),
                    raw_data.as_slice(),
                )
                .map_err(|err| StorageError::XAttrError(err))?
            }
        }
        None => {}
    })
}
