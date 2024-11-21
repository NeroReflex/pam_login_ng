use std::{ffi::OsString, path::{Path, PathBuf}};

use crate::{auth::{SecondaryAuth, SecondaryPassword}, user::{MainPassword, UserAuthData}};

use errors::ByteVecError;
use thiserror::Error;
use users::{get_user_by_name, os::unix::UserExt};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Uhandled data version")]
    UnhandledVersion,

    #[error("No data has been find")]
    NoData,

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

    let manifest = xattr::get_deref(home_dir_path.as_os_str(), format!("{}.manifest", crate::DEFAULT_XATTR_NAME)).map_err(|err| StorageError::XAttrError(err))?;
    if let None = manifest {
        return Err(StorageError::NoData)
    }

    let main = xattr::get_deref(home_dir_path.as_os_str(), format!("{}.main", crate::DEFAULT_XATTR_NAME)).map_err(|err| StorageError::XAttrError(err))?;
    if let None = main {
        return Err(StorageError::NoData)
    }

    let mut auth_data = UserAuthData::new();

    match main {
        Some(a) => {
            let main = MainPassword::decode::<u16>(a.as_slice()).map_err(|err| StorageError::SerializationError(err))?;
            auth_data.push_main(main);
        },
        None => return Err(StorageError::NoData)
    };

    let xattrs = xattr::list_deref(home_dir_path.as_os_str()).map_err(|err| StorageError::XAttrError(err))?;
    for attr in xattrs.into_iter() {
        match attr.to_str() {
            Some(s) => {
                if s.starts_with(format!("{}.auth.", crate::DEFAULT_XATTR_NAME).as_str()) {
                    let parts: Vec<&str> = s.split('.').collect();
                    if parts.len() >= 2 {
                        let index_and_type = parts[parts.len() - 1];
                        let index_parts: Vec<&str> = index_and_type.split('_').collect();
                        let t = index_parts[1].to_string();
                        if index_parts.len() == 2 {
                            match index_parts[0].parse::<usize>() {
                                Ok(idx) => {
                                    match xattr::get_deref(home_dir_path.as_os_str(), s).map_err(|err| StorageError::XAttrError(err))? {
                                        Some(raw_data) => {
                                            if t == "password" {
                                                auth_data.push_secondary(
                                                    SecondaryAuth::Password(
                                                        SecondaryPassword::decode::<u16>(&raw_data)
                                                            .map_err(|err| StorageError::SerializationError(err))?
                                                    )
                                                );
                                            } else {

                                            }
                                        },
                                        None => {}
                                    }
                                },
                                Err(_) => {}
                            };
                        }
                    }
                }
            },
            None => {}
        }
    }

    Ok(auth_data)
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
        Some(m) => Some(m.encode::<u16>().map_err(|err| StorageError::SerializationError(err))?),
        None => None
    };

    // remove everything that was already present
    remove_user_auth_data(source)?;

    // once everything is serialized perform the writing
    xattr::set(
        home_dir_path.as_os_str(), format!("{}.manifest", crate::DEFAULT_XATTR_NAME), manifest_serialization.as_slice()
    ).map_err(|err| StorageError::XAttrError(err))?;

    Ok(
        match &maybe_main_password_serialization {
            Some(data) => {
                // save the main password first so that if something bad happens after one or more secondary auth may be usable
                xattr::set(
                    home_dir_path.as_os_str(), format!("{}.main", crate::DEFAULT_XATTR_NAME), data.as_slice()
                ).map_err(|err| StorageError::XAttrError(err))?;

                let staged_attrs = auth_data.secondary().map(|val| {
                    match val {
                        SecondaryAuth::Password(pw) => {
                            (format!("password"), pw.encode::<u16>().map_err(|err| StorageError::SerializationError(err)))
                        }
                    }
                }).collect::<Vec<(String, Result<Vec<u8>, StorageError>)>>();

                let mut index: usize = 0;
                for attr in staged_attrs {
                    let (t, b) = attr;
                    match b {
                        Ok(d) => xattr::set(
                            home_dir_path.as_os_str(), format!("{}.auth.{}_{}", crate::DEFAULT_XATTR_NAME, index, t), d.as_slice()
                        ).map_err(|err| StorageError::XAttrError(err))?,
                        Err(err) => return Err(err)
                    };

                    index += 1;
                }
            },
            None => {}
        }
    )
}