#!/bin/sh
set -eu
test "${INFINITY_BUILD_KIT_ACTIVE:-}" != ""
mkdir -p build/behavior-tests
rustc --edition 2021 tools/clipboard-cursor-test.rs -o build/behavior-tests/clipboard-cursor-test
build/behavior-tests/clipboard-cursor-test
