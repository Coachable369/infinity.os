#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p build/behavior-tests
rustc --edition=2021 -C opt-level=2 -A warnings tools/crash-screen-test.rs \
    -o build/behavior-tests/crash-screen-test
build/behavior-tests/crash-screen-test
