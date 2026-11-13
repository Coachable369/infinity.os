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
rustc --edition=2021 -Awarnings -O tools/object-store-test.rs -o "$out/object-store"
"$out/object-store" --sdk-only
rustc --edition=2021 --crate-name infinity_browser_core --crate-type rlib -C opt-level=z sdk/infinity-browser-core/lib.rs -o "$out/libinfinity_browser_core.rlib"
rustc --edition=2021 -O -Awarnings --extern infinity_browser_core="$out/libinfinity_browser_core.rlib" tools/active-painter-test.rs -o "$out/painter"
"$out/painter" --desktop-drag-only
sips -s format png builds/validation/desktop-trash/trash.ppm --out builds/validation/desktop-trash/trash.png >/dev/null
