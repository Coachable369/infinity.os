#!/bin/bash
# Fast ARM64 test ISO: reuse dependencies and skip regression tests.
# Packaging, fresh-install parity, and checksum gates remain mandatory.
set -euo pipefail

if [[ $# -eq 1 && ( "$1" == --help || "$1" == -h ) ]]; then
    echo "Usage: ./build-arm-iso.sh"
    echo "Builds builds/InfinityOS-aarch64.iso incrementally without regression tests."
    echo "Retains packaging, parity, checksum checks, and a build manifest."
    exit 0
fi
if [[ $# -ne 0 ]]; then
    echo "Usage: ./build-arm-iso.sh [--help]" >&2
    exit 2
fi

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$project_root/build-kit" arm-iso
