/*
    pam_polyauth: A pam module written in rust that supports multiple
    authentication modes (including autologin).

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

use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    auth::{SecondaryAuth, SecondaryAuthMethod, SecondaryPassword},
    command::SessionCommand,
    mount::{MountParams, MountPoints},
    user::{MainPassword, UserAuthData},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytevec2::{ByteDecodable, ByteEncodable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Uhandled data version")]
    UnhandledVersion,

    #[error("Username not recognised")]
    UserDiscoveryError,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Bytevec serialization error: {0}")]
    SerializationError(#[from] bytevec2::errors::ByteVecError),

    #[error("Deserialization error")]
    DeserializationError,
}

/// Represents a source of user authentication data
pub enum StorageSource {
    /// Load/Store operations will be performed using /etc/polyauth/{username}.json
    Username(String),

    /// Load/Store operations will be performed on the given file path (JSON format)
    File(PathBuf),
}

const POLYAUTH_CONFIG_DIR: &str = "/etc/polyauth";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserConfig {
    version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_command: Option<SessionCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth_data: Option<AuthDataSerialized>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mountpoints: Option<MountPointsConfig>,
}

impl UserConfig {
    fn new() -> Self {
        Self {
            version: 0,
            session_command: None,
            auth_data: None,
            mountpoints: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MountPointsConfig {
    home: MountPointSerialized,
    #[serde(default)]
    additional: Vec<MountPointSerialized>,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
struct MountPointSerialized {
    fstype: String,
    device: String,
    directory: String,
    args: Vec<String>,
}

impl From<(&String, &MountParams)> for MountPointSerialized {
    fn from(mount_param: (&String, &MountParams)) -> Self {
        Self {
            directory: mount_param.0.clone(),
            fstype: mount_param.1.fstype().clone(),
            device: mount_param.1.device().clone(),
            args: mount_param.1.flags().clone(),
        }
    }
}

impl From<&MountPointSerialized> for (String, MountParams) {
    fn from(serialized: &MountPointSerialized) -> Self {
        (
            serialized.directory.clone(),
            MountParams::new(
                serialized.device.clone(),
                serialized.fstype.clone(),
                serialized.args.clone(),
            ),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthDataSerialized {
    #[serde(skip_serializing_if = "Option::is_none")]
    main: Option<String>, // base64-encoded MainPassword
    #[serde(default)]
    secondary: Vec<SecondaryAuthItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecondaryAuthItem {
    name: String,
    creation_date: u64,
    auth_type: u32,
    password: String, // base64-encoded SecondaryPassword
}

// Helper functions for config file paths
fn config_path_for_username(username: &str) -> PathBuf {
    PathBuf::from(POLYAUTH_CONFIG_DIR).join(format!("{}.json", username))
}

fn config_path_from_source(source: &StorageSource) -> PathBuf {
    match source {
        StorageSource::Username(username) => config_path_for_username(username),
        StorageSource::File(path) => path.clone(),
    }
}

fn load_config_from_source(source: &StorageSource) -> Result<Option<UserConfig>, StorageError> {
    let config_path = config_path_from_source(source);

    if !config_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&config_path)?;
    let config: UserConfig = serde_json::from_str(&contents)?;
    Ok(Some(config))
}

fn save_config_to_source(source: &StorageSource, config: &UserConfig) -> Result<(), StorageError> {
    let config_path = config_path_from_source(source);

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, contents)?;
    Ok(())
}

pub fn load_user_session_command(
    source: &StorageSource,
) -> Result<Option<SessionCommand>, StorageError> {
    let config = load_config_from_source(source)?;
    Ok(config.and_then(|c| c.session_command))
}

pub fn store_user_session_command(
    settings: &SessionCommand,
    source: &StorageSource,
) -> Result<(), StorageError> {
    let mut config = load_config_from_source(source)?.unwrap_or_else(UserConfig::new);
    config.session_command = Some(settings.clone());
    save_config_to_source(source, &config)?;
    Ok(())
}

pub fn load_user_auth_data(source: &StorageSource) -> Result<Option<UserAuthData>, StorageError> {
    let config = load_config_from_source(source)?;

    let Some(config) = config else {
        return Ok(None);
    };

    let Some(auth_data_ser) = config.auth_data else {
        return Ok(None);
    };

    let mut auth_data = UserAuthData::new();

    // Deserialize main password from base64
    if let Some(main_b64) = auth_data_ser.main {
        let main_bytes = BASE64
            .decode(&main_b64)
            .map_err(|_| StorageError::DeserializationError)?;
        let main = MainPassword::decode::<u16>(&main_bytes)?;
        auth_data.push_main(main);
    } else {
        return Ok(None);
    }

    // Deserialize secondary auth
    for item in auth_data_ser.secondary {
        let password_bytes = BASE64
            .decode(&item.password)
            .map_err(|_| StorageError::DeserializationError)?;
        let password = SecondaryPassword::decode::<u16>(&password_bytes)?;

        match item.auth_type {
            0 => {
                let secondary_auth =
                    SecondaryAuth::new_password(&item.name, Some(item.creation_date), password);
                auth_data.push_secondary(secondary_auth);
            }
            _ => return Err(StorageError::DeserializationError),
        }
    }

    Ok(Some(auth_data))
}

pub fn remove_user_data(source: &StorageSource) -> Result<(), StorageError> {
    let config_path = config_path_from_source(source);

    if config_path.exists() {
        fs::remove_file(config_path)?;
    }

    Ok(())
}

pub fn store_user_auth_data(
    auth_data: UserAuthData,
    source: &StorageSource,
) -> Result<(), StorageError> {
    let mut config = load_config_from_source(source)?.unwrap_or_else(UserConfig::new);

    // Serialize main password to base64
    let main_b64 = match auth_data.main_password() {
        Some(m) => {
            let main_bytes = m.encode::<u16>()?;
            Some(BASE64.encode(&main_bytes))
        }
        None => None,
    };

    // Serialize secondary auth
    let mut secondary = Vec::new();
    for val in auth_data.secondary() {
        let name = val.name();
        let creation_date = val.creation_date();

        let (auth_type, password_b64) = match val.data() {
            SecondaryAuthMethod::Password(secondary_password) => {
                let password_bytes = secondary_password.encode::<u16>()?;
                (0, BASE64.encode(&password_bytes))
            }
        };

        secondary.push(SecondaryAuthItem {
            name,
            creation_date,
            auth_type,
            password: password_b64,
        });
    }

    config.auth_data = Some(AuthDataSerialized {
        main: main_b64,
        secondary,
    });

    save_config_to_source(source, &config)?;
    Ok(())
}

pub fn load_user_mountpoints(source: &StorageSource) -> Result<Option<MountPoints>, StorageError> {
    let config = load_config_from_source(source)?;

    let Some(config) = config else {
        return Ok(None);
    };

    let Some(mountpoints_cfg) = config.mountpoints else {
        return Ok(None);
    };

    // Convert serialized home mount to MountParams
    let home_mount = MountParams::new(
        mountpoints_cfg.home.device,
        mountpoints_cfg.home.fstype,
        mountpoints_cfg.home.args,
    );

    // Convert additional mounts
    let mut mounts = HashMap::new();
    for mount_ser in mountpoints_cfg.additional {
        let mount_params = MountParams::new(mount_ser.device, mount_ser.fstype, mount_ser.args);
        mounts.insert(mount_ser.directory, mount_params);
    }

    Ok(Some(MountPoints::new(home_mount, mounts)))
}

pub fn store_user_mountpoints(
    mountpoints_data: Option<MountPoints>,
    source: &StorageSource,
) -> Result<(), StorageError> {
    let mut config = load_config_from_source(source)?.unwrap_or_else(UserConfig::new);

    let Some(mountpoints) = mountpoints_data else {
        config.mountpoints = None;
        save_config_to_source(source, &config)?;
        return Ok(());
    };

    // Serialize home mount
    let home = MountPointSerialized {
        fstype: mountpoints.mount().fstype().clone(),
        device: mountpoints.mount().device().clone(),
        directory: String::new(),
        args: mountpoints.mount().flags().clone(),
    };

    // Serialize additional mounts
    let additional = mountpoints.foreach(|dir, params| MountPointSerialized {
        fstype: params.fstype().clone(),
        device: params.device().clone(),
        directory: dir.clone(),
        args: params.flags().clone(),
    });

    config.mountpoints = Some(MountPointsConfig { home, additional });
    save_config_to_source(source, &config)?;
    Ok(())
}
