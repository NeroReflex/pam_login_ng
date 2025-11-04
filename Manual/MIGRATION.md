# Migration Guide: pam_polyauth-mount to polyauthctl

This document explains the migration from the standalone `pam_polyauth-mount` binary to the integrated `polyauthctl mount` commands.

## What Changed

### Before (pam_polyauth-mount)
```bash
# Authorize mounts
pam_polyauth-mount authorize

# With specific user
pam_polyauth-mount authorize -u username
```

### After (polyauthctl mount)
```bash
# Authorize mounts
polyauthctl mount authorize

# With specific user  
polyauthctl mount authorize -u username
```

## Why the Change

The `pam_polyauth-mount` functionality has been integrated into `polyauthctl` to:

1. **Consolidation**: All polyauth management in one tool
2. **Consistency**: Unified command interface and options
3. **Maintainability**: Single codebase for all user-facing commands
4. **Discoverability**: All features accessible from `polyauthctl --help`

## Migration Steps

### For End Users

If you have scripts or workflows using `pam_polyauth-mount`:

1. **Update your scripts:**
   ```bash
   # Old command
   pam_polyauth-mount authorize
   
   # New command
   polyauthctl mount authorize
   ```

2. **Update documentation:**
   Replace references to `pam_polyauth-mount` with `polyauthctl mount`

3. **Test the new commands:**
   ```bash
   # Verify mount authorization works
   polyauthctl mount authorize
   
   # Check your mount configuration
   polyauthctl inspect
   ```

### For System Administrators

1. **Update system scripts and automation:**
   ```bash
   # Find all references to pam_polyauth-mount
   grep -r "pam_polyauth-mount" /etc/ /usr/local/bin/
   
   # Update each file
   sed -i 's/pam_polyauth-mount/polyauthctl mount/g' /path/to/script
   ```

2. **Update systemd services** (if any):
   ```ini
   # Old
   ExecStart=/usr/bin/pam_polyauth-mount authorize
   
   # New
   ExecStart=/usr/bin/polyauthctl mount authorize
   ```

3. **Verify the binary is no longer needed:**
   ```bash
   # After package update, pam_polyauth-mount should be removed
   which pam_polyauth-mount  # Should not exist
   which polyauthctl          # Should exist
   ```

### For Package Maintainers

When upgrading packages:

1. **Remove old binary:**
   - Remove `/usr/bin/pam_polyauth-mount`

2. **Add/update polyauthctl:**
   - Ensure `/usr/bin/polyauthctl` is installed

3. **Update package documentation:**
   - Update man pages
   - Update package description
   - Add migration notes to changelog

4. **Handle conflicts:**
   ```
   Conflicts: pam-polyauth-mount (<< 0.8.5)
   Replaces: pam-polyauth-mount (<< 0.8.5)
   ```

## Compatibility

### Backward Compatibility

The new `polyauthctl mount` commands use the **exact same** D-Bus interface and configuration files as the old `pam_polyauth-mount`, so:

- ✅ Existing mount configurations work unchanged
- ✅ Authorization data remains valid
- ✅ No need to reconfigure mounts
- ✅ D-Bus service compatibility maintained

### What Stays the Same

- Mount configuration files location and format
- D-Bus service interface (`org.neroreflex.polyauth_mount`)
- Authorization hash calculation
- Configuration file paths
- Mount authorization persistence

## Command Comparison Table

| Old Command | New Command | Notes |
|------------|-------------|-------|
| `pam_polyauth-mount info` | `polyauthctl info` | Version info now unified |
| `pam_polyauth-mount authorize` | `polyauthctl mount authorize` | Same functionality |
| `pam_polyauth-mount authorize -u USER` | `polyauthctl mount authorize -u USER` | Same options |

## Troubleshooting

### "Command not found: pam_polyauth-mount"

This is expected after upgrading. Use `polyauthctl mount` instead.

### Scripts Still Reference Old Command

Update your scripts using:

```bash
# Create a temporary wrapper (not recommended for long-term)
sudo tee /usr/local/bin/pam_polyauth-mount > /dev/null <<'EOF'
#!/bin/bash
exec polyauthctl mount "$@"
EOF
sudo chmod +x /usr/local/bin/pam_polyauth-mount
```

**Note:** This is only for temporary compatibility. Update scripts to use the new command.

### Authorization Not Working

The authorization functionality is identical. If you experience issues:

1. Verify mount configuration:
   ```bash
   polyauthctl inspect
   ```

2. Check D-Bus service:
   ```bash
   systemctl status pam_polyauth
   ```

3. Try authorizing again:
   ```bash
   polyauthctl mount authorize
   ```

## Timeline

- **Version 0.8.5**: `pam_polyauth-mount` deprecated, functionality moved to `polyauthctl mount`
- **Current**: Both binaries may coexist during transition
- **Future**: `pam_polyauth-mount` binary completely removed

## Getting Help

If you encounter issues during migration:

1. Check this migration guide
2. Review the main manual: `man polyauthctl` or `Manual/README.md`
3. Check system logs: `journalctl -u pam_polyauth`
4. File an issue: https://github.com/NeroReflex/polyauth/issues

## Additional Resources

- [polyauthctl Manual](README.md) - Complete command reference
- [Installation Guide](INSTALL.md) - Installation instructions
- [Man Page](polyauthctl.1) - Quick reference

---

**Migration Guide Version:** 1.0  
**Applicable to pam_polyauth:** ≥ 0.8.5  
**Last Updated:** November 2025

