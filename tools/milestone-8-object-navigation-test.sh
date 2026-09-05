#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
mkdir -p build/behavior-tests
rustc --edition=2021 -C opt-level=2 -A warnings \
    tools/milestone-8-object-navigation-test.rs \
    -o build/behavior-tests/milestone-8-object-navigation-test
build/behavior-tests/milestone-8-object-navigation-test
