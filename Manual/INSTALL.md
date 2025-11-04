# Manual Installation Guide

## Installing the Manual Page

### System-wide Installation

To install the manpage system-wide, copy it to the appropriate location:

```bash
# Copy manpage to system location
sudo cp Manual/polyauthctl.1 /usr/share/man/man1/

# Update the man database
sudo mandb

# Test the installation
man polyauthctl
```

### Local User Installation

For a single user installation:

```bash
# Create local man directory if it doesn't exist
mkdir -p ~/.local/share/man/man1

# Copy manpage
cp Manual/polyauthctl.1 ~/.local/share/man/man1/

# Update MANPATH in your shell profile (~/.bashrc, ~/.zshrc, etc.)
echo 'export MANPATH="$HOME/.local/share/man:$MANPATH"' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc

# Test the installation
man polyauthctl
```

### Viewing Without Installation

You can view the manpage without installing it:

```bash
# View directly with man
man ./Manual/polyauthctl.1

# Or convert to text
man -l ./Manual/polyauthctl.1

# Or convert to PDF
man -l -Tpdf ./Manual/polyauthctl.1 > polyauthctl.pdf
```

## Accessing the Markdown Manual

The comprehensive markdown manual is available at:

```
Manual/README.md
```

You can view it with any markdown viewer or text editor:

```bash
# View in terminal
less Manual/README.md

# Or with a markdown viewer if installed
mdless Manual/README.md
glow Manual/README.md
```

## Building the Package

When building the Debian package, the manpage will be automatically installed:

```bash
# Build the package
cargo deb

# Install the package (includes manpage)
sudo dpkg -i target/debian/pam_polyauth_*.deb
```

## Verifying Installation

After installation, verify that the manpage is accessible:

```bash
# Check if the manpage exists
man -w polyauthctl

# View the manpage
man polyauthctl

# Search for a specific section
man polyauthctl | grep -A 5 "EXAMPLES"
```

## Uninstallation

To remove the manpage:

### System-wide
```bash
sudo rm /usr/share/man/man1/polyauthctl.1
sudo mandb
```

### Local user
```bash
rm ~/.local/share/man/man1/polyauthctl.1
```

## Troubleshooting

### "No manual entry for polyauthctl"

This means the manpage is not in your MANPATH.

**Solutions:**
1. Check if the manpage file exists:
   ```bash
   ls -l /usr/share/man/man1/polyauthctl.1
   ```

2. Update the man database:
   ```bash
   sudo mandb
   ```

3. Check your MANPATH:
   ```bash
   echo $MANPATH
   ```

### Manpage not formatting correctly

The manpage uses groff format. Ensure you have `man` or `groff` installed:

```bash
# On Debian/Ubuntu
sudo apt-get install man-db groff

# On Fedora/RHEL
sudo dnf install man-db groff

# On Arch
sudo pacman -S man-db groff
```

## Converting the Manpage

### To HTML
```bash
man -l -Thtml ./Manual/polyauthctl.1 > polyauthctl.html
```

### To PDF
```bash
man -l -Tpdf ./Manual/polyauthctl.1 > polyauthctl.pdf
```

### To Plain Text
```bash
man -l ./Manual/polyauthctl.1 | col -b > polyauthctl.txt
```

## Integration with Package Managers

### Debian Package

The manpage is automatically included in the Debian package. When users install
the package via `apt` or `dpkg`, the manpage will be installed to
`/usr/share/man/man1/polyauthctl.1.gz` (compressed).

### Manual RPM Creation

For RPM-based distributions, add to your `.spec` file:

```spec
%files
/usr/bin/polyauthctl
/usr/share/man/man1/polyauthctl.1.gz
```

## Keeping Documentation in Sync

When updating polyauthctl functionality:

1. Update the Markdown manual (`Manual/README.md`)
2. Update the manpage (`Manual/polyauthctl.1`)
3. Test both documents for accuracy
4. Rebuild and reinstall to verify changes

## Additional Resources

- **Man Page Format Reference**: `man 7 man`
- **Groff Documentation**: `info groff`
- **Markdown Guide**: https://www.markdownguide.org/

