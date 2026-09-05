#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p build/behavior-tests
rustc --edition=2021 -C opt-level=2 tools/ui-install-parity-test.rs -o build/behavior-tests/ui-install-parity-test
build/behavior-tests/ui-install-parity-test "$@"
