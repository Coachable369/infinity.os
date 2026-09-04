#!/bin/sh
set -eu
mkdir -p build/tests
rustc --edition=2021 -O tools/object-store-test.rs -o build/tests/object-store-test
build/tests/object-store-test
