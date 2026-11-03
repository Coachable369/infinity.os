#!/bin/sh

# ------------------------=
# FUNC: resolve_vboxmanage
# DESC: Honor an explicit executable, then PATH, then the standard macOS VirtualBox app bundle without changing shell configuration.
# ------------------=
resolve_vboxmanage() {
    if test -n "${1:-}"; then
        command -v "$1" || return 1
        return 0
    fi
    if command -v VBoxManage >/dev/null 2>&1; then
        command -v VBoxManage
        return 0
    fi
    vbox_bundle=${2:-/Applications/VirtualBox.app}
    if test -x "$vbox_bundle/Contents/MacOS/VBoxManage"; then
        printf '%s\n' "$vbox_bundle/Contents/MacOS/VBoxManage"
        return 0
    fi
    return 1
}
