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

use sys_mount::{Mount, Unmount, UnmountDrop, UnmountFlags};

use login_ng::mount::MountPoints;

use std::{fs::create_dir, path::Path};

use std::io;

/// Mounts a filesystem at the specified path.
///
/// This function takes a tuple containing information necessary for mounting a filesystem.
/// It checks if the specified mount path exists and is a directory. If the path does not exist,
/// it attempts to create it. Depending on whether the filesystem type is provided, it constructs
/// a mount operation accordingly.
///
/// # Parameters
///
/// - `data`: A tuple of four `String` values:
///   - `data.0`: The filesystem type (e.g., "ext4", "nfs"). If this is an empty string, the mount
///     operation will be performed without specifying a filesystem type.
///   - `data.1`: Additional data required for the mount operation (e.g., options for the mount).
///   - `data.2`: The source of the filesystem to mount (e.g., a device or remote filesystem).
///   - `data.3`: The target directory where the filesystem should be mounted.
///
/// # Returns
///
/// Returns a `Result<Mount, io::Error>`. On success, it returns a `Mount` object representing
/// the mounted filesystem. On failure, it returns an `io::Error` indicating what went wrong,
/// which could include issues with directory creation or mounting the filesystem.
///
/// # Errors
///
/// This function may fail if:
/// - The specified mount path does not exist and cannot be created due to permission issues.
/// - The mount operation fails due to invalid parameters or system errors.
///
/// # Example
///
/// ```rust
/// let mount_data = (String::from("ext4"), String::from("rw"), String::from("/dev/sda1"), String::from("/mnt/my_mount"));
/// match mount(mount_data) {
///     Ok(mount) => println!("Mounted successfully: {:?}", mount),
///     Err(e) => eprintln!("Failed to mount: {}", e),
/// }
/// ```
///
fn mount(data: (String, String, String, String)) -> io::Result<Mount> {
    let mount_path = Path::new(data.3.as_str());
    if !mount_path.exists() || !mount_path.is_dir() {
        // if the path is a file this will fail
        create_dir(mount_path)?;
    }

    match data.0.is_empty() {
        true => Mount::builder().mount(data.2.as_str(), mount_path.as_os_str()),
        false => Mount::builder()
            .fstype(data.0.as_str())
            .data(data.1.as_str())
            .mount(data.2.as_str(), data.3.as_str()),
    }
}

pub(crate) fn mount_all(mounts: MountPoints, username: String, homedir: String) -> Vec<UnmountDrop<Mount>> {
    let mut mounted_devices = vec![];

    for m in mounts
        .foreach(|a, b| {
            (
                b.fstype().clone(),
                b.flags().join(",").clone(),
                b.device().clone(),
                a.clone(),
            )
        })
        .iter()
    {
        match mount(m.clone()) {
            Ok(mount) => {
                println!(
                    "Mounted device {} into {} for user '{username}'",
                    m.2.as_str(),
                    m.3.as_str(),
                );

                // Make the mount temporary, so that it will be unmounted on drop.
                mounted_devices.push(mount.into_unmount_drop(UnmountFlags::DETACH));
            }
            Err(err) => {
                eprintln!(
                    "failed to mount device {} into {}: {}",
                    m.2.as_str(),
                    m.3.as_str(),
                    err
                );

                return vec![];
            }
        }
    }

    match mount((
        mounts.mount().fstype().clone(),
        mounts.mount().flags().join(","),
        mounts.mount().device().clone(),
        homedir,
    )) {
        Ok(mount) => {
            println!(
                "Mounted device {} on home directory for user '{username}'",
                mounts.mount().device().as_str(),
            );

            // Make the mount temporary, so that it will be unmounted on drop.
            mounted_devices.push(mount.into_unmount_drop(UnmountFlags::DETACH));
        }
        Err(err) => {
            eprintln!("failed to mount user directory: {err}");
            return vec![];
        }
    }

    mounted_devices
}
