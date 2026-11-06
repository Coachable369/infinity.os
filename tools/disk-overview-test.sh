#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test "$(printenv INFINITY_BUILD_KIT_ACTIVE)" = 1
mkdir -p build/behavior-tests
rustc --edition=2021 tools/disk-overview-test.rs -o build/behavior-tests/disk-overview
build/behavior-tests/disk-overview
