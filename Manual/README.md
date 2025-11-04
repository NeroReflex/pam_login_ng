# polyauthctl Manual

## Overview

**polyauthctl** is a command-line tool for managing polyauth authentication methods. It provides a comprehensive interface for setting up multi-factor authentication, managing user mounts, and configuring user sessions.

## Table of Contents

1. [Installation](#installation)
2. [Getting Started](#getting-started)
3. [Commands](#commands)
   - [info](#info)
   - [setup](#setup)
   - [reset](#reset)
   - [inspect](#inspect)
   - [add](#add)
   - [set-session](#set-session)
   - [set-home-mount](#set-home-mount)
   - [set-pre-mount](#set-pre-mount)
   - [mount](#mount)
4. [Global Options](#global-options)
5. [Examples](#examples)
6. [Configuration Files](#configuration-files)
7. [Security Considerations](#security-considerations)
8. [Troubleshooting](#troubleshooting)

## Installation

The `polyauthctl` binary is typically installed to `/usr/bin/polyauthctl` as part of the pam_polyauth package.

```bash
# Check if polyauthctl is installed
which polyauthctl

# Check version information
polyauthctl info
```

### Shell Completion

Tab-completion is available for both bash and zsh. When installed via package manager, completions are automatically set up. For manual installation, see `completions/README.md`.

**Quick test:**
```bash
# Type this and press TAB
polyauthctl <TAB>

# Should show available commands
```

## Getting Started

Before using polyauthctl, you need to initialize authentication data for your user:

```bash
# Initialize authentication with an intermediate key
polyauthctl setup
```

This will prompt you for:
1. An **intermediate key** - used to unlock additional authentication methods
2. Your **main password** - the primary password accepted by PAM

## Commands

### info

Display version and copyright information about polyauthctl.

```bash
polyauthctl info
```

**Output:**
```
pam_polyauth version X.X.X, Copyright (C) 2024-2025 Denis Benato
pam_polyauth comes with ABSOLUTELY NO WARRANTY;
This is free software, and you are welcome to redistribute it
under certain conditions.
```

### setup

Initialize authentication data and create a new intermediate key.

```bash
polyauthctl setup [OPTIONS]
```

**Options:**
- `-i, --intermediate <KEY>` - Provide the intermediate key directly (optional)

**Interactive Mode:**
If you don't provide the intermediate key via command line, you'll be prompted:

```bash
polyauthctl setup
# You will be prompted for:
# - intermediate key:
# - intermediate key (confirm):
# - main password:
```

**Example:**
```bash
# Setup with prompted input
polyauthctl setup

# Setup with pre-defined intermediate key
polyauthctl setup -i "my-secure-intermediate-key"
```

**Notes:**
- This command will fail if an intermediate key already exists
- Use `reset` to remove existing authentication data before running setup again

### reset

Remove all additional authentication data and destroy the intermediate key.

```bash
polyauthctl reset
```

**Warning:** This is a destructive operation that will:
- Remove all secondary authentication methods
- Delete the intermediate key
- Clear mount configurations
- Require you to run `setup` again to re-enable polyauth

**Example:**
```bash
polyauthctl reset
# Confirm when prompted
```

### inspect

Display current user authentication settings, mounts, and configured authentication methods.

```bash
polyauthctl inspect [OPTIONS]
```

**Options:**
- `-u, --username <USER>` - Inspect settings for a specific user
- `-c, --config-file <PATH>` - Use a specific configuration file

**Output includes:**
- User or configuration file path
- Mount configuration hash
- Primary mount device and filesystem
- Mount flags and options
- Pre-mount configurations
- Default session command
- List of all configured authentication methods with:
  - Name
  - Creation date
  - Type (e.g., password)

**Example:**
```bash
# Inspect current user
polyauthctl inspect

# Inspect specific user
polyauthctl -u johndoe inspect
```

**Sample Output:**
```
-----------------------------------------------------------
👤 User: johndoe
-----------------------------------------------------------
🔑 hash: a1b2c3d4e5f6...
💾 device: /dev/sda1
📂 filesystem: ext4
⚙️  args: rw,relatime
***********************************************************
    📁 directory: /mnt/data
    💾 device: /dev/sdb1
    📂 filesystem: ext4
    ⚙️  args: rw,nosuid
-----------------------------------------------------------
🖥️  Default session command: /usr/bin/gnome-session
-----------------------------------------------------------
🔐 There are 2 authentication methods:
-----------------------------------------------------------
🏷️  name: backup-password
    📅 created at: 2025-01-15 10:30:45
    🔑 type: password
-----------------------------------------------------------
🏷️  name: emergency-access
    📅 created at: 2025-02-20 14:22:10
    🔑 type: password
-----------------------------------------------------------
```

### add

Add a new authentication method.

```bash
polyauthctl add --name <NAME> [OPTIONS] <METHOD>
```

**Options:**
- `--name <NAME>` - Name for the authentication method (required)
- `--intermediate <KEY>` - Intermediate key (prompted if not provided)

**Methods:**
- `password` - Add password-based authentication

#### Adding a Password

```bash
polyauthctl add --name <NAME> password [OPTIONS]
```

**Password Options:**
- `--secondary-pw <PASSWORD>` - Secondary password (prompted if not provided)

**Example:**
```bash
# Add a password with prompts
polyauthctl add --name backup-password password

# Add a password with all parameters
polyauthctl add --name backup-password --intermediate "my-key" password --secondary-pw "my-secondary-password"
```

**Interactive Flow:**
1. Enter intermediate key (if not provided)
2. Enter secondary password (if not provided)
3. Confirm secondary password

**Notes:**
- The intermediate key must match the one set during setup
- Secondary passwords are encrypted using the intermediate key
- You can have multiple authentication methods with different names

### set-session

Configure the default session command to execute when a user logs in.

```bash
polyauthctl set-session --cmd <COMMAND> [--args <ARG>...]
```

**Options:**
- `--cmd <COMMAND>` - Command to execute (required)
- `--args <ARG>` - Additional arguments for the command (optional, can be repeated)

**Example:**
```bash
# Set GNOME session as default
polyauthctl set-session --cmd /usr/bin/gnome-session

# Set custom session with arguments
polyauthctl set-session --cmd /usr/local/bin/my-session --args --debug --args --verbose
```

### set-home-mount

Configure the mount command for the user's home directory.

```bash
polyauthctl set-home-mount --device <DEVICE> --fstype <TYPE> [--flags <FLAG>...]
```

**Options:**
- `--device <DEVICE>` - Device to mount (required)
- `--fstype <TYPE>` - Filesystem type, e.g., ext4, btrfs, xfs (required)
- `--flags <FLAG>` - Mount options (optional, can be repeated)

**Example:**
```bash
# Mount an ext4 home directory
polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags rw --flags relatime

# Mount an encrypted device
polyauthctl set-home-mount --device /dev/mapper/home_crypt --fstype ext4 --flags rw --flags nosuid --flags nodev
```

**Common Filesystem Types:**
- `ext4` - Fourth extended filesystem
- `btrfs` - B-tree filesystem
- `xfs` - XFS filesystem
- `f2fs` - Flash-Friendly File System
- `ntfs` - NTFS (via ntfs-3g)

**Common Mount Flags:**
- `rw` - Read-write mode
- `ro` - Read-only mode
- `nosuid` - Do not allow set-user-ID or set-group-ID bits
- `nodev` - Do not interpret character or block special devices
- `noexec` - Do not allow direct execution of binaries
- `relatime` - Update inode access times relative to modify time
- `user_xattr` - Support user extended attributes
- `acl` - Support POSIX Access Control Lists

### set-pre-mount

Configure additional mounts that should be performed before mounting the home directory.

```bash
polyauthctl set-pre-mount --dir <DIRECTORY> --device <DEVICE> --fstype <TYPE> [--flags <FLAG>...]
```

**Options:**
- `--dir <DIRECTORY>` - Directory to mount the device into (required)
- `--device <DEVICE>` - Device to mount (required)
- `--fstype <TYPE>` - Filesystem type (required)
- `--flags <FLAG>` - Mount options (optional, can be repeated)

**Example:**
```bash
# Mount a data partition before home
polyauthctl set-pre-mount --dir /mnt/data --device /dev/sdb1 --fstype ext4 --flags rw --flags nosuid

# Mount a shared storage
polyauthctl set-pre-mount --dir /mnt/shared --device //server/share --fstype cifs --flags username=user --flags password=pass
```

**Use Cases:**
- Mount encrypted volumes before the home directory
- Mount network shares needed by the user
- Mount additional data partitions
- Set up complex storage hierarchies

### mount

Manage mount authorizations.

```bash
polyauthctl mount <SUBCOMMAND>
```

**Subcommands:**

#### mount authorize

Authorize a user to mount configured devices on each login.

```bash
polyauthctl mount authorize [OPTIONS]
```

**Options:**
- `-u, --username <USER>` - Username to authorize mount for (optional, defaults to current user)

**Example:**
```bash
# Authorize mounts for current user
polyauthctl mount authorize

# Authorize mounts for a specific user
polyauthctl mount authorize -u johndoe
```

**What it does:**
1. Loads the user's mount configuration
2. Calculates a hash of the mount configuration
3. Registers the hash with the mount authentication service via D-Bus
4. Allows the PAM module to automatically mount devices during login

**Notes:**
- You must have mount configurations set via `set-home-mount` or `set-pre-mount` first
- This authorization persists across reboots
- You need to reauthorize after changing mount configurations
- Requires the pam_polyauth-service to be running

## Global Options

These options can be used with any command:

- `-u, --username <USER>` - Specify a username (defaults to current user)
- `-c, --config-file <PATH>` - Use a specific configuration file instead of the default
- `-p, --password <PASSWORD>` - Provide the main password (not recommended for security reasons)
- `--update-as-needed` - Force update of user configuration if required

**Example:**
```bash
# Use a custom configuration file
polyauthctl -c /tmp/test-config.json inspect

# Operate on a specific user
polyauthctl -u johndoe inspect
```

## Examples

### Complete Setup for a New User

```bash
# 1. Initialize authentication
polyauthctl setup
# Enter intermediate key when prompted
# Enter main password when prompted

# 2. Configure home directory mount
polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags rw --flags relatime

# 3. Add a backup authentication method
polyauthctl add --name backup-password password
# Enter intermediate key
# Enter secondary password

# 4. Set default session
polyauthctl set-session --cmd /usr/bin/gnome-session

# 5. Authorize mounts
polyauthctl mount authorize

# 6. Verify configuration
polyauthctl inspect
```

### Adding Multiple Pre-Mounts

```bash
# Mount encrypted data partition
polyauthctl set-pre-mount --dir /mnt/encrypted --device /dev/mapper/data --fstype ext4 --flags rw

# Mount network storage
polyauthctl set-pre-mount --dir /mnt/nfs --device server:/export --fstype nfs --flags ro

# Mount USB storage
polyauthctl set-pre-mount --dir /media/usb --device /dev/sdc1 --fstype vfat --flags rw --flags utf8
```

### Resetting and Reconfiguring

```bash
# Reset all authentication data
polyauthctl reset

# Start fresh setup
polyauthctl setup

# Reconfigure everything as needed
polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags rw
polyauthctl mount authorize
```

## Configuration Files

### Default Locations

Configuration files are stored per-user in:
```
/var/lib/polyauth/<username>/
```

Or you can use custom locations with the `-c` flag.

### File Format

Configuration files use JSON format:

**Authentication Data:**
```json
{
  "main": {
    "encrypted_password": "...",
    "salt": "..."
  },
  "secondary": [
    {
      "name": "backup-password",
      "type": "password",
      "created_at": 1705315845,
      "encrypted_data": "..."
    }
  ]
}
```

**Mount Data:**
```json
{
  "mount": {
    "device": "/dev/sda1",
    "fstype": "ext4",
    "flags": ["rw", "relatime"]
  },
  "pre_mounts": {
    "/mnt/data": {
      "device": "/dev/sdb1",
      "fstype": "ext4",
      "flags": ["rw"]
    }
  }
}
```

## Security Considerations

### Intermediate Keys

- The intermediate key is used to encrypt secondary authentication methods
- Choose a strong, unique intermediate key
- Never share your intermediate key
- Store it securely (consider using a password manager)

### Passwords on Command Line

Avoid using the `-p` flag to provide passwords on the command line:
```bash
# BAD: Password visible in process list
polyauthctl -p "mypassword" setup

# GOOD: Use interactive prompts
polyauthctl setup
```

### File Permissions

- Configuration files should be readable only by the user and root
- Default permissions: `0600` (user read/write only)
- Check permissions: `ls -l /var/lib/polyauth/<username>/`

### Mount Security

- Use `nosuid` and `nodev` flags for non-system mounts
- Be cautious with network mounts (NFS, CIFS)
- Encrypted devices should be unlocked before mounting
- Review mount authorizations regularly

## Troubleshooting

### "User already has an intermediate key present"

You're trying to run `setup` when authentication is already configured.

**Solution:** Run `polyauthctl reset` first, then `polyauthctl setup`

### "User does not have mounts configured"

You're trying to authorize mounts but haven't configured any.

**Solution:** Configure mounts first:
```bash
polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags rw
polyauthctl mount authorize
```

### "Error connecting to system bus"

The D-Bus system bus is not available or you don't have permission.

**Solution:**
- Ensure D-Bus is running: `systemctl status dbus`
- Check if pam_polyauth-service is running: `systemctl status pam_polyauth`

### "Error in loading user mounts data"

Configuration file is corrupted or unreadable.

**Solution:**
- Check file permissions
- Verify JSON format
- Try resetting: `polyauthctl reset`

### "Intermediate key and confirmation not matching"

You typed different intermediate keys during setup.

**Solution:** Run `setup` again and ensure you type the same key twice

### "Could not verify the correctness of the intermediate key"

The intermediate key you provided doesn't match the stored one.

**Solution:**
- Double-check your intermediate key
- If forgotten, you'll need to run `polyauthctl reset` and reconfigure

### Verbose Debugging

For debugging issues, check system logs:
```bash
# View polyauth service logs
journalctl -u pam_polyauth

# View all polyauth-related logs
journalctl | grep polyauth

# Follow logs in real-time
journalctl -u pam_polyauth -f
```

## License

pam_polyauth and polyauthctl are licensed under the GNU General Public License v2.0.

Copyright (C) 2024-2025 Denis Benato

This program comes with ABSOLUTELY NO WARRANTY. This is free software, and you are welcome to redistribute it under certain conditions. See the LICENSE.md file for details.

## Support

For bug reports and feature requests, please visit:
https://github.com/NeroReflex/polyauth

---

**Last Updated:** November 2025  
**Version:** 0.8.5

