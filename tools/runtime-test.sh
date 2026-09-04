#!/bin/sh
set -eu
mkdir -p build/tests
rustc --edition=2021 -O tools/runtime-test.rs -o build/tests/runtime-test
build/tests/runtime-test
