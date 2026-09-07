#!/bin/sh
set -eu

# ------------------------=
# FUNC: run_milestone9_harness
# DESC: Runs the behavioral two-node trust harness and fails on any state-transition assertion.
# ------------------=
run_milestone9_harness() {
    CARGO_TARGET_DIR=build/milestone9-harness cargo run --quiet --manifest-path tools/milestone9-harness/Cargo.toml
}

run_milestone9_harness
