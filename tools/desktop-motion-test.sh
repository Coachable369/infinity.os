#!/bin/sh
set -eu
test -n "${INFINITY_BUILD_KIT_ACTIVE:-}"
mkdir -p build/behavior-tests
rustc --edition=2021 --test -Awarnings tools/drop-compass-test.rs -o build/behavior-tests/drop-compass-test
build/behavior-tests/drop-compass-test
rustc --edition=2021 -Awarnings -C opt-level=z tools/active-painter-test.rs -o build/behavior-tests/desktop-motion-painter
build/behavior-tests/desktop-motion-painter --desktop-drag-only
build/behavior-tests/desktop-motion-painter --motion-benchmark-only
