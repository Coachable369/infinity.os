#!/bin/sh
set -eu
. tools/require-build-kit.sh
out=builds/.studio-work/behavior-tests
mkdir -p "$out"
rustc --edition=2021 -Awarnings --test tools/minimized-shelf-test.rs -o "$out/task-tray-motion-test"
"$out/task-tray-motion-test" --test-threads=1
rustc --edition=2021 -O -Awarnings --test --extern infinity_browser_core=build/behavior-tests/libinfinity_browser_core.rlib tools/active-painter-test.rs -o "$out/task-tray-painter-test"
"$out/task-tray-painter-test" minimized_shelf_fixture::shelf_ --test-threads=1
