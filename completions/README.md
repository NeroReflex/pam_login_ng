# Shell Completion Scripts for polyauthctl

This directory contains shell completion scripts for `polyauthctl` that enable tab-completion for commands, subcommands, and options.

## Available Shells

- **Bash** - `polyauthctl.bash`
- **Zsh** - `polyauthctl.zsh`

## Installation

### Bash Completion

#### System-wide (requires root)
```bash
sudo cp completions/polyauthctl.bash /usr/share/bash-completion/completions/polyauthctl
```

After installation, completions will be automatically loaded in new bash sessions.

#### Local user installation
```bash
# Create local completions directory
mkdir -p ~/.local/share/bash-completion/completions

# Copy completion script
cp completions/polyauthctl.bash ~/.local/share/bash-completion/completions/polyauthctl

# Add to ~/.bashrc if not already present
echo 'export BASH_COMPLETION_USER_DIR="$HOME/.local/share/bash-completion"' >> ~/.bashrc

# Reload bash configuration
source ~/.bashrc
```

#### Manual load (temporary)
```bash
source completions/polyauthctl.bash
```

### Zsh Completion

#### System-wide (requires root)
```bash
sudo cp completions/polyauthctl.zsh /usr/share/zsh/site-functions/_polyauthctl
```

After installation, reload zsh completions:
```bash
rm -f ~/.zcompdump
compinit
```

#### Local user installation
```bash
# Create local completions directory
mkdir -p ~/.zsh/completions

# Copy completion script
cp completions/polyauthctl.zsh ~/.zsh/completions/_polyauthctl

# Add to ~/.zshrc if not already present
cat >> ~/.zshrc << 'EOF'

# Add custom completions directory
fpath=(~/.zsh/completions $fpath)

# Initialize completion system
autoload -Uz compinit
compinit
EOF

# Reload zsh configuration
source ~/.zshrc
```

#### Manual load (temporary)
```bash
# Add to fpath and reload
fpath=(./completions $fpath)
autoload -Uz compinit
compinit
```

### Verification

After installation, verify that completions work:

```bash
# Type this and press TAB
polyauthctl <TAB>

# Should show: info setup reset inspect add set-session set-home-mount set-pre-mount mount

# Try subcommand completion
polyauthctl mount <TAB>

# Should show: authorize

# Try option completion
polyauthctl -<TAB>

# Should show: -u --username -c --config-file -p --password --update-as-needed --help
```

## Features

### Global Options Completion
- `-u/--username` - Completes with system usernames
- `-c/--config-file` - Completes with file paths
- `-p/--password` - No completion (security)
- `--update-as-needed` - Flag completion

### Command-Specific Completions

#### `setup`
- `-i/--intermediate` - No completion (security)

#### `add`
- `--name` - User provides name
- `--intermediate` - No completion (security)
- Method completion: `password`
- `--secondary-pw` - No completion (security)

#### `set-session`
- `--cmd` - Completes with available commands
- `--args` - Completes with file paths

#### `set-home-mount` / `set-pre-mount`
- `--device` - Completes with block devices from `/dev/`
- `--fstype` - Completes with common filesystem types:
  - ext4, ext3, ext2
  - btrfs, xfs, f2fs
  - ntfs, vfat, exfat
  - nfs, cifs (set-pre-mount only)
- `--flags` - Completes with common mount flags:
  - rw, ro
  - nosuid, nodev, noexec
  - relatime, noatime
  - user_xattr, acl

#### `mount`
- Subcommand completion: `authorize`
- Uses global `-u/--username` for user selection

## Examples

### Bash
```bash
# Complete command
$ polyauthctl <TAB>
info  setup  reset  inspect  add  set-session  set-home-mount  set-pre-mount  mount

# Complete options
$ polyauthctl -<TAB>
-u  --username  -c  --config-file  -p  --password  --update-as-needed  --help

# Complete filesystem types
$ polyauthctl set-home-mount --device /dev/sda1 --fstype <TAB>
ext4  ext3  ext2  btrfs  xfs  f2fs  ntfs  vfat  exfat

# Complete mount flags
$ polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags <TAB>
rw  ro  nosuid  nodev  noexec  relatime  noatime  user_xattr  acl
```

