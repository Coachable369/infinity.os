#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p build/behavior-tests
rustc --edition 2021 tools/firmware-key-test.rs -o build/behavior-tests/firmware-key-test
build/behavior-tests/firmware-key-test
rustc --edition 2021 --test kernel/ui/input_batch.rs -o build/behavior-tests/input-batch-test
build/behavior-tests/input-batch-test
rustc --edition 2021 --test kernel/drivers/input/buffer.rs -o build/behavior-tests/input-buffer-test
build/behavior-tests/input-buffer-test
for test_name in \
    pointer-protocol-test \
    ui-redraw-policy-test \
    window-session-state-test \
    file-navigator-layout-test \
    desktop-layout-identity-test \
    settings-overflow-test \
    task-manager-test \
    file-navigator-workspace-test \
    application-association-test
do
    CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
        --manifest-path tools/behavior-harness/Cargo.toml --bin "$test_name"
done
