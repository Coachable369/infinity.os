#!/bin/sh
set -eu

mkdir -p build/tools
rustc -Awarnings --edition 2021 tools/settings-overflow-test.rs -o build/tools/settings-overflow-test
build/tools/settings-overflow-test
echo "Settings bounded overflow behavior: PASS"
