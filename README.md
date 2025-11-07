# polyauth

A pam plugin with the additional (and totally optional) feature of shielding the real password
behind another password that can be unlocked by various means while logging in:
    - __autologin__: provide autologin functionality that has been long lost in systemd-homed
    - __secondary password(s)__: allow the use of one or more secondary passwords
    - __controllers__: enter a password via a gaming controller
    - __fingerprint__: no password required: login via fingerprint
    - __files__ use a specific file on some kind of removable media to authenticate
    - __pin__ a numeric pin just as in your phone

Using *polyauthctl* utility you can set more authentication options or even configure an
account with an encrypted home directory.

## Security considerations

When *additional authentication methods* feature is not in use the security of the account is not expected to any different,
than a standard account, however when such feature is used there are additional considerations: __read below__.

__Disclaimer__: Despite me attempting to provide something that is at least as secure as the secondary authentication method used
(because if you set an empty password that will nullify the security of home encryption) and no more secure than the
main account password, this software is very new and born out of a necessity of mine, with my usecase in mind
and only my own experience to back up security.

With the above in mind it's firstly necessary to address the elefant in the room:
I have avoided what I was aware of in terms of attacks to encryption and tried to stick with audited rust crates, or well-known ones:
aes_gcm has received a security audit, and this software actively avoids to reuse the same (nonce, password) tuple, however if
adding and removing secondary authentication methods up to the point of collisions of randomly-generated nonce (96bits) data being a possibility
then this software is not suitable!

In addition to the above warning the user must know that using the additional authentication features will
require the user to have an intermediate key that cannot be changed unless every data associated with this software is removed for
the specific user; that intermediate key is used to decrypt the user main password, therefore in case such intermediate key is disclosed
changing the main password won't suffice!

The good part is that knowing the main password will __NOT__ disclose the intermediate key nor any secondary authentication method,
and knowing the intermediate key will __NOT__ disclose secondary authentication methods.

## Documentation

Comprehensive documentation is available in the `Manual/` directory:

- **[Manual/README.md](Manual/README.md)** - Complete user manual with detailed command reference, examples, and troubleshooting
- **[Manual/polyauthctl.1](Manual/polyauthctl.1)** - Man page for `polyauthctl` command
- **[Manual/INSTALL.md](Manual/INSTALL.md)** - Instructions for installing documentation
- **[Manual/MIGRATION.md](Manual/MIGRATION.md)** - Migration guide from `pam_polyauth-mount` to `polyauthctl mount`
- **[Manual/INDEX.md](Manual/INDEX.md)** - Documentation overview and quick start guide

### Quick Start with polyauthctl

```bash
# Initialize authentication
polyauthctl setup

# Configure home directory mount
polyauthctl set-home-mount --device /dev/sda1 --fstype ext4 --flags rw

# Add a backup authentication method
polyauthctl add --name backup-password password

# Authorize mounts
polyauthctl mount authorize

# Verify configuration
polyauthctl inspect
```

For detailed usage, see `man polyauthctl` or the comprehensive manual in `Manual/README.md`.

### Shell Completion

Tab-completion is available for both **bash** and **zsh**:

**Installation:**
```bash
# Bash (system-wide)
sudo cp completions/polyauthctl.bash /usr/share/bash-completion/completions/polyauthctl

# Zsh (system-wide)
sudo cp completions/polyauthctl.zsh /usr/share/zsh/site-functions/_polyauthctl
```

Completions are automatically installed when using the package manager. See `completions/README.md` for detailed installation instructions and troubleshooting.

## Additional notes

Here is some notes of general interest:
    - on Archlinux if you install the *kwallet-pam* package and your wallet password is the same as your account the wallet can be automatically unlocked: this will chain with autologin: [Archlinux Wiki](https://wiki.archlinux.org/title/KDE_Wallet).
