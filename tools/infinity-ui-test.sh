#!/bin/sh
set -eu

mkdir -p build/tools
rustc --edition 2021 tools/infinity-ui-test.rs -o build/tools/infinity-ui-test
build/tools/infinity-ui-test
test -s assets/desktop/infinity-default-dark-wallpaper-v2.bmp
test -s assets/skins/infinity.default.dark/infinity.default.dark.iskin
rg -q 'fn authentication_frame' kernel/core/bootstrap.rs
rg -q 'infinity-default-dark-wallpaper-v2\.bmp' kernel/core/bootstrap.rs Makefile
rg -q 'cp -R assets/skins' Makefile
rg -q 'SERVICE_INFINITY_UI' kernel/runtime/service.rs kernel/runtime/mod.rs
rg -q 'AppearanceSetSkin' kernel/runtime/iop.rs kernel/runtime/console_language.rs
echo "InfinityUI integration and fresh-install parity: PASS"
