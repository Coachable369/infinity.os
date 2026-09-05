#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
rustc --edition=2021 tools/icon-theme-test.rs -o target/icon-theme-test
target/icon-theme-test