### Zsh
```bash
# Complete with descriptions
$ polyauthctl <TAB>
info           -- Print information about the software
setup          -- Setup initial authentication data also creating a new intermediate key
reset          -- Reset additional authentication data also destroying the intermediate key
inspect        -- Inspects user login settings
add            -- Add a new authentication method
set-session    -- Set the default session command to be executed when a user login
set-home-mount -- Set the mount command that has to be used to mount the user home directory
set-pre-mount  -- Set the mount command that has to be used to mount additional directories
mount          -- Mount management commands

# Complete filesystem types with descriptions
$ polyauthctl set-home-mount --fstype <TAB>
ext4   -- Fourth extended filesystem
btrfs  -- B-tree filesystem
xfs    -- XFS filesystem
...
```

## Troubleshooting

### Bash: Completions not working

1. **Check if bash-completion is installed:**
   ```bash
   # Debian/Ubuntu
   sudo apt-get install bash-completion
   
   # Fedora/RHEL
   sudo dnf install bash-completion
   
   # Arch
   sudo pacman -S bash-completion
   ```

2. **Verify bash-completion is enabled:**
   ```bash
   # Should be in /etc/bash.bashrc or ~/.bashrc
   grep -r "bash_completion" /etc/bash.bashrc ~/.bashrc
   ```

3. **Manually source the completion:**
   ```bash
   source /usr/share/bash-completion/completions/polyauthctl
   ```

4. **Check if the function is loaded:**
   ```bash
   type _polyauthctl
   # Should output the function definition
   ```

### Zsh: Completions not working

1. **Check if completion system is initialized:**
   ```bash
   # Add to ~/.zshrc if missing
   autoload -Uz compinit
   compinit
   ```

2. **Clear completion cache:**
   ```bash
   rm -f ~/.zcompdump*
   compinit
   ```

3. **Check fpath:**
   ```bash
   echo $fpath
   # Should include the directory containing _polyauthctl
   ```

4. **Verify the completion file is found:**
   ```bash
   which _polyauthctl
   # Or check:
   whence -v _polyauthctl
   ```

5. **Ensure file has correct name:**
   - Must be named `_polyauthctl` (with underscore prefix)
   - Must be in a directory listed in `$fpath`

### General Issues

**Completions show for wrong command:**
- Ensure only one completion file per shell
- Check for conflicts: `complete -p | grep polyauthctl` (bash)

**Partial completions only:**
- This is normal for context-sensitive completion
- Type more of the command to get more specific completions

**No device completion:**
- Ensure `/dev` is accessible
- Check permissions on `/dev/sd*`, `/dev/nvme*`, etc.

**Command completions don't show:**
- For `--cmd`, ensure `compgen -c` works (bash)
- For zsh, ensure `_command_names` is available

## Package Installation

When installing via package manager (apt, dnf, etc.), completions are automatically installed to:

- **Bash**: `/usr/share/bash-completion/completions/polyauthctl`
- **Zsh**: `/usr/share/zsh/site-functions/_polyauthctl`

No manual installation needed for packaged installs.

## Development

### Testing Bash Completions
```bash
# Load in current shell
source completions/polyauthctl.bash

# Enable debugging
set -x
polyauthctl <TAB>
set +x
```

### Testing Zsh Completions
```bash
# Load in current shell
fpath=(./completions $fpath)
autoload -Uz compinit
compinit

# Enable debugging
zstyle ':completion:*' verbose yes
zstyle ':completion:*:descriptions' format '%B%d%b'
```

### Updating Completions

When adding new commands or options to polyauthctl:

1. Update the respective completion script
2. Test both bash and zsh versions
3. Update this README if needed
4. Increment version in completion comments

## Contributing

When contributing completion improvements:

1. Test on both bash and zsh
2. Ensure security-sensitive options don't complete
3. Add helpful descriptions (especially for zsh)
4. Follow existing code style
5. Test with various shell configurations

## License

These completion scripts are part of pam_polyauth and are licensed under GPL v2.0.

Copyright (C) 2024-2025 Denis Benato

---

**Last Updated:** November 2025  
**Compatible with:** polyauthctl 0.8.5+

