#!/bin/sh
set -eu
mkdir -p build/tests
rustc --edition=2021 -O tools/network-test.rs -o build/tests/network-test
build/tests/network-test
