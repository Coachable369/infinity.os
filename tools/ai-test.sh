#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
    --manifest-path tools/behavior-harness/Cargo.toml --bin ai-test
