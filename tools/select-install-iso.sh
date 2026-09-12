#!/bin/sh
# ------------------------=
# FUNC: select_install_iso
# DESC: Resolves the model-enabled default without silently falling back to a legacy ISO.
# ------------------=
select_install_iso() {
    selected_iso=${2:-$1/build/qwen/InfinityOS-Qwen3-8B-aarch64.iso}
    if ! test -s "$selected_iso"; then
        printf 'ERROR: installer missing: %s. Run sh tools/build-qwen.sh first.\n' "$selected_iso" >&2
        return 1
    fi
    printf '%s\n' "$selected_iso"
}
