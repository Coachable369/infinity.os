#!/bin/sh
set -eu
. "$(dirname "$0")/select-install-iso.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
# Verify failure even when the legacy export exists.
mkdir -p "$fixture/builds" "$fixture/build/qwen"
printf old > "$fixture/builds/InfinityOS-aarch64.iso"
if select_install_iso "$fixture" >/dev/null 2>&1; then exit 1; fi
printf models > "$fixture/build/qwen/InfinityOS-Qwen3-8B-aarch64.iso"
chosen=$(select_install_iso "$fixture")
cmp "$chosen" "$fixture/build/qwen/InfinityOS-Qwen3-8B-aarch64.iso"
chosen=$(select_install_iso "$fixture" "$fixture/builds/InfinityOS-aarch64.iso")
cmp "$chosen" "$fixture/builds/InfinityOS-aarch64.iso"
