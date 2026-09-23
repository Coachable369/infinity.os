#!/bin/sh
# ------------------------=
# FUNC: select_install_iso
# DESC: Resolves installer media only inside the repository's canonical builds directory.
# ------------------=
select_install_iso() {
    installer_dir=$(CDPATH= cd -- "$1/builds" 2>/dev/null && pwd -P) || {
        printf 'ERROR: builds directory missing. Run sh build.sh.\n' >&2
        return 1
    }
    selected_iso=${2:-$installer_dir/InfinityOS-aarch64.iso}
    selected_dir=$(CDPATH= cd -- "$(dirname -- "$selected_iso")" 2>/dev/null && pwd -P) || return 1
    if test "$selected_dir" != "$installer_dir" || test -L "$selected_iso"; then
        printf 'ERROR: installer must be a regular ISO in %s.\n' "$installer_dir" >&2
        return 1
    fi
    selected_iso=$installer_dir/$(basename -- "$selected_iso")
    case "$selected_iso" in *.iso) ;; *) return 1 ;; esac
    if ! test -f "$selected_iso" || ! test -s "$selected_iso"; then
        printf 'ERROR: installer missing or empty: %s. Run sh build.sh to build the installer, or supply a valid ISO path.\n' "$selected_iso" >&2
        return 1
    fi
    printf '%s\n' "$selected_iso"
}
