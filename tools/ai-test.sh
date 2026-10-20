#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
    --manifest-path tools/behavior-harness/Cargo.toml --bin ai-test
CARGO_TARGET_DIR=build/behavior-harness RUST_MIN_STACK=33554432 cargo test --quiet --release \
    --manifest-path tools/behavior-harness/Cargo.toml --bin ai-test inquiry_tests
CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --release \
    --manifest-path tools/behavior-harness/Cargo.toml --bin hermes-native-test native_swiglu_matches_reference
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release \
    --manifest-path tools/behavior-harness/Cargo.toml --bin settings-reboot-test
