#!/bin/sh
set -eu
. ./tools/require-build-kit.sh
mkdir -p builds/validation/studio-menu
rustc --edition=2021 -O tools/studio-menu-native-test.rs -o builds/validation/studio-menu/native-test
builds/validation/studio-menu/native-test
