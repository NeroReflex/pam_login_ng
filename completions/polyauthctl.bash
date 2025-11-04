#!/bin/bash
# Bash completion script for polyauthctl
# Install to: /usr/share/bash-completion/completions/polyauthctl

_polyauthctl() {
    local cur prev words cword
    _init_completion || return

    # Global options
    local global_opts="-u --username -c --config-file -p --password --update-as-needed --help"
    
    # Main commands
    local commands="info setup reset inspect add set-session set-home-mount set-pre-mount mount"
    
    # Mount subcommands
    local mount_cmds="authorize"
    
    # Add subcommands
    local add_methods="password"

    # Get the command position (after global options)
    local cmd_pos=1
    local cmd=""
    local i
    for ((i=1; i < cword; i++)); do
        case "${words[i]}" in
            -u|--username|-c|--config-file|-p|--password)
                ((i++))  # Skip the argument
                ;;
            --update-as-needed|--help)
                ;;
            info|setup|reset|inspect|add|set-session|set-home-mount|set-pre-mount|mount)
                cmd="${words[i]}"
                cmd_pos=$i
                break
                ;;
        esac
    done

    # If we haven't found a command yet, complete commands and global options
    if [[ -z "$cmd" ]]; then
        case "$prev" in
            -u|--username)
                # Complete usernames
                COMPREPLY=($(compgen -u -- "$cur"))
                return
                ;;
            -c|--config-file)
                # Complete file paths
                _filedir
                return
                ;;
            -p|--password)
                # Don't complete passwords
                return
                ;;
            *)
                COMPREPLY=($(compgen -W "$global_opts $commands" -- "$cur"))
                return
                ;;
        esac
    fi

    # Command-specific completion
    case "$cmd" in
        info)
            # No options for info
            return
            ;;
        
        setup)
            case "$prev" in
                -i|--intermediate)
                    # Don't complete intermediate key
                    return
                    ;;
                *)
                    COMPREPLY=($(compgen -W "-i --intermediate" -- "$cur"))
                    return
                    ;;
            esac
            ;;
        
        reset)
            # No options for reset
            return
            ;;
        
        inspect)
            # No specific options (uses global options)
            return
            ;;
        
        add)
            # Check if we already have a method
            local has_method=0
            local j
            for ((j=cmd_pos+1; j < cword; j++)); do
                if [[ "${words[j]}" == "password" ]]; then
                    has_method=1
                    break
                fi
            done

            case "$prev" in
                --name)
                    # User provides name
                    return
                    ;;
                --intermediate)
                    # Don't complete intermediate key
                    return
                    ;;
                --secondary-pw)
                    # Don't complete password
                    return
                    ;;
                add)
                    COMPREPLY=($(compgen -W "--name --intermediate" -- "$cur"))
                    return
                    ;;
                *)
                    if [[ $has_method -eq 0 ]]; then
                        COMPREPLY=($(compgen -W "--name --intermediate $add_methods" -- "$cur"))
                    else
                        # After method, show method-specific options
                        COMPREPLY=($(compgen -W "--secondary-pw" -- "$cur"))
                    fi
                    return
                    ;;
            esac
            ;;
        
        set-session)
            case "$prev" in
                --cmd)
                    # Complete commands
                    COMPREPLY=($(compgen -c -- "$cur"))
                    return
                    ;;
                --args)
                    # Complete files for arguments
                    _filedir
                    return
                    ;;
                *)
                    COMPREPLY=($(compgen -W "--cmd --args" -- "$cur"))
                    return
                    ;;
            esac
            ;;
        
        set-home-mount)
            case "$prev" in
                --device)
                    # Complete block devices
                    COMPREPLY=($(compgen -W "$(ls /dev/sd* /dev/nvme* /dev/mapper/* 2>/dev/null)" -- "$cur"))
                    return
                    ;;
                --fstype)
                    # Complete filesystem types
                    COMPREPLY=($(compgen -W "ext4 ext3 ext2 btrfs xfs f2fs ntfs vfat exfat" -- "$cur"))
                    return
                    ;;
                --flags)
                    # Complete mount flags
                    COMPREPLY=($(compgen -W "rw ro nosuid nodev noexec relatime noatime user_xattr acl" -- "$cur"))
                    return
                    ;;
                *)
                    COMPREPLY=($(compgen -W "--device --fstype --flags" -- "$cur"))
                    return
                    ;;
            esac
            ;;
        
        set-pre-mount)
            case "$prev" in
                --dir)
                    # Complete directories
                    _filedir -d
                    return
                    ;;
                --device)
                    # Complete block devices
                    COMPREPLY=($(compgen -W "$(ls /dev/sd* /dev/nvme* /dev/mapper/* 2>/dev/null)" -- "$cur"))
                    return
                    ;;
                --fstype)
                    # Complete filesystem types
                    COMPREPLY=($(compgen -W "ext4 ext3 ext2 btrfs xfs f2fs ntfs vfat exfat nfs cifs" -- "$cur"))
                    return
                    ;;
                --flags)
                    # Complete mount flags
                    COMPREPLY=($(compgen -W "rw ro nosuid nodev noexec relatime noatime user_xattr acl" -- "$cur"))
                    return
                    ;;
                *)
                    COMPREPLY=($(compgen -W "--dir --device --fstype --flags" -- "$cur"))
                    return
                    ;;
            esac
            ;;
        
        mount)
            # Check if we have a subcommand
            local mount_subcmd=""
            for ((j=cmd_pos+1; j < cword; j++)); do
                if [[ "${words[j]}" == "authorize" ]]; then
                    mount_subcmd="${words[j]}"
                    break
                fi
            done

            if [[ -z "$mount_subcmd" ]]; then
                # Suggest mount subcommands
                COMPREPLY=($(compgen -W "$mount_cmds" -- "$cur"))
            else
                # Mount authorize has no specific options (uses global -u)
                return
            fi
            ;;
    esac
}

complete -F _polyauthctl polyauthctl

