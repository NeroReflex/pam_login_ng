use std::{ffi::OsString, path::{Path, PathBuf}};

use crate::user::UserAuthData;

use errors::ByteVecError;
use thiserror::Error;
use users::{get_user_by_name, os::unix::UserExt};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Username not recognised")]
    UserDiscoveryError,

    #[error("Home directory not found")]
    HomeDirNotFound(OsString),

    #[error("Error with xattrs: {0}")]
    XAttrError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] ByteVecError),
}

/// Represents a source of user authentication data
pub enum StorageSource {
    /// Load/Store operations will be performed on the autodetected home directory
    Username(String),

    /// Load/Store operations will be performed on the given path
    Path(PathBuf)
}

use bytevec::*;

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    struct AuthDatamanifest {
        version: u32
    }
}

impl AuthDatamanifest {
    fn new() -> Self {
        Self {
            version: 0
        }
    }
}

fn homedir_by_username(username: &String) -> Result<OsString, StorageError> {
    let user = get_user_by_name(&username).ok_or(StorageError::UserDiscoveryError)?;

    let systemd_homed_str: OsString = format!("/home/{}.homedir", username).into();
    let systemd_homed_path = Path::new(systemd_homed_str.as_os_str());

    let home_dir_path = match systemd_homed_path.exists() {
        true => systemd_homed_str,
        false => user.home_dir().as_os_str().into()
    };

    match Path::new(home_dir_path.as_os_str()).exists() {
        true => Ok(home_dir_path),
        false => Err(StorageError::HomeDirNotFound(home_dir_path))
    }
}

pub fn load_user_auth_data(source: &StorageSource) -> Result<UserAuthData, StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string()
    };

    let xattrs = xattr::list_deref(home_dir_path.as_os_str()).map_err(|err| StorageError::XAttrError(err))?;
    for attr in xattrs.into_iter() {
        println!(" - {:?}", attr);
    }

    todo!()
}

pub fn remove_user_auth_data(source: &StorageSource) -> Result<(), StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string()
    };

    let xattrs = xattr::list_deref(home_dir_path.as_os_str()).map_err(|err| StorageError::XAttrError(err))?;
    for attr in xattrs.into_iter() {
        if attr.to_string_lossy().starts_with(crate::DEFAULT_XATTR_NAME) {
            xattr::remove_deref(home_dir_path.as_os_str(), crate::DEFAULT_XATTR_NAME).map_err(|err| StorageError::XAttrError(err))?
        }
    }

    Ok(())
}

pub fn save_user_auth_data(auth_data: UserAuthData, source: &StorageSource) -> Result<(), StorageError> {
    let home_dir_path = match source {
        StorageSource::Username(username) => homedir_by_username(&username)?,
        StorageSource::Path(pathbuf) => pathbuf.as_os_str().to_os_string()
    };

    // this is used in case a future format will be required
    let manifest = AuthDatamanifest::new();
    let manifest_serialization = manifest.encode::<u8>().map_err(|err| StorageError::SerializationError(err))?;
    
    let maybe_main_password_serialization = match auth_data.main_password() {
        Some(m) => Some(m.encode::<u8>().map_err(|err| StorageError::SerializationError(err))?),
        None => None
    };

    // once everything is serialized perform the writing
    xattr::set(
        home_dir_path.as_os_str(), format!("{}.manifest", crate::DEFAULT_XATTR_NAME), manifest_serialization.as_slice()
    ).map_err(|err| StorageError::XAttrError(err))?;

    Ok(
        match &maybe_main_password_serialization {
            Some(data) => {
                todo!();

                // save the main password first so that if something bad happens before one or more secondary auth may be usable
                xattr::set(
                    home_dir_path.as_os_str(), format!("{}.main", crate::DEFAULT_XATTR_NAME), data.as_slice()
                ).map_err(|err| StorageError::XAttrError(err))?;
            },
            None => {}
        }
    )
}