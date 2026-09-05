#!/bin/sh
set -eu

mkdir -p build/tests
rustc --edition=2021 -O \
    tools/installer-capacity-test.rs -o build/tests/installer-capacity-test
build/tests/installer-capacity-test
