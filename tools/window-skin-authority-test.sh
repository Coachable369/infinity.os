#!/bin/sh
set -eu
test -n "${INFINITY_BUILD_KIT_ACTIVE:-}"
mkdir -p build/behavior-tests
rustc --edition=2021 -Awarnings --cfg studio_native_preview tools/window-skin-authority-test.rs -o build/behavior-tests/window-skin-authority-test
build/behavior-tests/window-skin-authority-test
rustc --edition=2021 -Awarnings tools/active-painter-test.rs -o build/behavior-tests/window-skin-painter
build/behavior-tests/window-skin-painter --universal-frames-only
build/behavior-tests/window-skin-painter --window-controls-only
