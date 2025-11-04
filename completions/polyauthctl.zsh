#compdef polyauthctl
# Zsh completion script for polyauthctl
# Install to: /usr/share/zsh/site-functions/_polyauthctl or ~/.zsh/completions/_polyauthctl

_polyauthctl() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    # Define global options
    local -a global_opts
    global_opts=(
        '(-u --username)'{-u,--username}'[username to be used]:username:_users'
        '(-c --config-file)'{-c,--config-file}'[force the use of a specific configuration file]:config file:_files'
        '(-p --password)'{-p,--password}'[main password for authentication]:password:'
        '--update-as-needed[force update of user configuration if required]'
        '(- *)--help[display usage information]'
    )

    # Main command specification
    _arguments -C \
        $global_opts \
        '1: :->command' \
        '*:: :->args' \
        && return 0

    case $state in
        command)
            local -a commands
            commands=(
                'info:Print information about the software'
                'setup:Setup initial authentication data also creating a new intermediate key'
                'reset:Reset additional authentication data also destroying the intermediate key'
                'inspect:Inspects user login settings'
                'add:Add a new authentication method'
                'set-session:Set the default session command to be executed when a user login'
                'set-home-mount:Set the mount command that has to be used to mount the user home directory'
                'set-pre-mount:Set the mount command that has to be used to mount additional directories'
                'mount:Mount management commands'
            )
            _describe 'command' commands
            ;;

        args)
            case $words[1] in
                info)
                    # No arguments for info
                    ;;

                setup)
                    _arguments \
                        '(-i --intermediate)'{-i,--intermediate}'[the intermediate key]:intermediate key:'
                    ;;

                reset)
                    # No arguments for reset
                    ;;

                inspect)
                    # Uses global options only
                    ;;

                add)
                    local -a add_methods
                    add_methods=(
                        'password:Add password-based authentication'
                    )

                    _arguments -C \
                        '--name[name of the authentication method]:name:' \
                        '--intermediate[intermediate key]:intermediate key:' \
                        '1: :->method' \
                        '*:: :->method_args' \
                        && return 0

                    case $state in
                        method)
                            _describe 'authentication method' add_methods
                            ;;
                        method_args)
                            case $words[1] in
                                password)
                                    _arguments \
                                        '--secondary-pw[secondary password for authentication]:secondary password:'
                                    ;;
                            esac
                            ;;
                    esac
                    ;;

                set-session)
                    _arguments \
                        '--cmd[command to execute]:command:_command_names' \
                        '*--args[additional arguments for the command]:argument:'
                    ;;

                set-home-mount)
                    local -a fstypes
                    fstypes=(
                        'ext4:Fourth extended filesystem'
                        'ext3:Third extended filesystem'
                        'ext2:Second extended filesystem'
                        'btrfs:B-tree filesystem'
                        'xfs:XFS filesystem'
                        'f2fs:Flash-Friendly File System'
                        'ntfs:NTFS filesystem'
                        'vfat:FAT filesystem'
                        'exfat:exFAT filesystem'
                    )

                    local -a mount_flags
                    mount_flags=(
                        'rw:Read-write mode'
                        'ro:Read-only mode'
                        'nosuid:Do not allow set-user-ID'
                        'nodev:Do not interpret character or block special devices'
                        'noexec:Do not allow direct execution of binaries'
                        'relatime:Update inode access times relative to modify time'
                        'noatime:Do not update inode access times'
                        'user_xattr:Support user extended attributes'
                        'acl:Support POSIX Access Control Lists'
                    )

                    _arguments \
                        '--device[device to mount]:device:_files -W /dev' \
                        '--fstype[filesystem type]:filesystem type:_describe "filesystem type" fstypes' \
                        '*--flags[mount options]:mount flag:_describe "mount flag" mount_flags'
                    ;;

                set-pre-mount)
                    local -a fstypes
                    fstypes=(
                        'ext4:Fourth extended filesystem'
                        'ext3:Third extended filesystem'
                        'ext2:Second extended filesystem'
                        'btrfs:B-tree filesystem'
                        'xfs:XFS filesystem'
                        'f2fs:Flash-Friendly File System'
                        'ntfs:NTFS filesystem'
                        'vfat:FAT filesystem'
                        'exfat:exFAT filesystem'
                        'nfs:Network File System'
                        'cifs:Common Internet File System'
                    )

                    local -a mount_flags
                    mount_flags=(
                        'rw:Read-write mode'
                        'ro:Read-only mode'
                        'nosuid:Do not allow set-user-ID'
                        'nodev:Do not interpret character or block special devices'
                        'noexec:Do not allow direct execution of binaries'
                        'relatime:Update inode access times relative to modify time'
                        'noatime:Do not update inode access times'
                        'user_xattr:Support user extended attributes'
                        'acl:Support POSIX Access Control Lists'
                    )

                    _arguments \
                        '--dir[directory to mount the device into]:directory:_directories' \
                        '--device[device to mount]:device:_files -W /dev' \
                        '--fstype[filesystem type]:filesystem type:_describe "filesystem type" fstypes' \
                        '*--flags[mount options]:mount flag:_describe "mount flag" mount_flags'
                    ;;

                mount)
                    local -a mount_commands
                    mount_commands=(
                        'authorize:Authorize a user to mount devices on each login'
                    )

                    _arguments -C \
                        '1: :->mount_command' \
                        '*:: :->mount_args' \
                        && return 0

                    case $state in
                        mount_command)
                            _describe 'mount command' mount_commands
                            ;;
                        mount_args)
                            case $words[1] in
                                authorize)
                                    # Uses global -u option
                                    ;;
                            esac
                            ;;
                    esac
                    ;;
            esac
            ;;
    esac
}

_polyauthctl "$@"

