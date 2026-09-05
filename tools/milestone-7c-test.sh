#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p build/behavior-tests
rustc --edition=2021 -Awarnings tools/infinity-ui-test.rs -o build/behavior-tests/milestone-7c-ui
build/behavior-tests/milestone-7c-ui
tools/runtime-test.sh

RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/check-7c-x86_64 \
    cargo check -q -Z build-std=core --target x86_64-unknown-none --features installer
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/check-7c-aarch64 \
    cargo check -q -Z build-std=core --target aarch64-unknown-none --features installer

echo "Milestone 7C architecture-neutral behavior and target compilation: PASS"
