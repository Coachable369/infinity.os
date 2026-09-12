#!/bin/sh
# Executes patched Clang parser behavior; not installed compilation acceptance.
set -eu
cd "$(dirname "$0")/../.."
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
LLVM_SOURCE=build/native-c/llvm-project-ea7d852a70e8bdfaf601d6626a760f9771b2c4b4
mkdir -p build/native-c
"$LLVM_BIN/clang++" -I"$LLVM_SOURCE/clang/include" \
  $("$LLVM_BIN/llvm-config" --cxxflags) -UNDEBUG \
  "$LLVM_SOURCE/clang/lib/Frontend/LayoutOverrideSource.cpp" \
  tools/native-c-probe/layout-reader-test.cpp \
  $("$LLVM_BIN/llvm-config" --ldflags --libs support --system-libs) -lclang-cpp \
  -o build/native-c/layout-reader-test
build/native-c/layout-reader-test
