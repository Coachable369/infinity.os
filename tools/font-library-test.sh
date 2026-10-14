#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p build/tests
rustc --edition=2021 --test tools/font-library-test.rs -o build/tests/font-library-test
build/tests/font-library-test
