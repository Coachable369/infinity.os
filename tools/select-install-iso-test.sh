#!/bin/sh
set -eu
. "$(dirname "$0")/select-install-iso.sh"
fixture=$(mktemp -d)
fixture=$(CDPATH= cd -- "$fixture" && pwd -P)
trap 'rm -r -- "$fixture"' EXIT
mkdir -p "$fixture/builds" "$fixture/build/hermes"
printf legacy > "$fixture/build/hermes/old.iso"
# Never fall back to legacy media or accept an explicit external installer.
if select_install_iso "$fixture" >/dev/null 2>&1; then exit 1; fi
if select_install_iso "$fixture" "$fixture/build/hermes/old.iso" >/dev/null 2>&1; then exit 1; fi
touch "$fixture/builds/InfinityOS-aarch64.iso"
if select_install_iso "$fixture" >/dev/null 2>&1; then exit 1; fi
printf canonical > "$fixture/builds/InfinityOS-aarch64.iso"
chosen=$(select_install_iso "$fixture")
test "$chosen" = "$fixture/builds/InfinityOS-aarch64.iso"
cmp "$chosen" "$fixture/builds/InfinityOS-aarch64.iso"
chosen=$(select_install_iso "$fixture" "$fixture/builds/../builds/InfinityOS-aarch64.iso")
test "$chosen" = "$fixture/builds/InfinityOS-aarch64.iso"
ln -s "$fixture/build/hermes/old.iso" "$fixture/builds/escape.iso"
if select_install_iso "$fixture" "$fixture/builds/escape.iso" >/dev/null 2>&1; then exit 1; fi
