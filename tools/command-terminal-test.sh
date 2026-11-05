#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test "${INFINITY_BUILD_KIT_ACTIVE:-0}" = 1
mkdir -p build/behavior-tests
rustc --edition=2021 tools/command-terminal-test.rs -o build/behavior-tests/command-terminal
build/behavior-tests/command-terminal
cargo test --manifest-path kernel/runtime/ssh/Cargo.toml client_tests
