#!/bin/sh
set -eu
mkdir -p build/tests
rustc --edition=2021 -O tools/object-store-test.rs -o build/tests/object-store-test
build/tests/object-store-test
rustc --edition=2021 -O tools/metadata-expansion-test.rs -o build/tests/metadata-expansion-test
build/tests/metadata-expansion-test
