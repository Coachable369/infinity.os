#!/bin/sh
set -eu
. tools/require-build-kit.sh
out=builds/.studio-work/password-confirmation
mkdir -p "$out"
rustc --edition=2021 --crate-name infinity_browser_core --crate-type rlib -C opt-level=z sdk/infinity-browser-core/lib.rs -o "$out/libinfinity_browser_core.rlib"
rustc --edition=2021 -O -Awarnings --extern infinity_browser_core="$out/libinfinity_browser_core.rlib" tools/active-painter-test.rs -o "$out/painter"
"$out/painter" --password-confirmation
