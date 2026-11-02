#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR=build/behavior-harness
mkdir -p build/behavior-tests
rustc --edition=2021 --test tools/accounts-ui-state-test.rs -o build/behavior-tests/accounts-ui-state-test
build/behavior-tests/accounts-ui-state-test
for suite in accounts-test accounts-storage-test accounts-cli-test milestone-7-test desktop-layout-identity-test settings-reboot-test; do
    cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin "$suite"
done
