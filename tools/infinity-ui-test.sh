#!/bin/sh
set -eu

mkdir -p build/tools
rustc --edition 2021 tools/infinity-ui-test.rs -o build/tools/infinity-ui-test
build/tools/infinity-ui-test
test -s assets/desktop/infinity-default-dark-wallpaper-v2.bmp
test -s assets/skins/infinity.default.dark/infinity.default.dark.iskin
echo "InfinityUI integration and fresh-install parity: PASS"
