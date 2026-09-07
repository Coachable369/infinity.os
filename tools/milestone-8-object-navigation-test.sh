#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
mkdir -p build/behavior-tests
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
    --manifest-path tools/behavior-harness/Cargo.toml --bin milestone-8-object-navigation-test
