#!/bin/sh
set -eu
. tools/require-build-kit.sh
out=builds/.studio-work/desktop-refinement
mkdir -p "$out"
rustc --edition=2021 -Awarnings tools/app-launcher-interaction-test.rs -o "$out/launcher"
"$out/launcher"
rustc --edition=2021 -Awarnings --test tools/desktop-widgets-test.rs -o "$out/widgets"
"$out/widgets" --test-threads=1
python3 tools/memory-refinement-test.py
