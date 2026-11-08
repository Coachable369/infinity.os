#!/bin/sh
set -eu
. tools/require-build-kit.sh
mkdir -p build/behavior-tests
rustc --edition=2021 -Awarnings tools/automation-test.rs -o build/behavior-tests/automation-test
build/behavior-tests/automation-test
rustc --edition=2021 -Awarnings tools/automation-worker-test.rs -o build/behavior-tests/automation-worker-test
build/behavior-tests/automation-worker-test
rustc --edition=2021 -Awarnings tools/remote-keys-test.rs -o build/behavior-tests/remote-keys-test
build/behavior-tests/remote-keys-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin automation-console-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin console-listing-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin enterprise-access-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin task-automation-controller-test
rustc --edition=2021 -Awarnings tools/active-painter-test.rs -o build/behavior-tests/task-automation-painter-test
build/behavior-tests/task-automation-painter-test --task-management-only
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin automation-storage-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin llm-advice-test
