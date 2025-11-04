# polyauthctl Documentation Index

Welcome to the polyauthctl documentation! This directory contains comprehensive documentation for using and maintaining polyauthctl.

## Available Documentation

### 1. User Manual (README.md)
**[README.md](README.md)** - Comprehensive user manual in Markdown format

This is the main documentation file containing:
- Complete command reference
- Detailed examples
- Configuration guides
- Security best practices
- Troubleshooting information

**Best for:**
- Learning how to use polyauthctl
- Finding command syntax and options
- Understanding configuration files
- Troubleshooting issues

**View online:** Can be read directly in GitHub or any markdown viewer

---

### 2. Man Page (polyauthctl.1)
**[polyauthctl.1](polyauthctl.1)** - Traditional Unix man page in groff format

Standard Unix manual page compatible with the `man` command system.

**Best for:**
- Quick reference from the command line
- System administrators familiar with man pages
- Integration with Unix help systems
- Offline documentation access

**Usage:**
```bash
# View the manpage directly
man ./Manual/polyauthctl.1

# After installation
man polyauthctl
```

---

### 3. Installation Guide (INSTALL.md)
**[INSTALL.md](INSTALL.md)** - Documentation installation guide

Instructions for installing and managing the documentation itself.

**Contains:**
- System-wide installation instructions
- User-local installation
- Package integration
- Troubleshooting installation issues
- Converting to other formats (PDF, HTML, etc.)

---

### 4. Shell Completions (../completions/)
**[../completions/README.md](../completions/README.md)** - Shell completion installation and usage

**Available for:**
- Bash - Tab completion for all commands and options
- Zsh - Tab completion with descriptions

**Quick Install:**
```bash
# Bash
sudo cp completions/polyauthctl.bash /usr/share/bash-completion/completions/polyauthctl

# Zsh
sudo cp completions/polyauthctl.zsh /usr/share/zsh/site-functions/_polyauthctl
```

---

## Quick Start

### For New Users
1. Start with **[README.md](README.md)** sections:
   - Getting Started
   - Commands overview
   - Examples

2. Try the basic setup:
   ```bash
   polyauthctl setup
   polyauthctl inspect
   ```

3. Refer to the man page for quick command reference:
   ```bash
   man polyauthctl
   ```

### For System Administrators
1. Read **[INSTALL.md](INSTALL.md)** for documentation deployment
2. Review **[README.md](README.md)** sections:
   - Security Considerations
   - Configuration Files
   - Troubleshooting

3. Install the manpage system-wide:
   ```bash
   sudo cp Manual/polyauthctl.1 /usr/share/man/man1/
   sudo mandb
   ```

## Documentation Structure

```
Manual/
├── INDEX.md              # This file - documentation overview
├── README.md             # Complete user manual (Markdown)
├── polyauthctl.1         # Man page (groff format)
└── INSTALL.md            # Installation guide for documentation
```

## Accessing Documentation

### From the Command Line

**Man Page (after installation):**
```bash
man polyauthctl
man -k polyauth         # Search for polyauth-related pages
```

**Markdown Manual:**
```bash
# View in terminal
less Manual/README.md
cat Manual/README.md | less

# With syntax highlighting (if installed)
bat Manual/README.md
glow Manual/README.md
mdless Manual/README.md
```

### From a Web Browser

**View Markdown:**
- Open README.md in any text editor
- Use a markdown preview extension
- View on GitHub repository

**Convert Man Page to HTML:**
```bash
man -l -Thtml Manual/polyauthctl.1 > polyauthctl.html
xdg-open polyauthctl.html
```

### Offline PDF Generation

**From Man Page:**
```bash
man -l -Tpdf Manual/polyauthctl.1 > polyauthctl.pdf
```

**From Markdown:**
```bash
# Using pandoc
pandoc Manual/README.md -o polyauthctl.pdf

# Using mdpdf (if installed)
mdpdf Manual/README.md
```

## Searching Documentation

### Search Man Page
```bash
# Search for a specific term
man polyauthctl | grep -i "mount"

# Show section about a command
man polyauthctl | sed -n '/^COMMANDS/,/^EXAMPLES/p'
```

### Search Markdown Manual
```bash
# Search for specific content
grep -n "setup" Manual/README.md

# Case-insensitive search
grep -ni "authentication" Manual/README.md

# Search with context
grep -A 5 -B 5 "mount authorize" Manual/README.md
```

## Documentation Maintenance

### Keeping Docs in Sync

When updating polyauthctl code:

1. Update functionality in `src/bin/polyauthctl/main.rs`
2. Update **README.md** with new features/changes
3. Update **polyauthctl.1** man page
4. Test both documents for accuracy
5. Update version numbers if needed

### Building with Documentation

The documentation is automatically included when building packages:

```bash
# Build Debian package (includes manpage)
cargo deb

# Install with Makefile (includes manpage)
make install PREFIX=/usr/local
```

## Contributing to Documentation

When contributing documentation improvements:

1. **For new features:**
   - Add to both README.md and polyauthctl.1
   - Include examples
   - Update the relevant sections

2. **For bug fixes in docs:**
   - Ensure consistency between Markdown and man page
   - Verify command syntax is correct
   - Test examples before submitting

3. **For formatting:**
   - Markdown: Use standard GitHub-flavored Markdown
   - Man page: Follow groff formatting conventions
   - Keep line length reasonable (80-100 chars)

## Documentation Conventions

### Command Syntax
- **Required arguments**: `<ARGUMENT>`
- **Optional arguments**: `[ARGUMENT]`
- **Choices**: `{choice1|choice2}`
- **Repeatable**: `[ARGUMENT]...`

### Examples
All examples should:
- Be tested and working
- Include expected output where relevant
- Show both simple and complex use cases
- Include error handling when appropriate

### Formatting
- Use **bold** for commands and important terms
- Use `code blocks` for file paths, variables, and code
- Use > blockquotes for important warnings
- Use lists for steps or options

## Getting Help

If the documentation doesn't answer your question:

1. Check the **Troubleshooting** section in README.md
2. Run `polyauthctl info` for version information
3. Check system logs: `journalctl -u pam_polyauth`
4. Visit the project repository for issues and discussions
5. File a bug report if you've found an issue

## Additional Resources

- **Project Repository**: https://github.com/NeroReflex/polyauth
- **Issue Tracker**: https://github.com/NeroReflex/polyauth/issues
- **Related Documentation**:
  - `man pam` - PAM documentation
  - `man mount` - Mount command documentation
  - `man pam_polyauth-service` - Service documentation

## License

Documentation is licensed under the same terms as pam_polyauth:

Copyright (C) 2024-2025 Denis Benato  
License: GNU General Public License v2.0

---

**Documentation Version:** 0.8.5  
**Last Updated:** November 2025

