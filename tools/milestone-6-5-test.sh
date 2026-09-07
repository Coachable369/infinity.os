#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p build
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
    --manifest-path tools/behavior-harness/Cargo.toml --bin milestone-6-5-test
