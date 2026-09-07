#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
    --manifest-path tools/behavior-harness/Cargo.toml --bin runtime-test
