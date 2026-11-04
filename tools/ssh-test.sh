#!/bin/sh
# Run through ./build-kit run sh tools/ssh-test.sh (or the ssh-tests profile).
set -eu
cd "$(dirname "$0")/.."
test "${INFINITY_BUILD_KIT_ACTIVE:-0}" = 1
cargo test --manifest-path kernel/runtime/ssh/Cargo.toml
cargo test --manifest-path kernel/runtime/http/Cargo.toml sockets::
cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin ssh-identity-test
cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin ssh-storage-test
cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin ssh-console-test
# Lint only; this is not installed-system acceptance.
python3 -m py_compile tools/ssh-installed-test.py
