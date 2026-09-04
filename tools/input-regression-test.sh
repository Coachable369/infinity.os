#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p build/behavior-tests
rustc --edition=2021 -C opt-level=2 tools/pointer-protocol-test.rs -o build/behavior-tests/pointer-protocol-test
build/behavior-tests/pointer-protocol-test
rustc --edition=2021 -C opt-level=2 tools/ui-redraw-policy-test.rs -o build/behavior-tests/ui-redraw-policy-test
build/behavior-tests/ui-redraw-policy-test
