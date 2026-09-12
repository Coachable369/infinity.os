#!/bin/sh
# Executes the patched upstream native branch using host LLVM support libraries.
set -eu
cd "$(dirname "$0")/../.."
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
mkdir -p build/native-c
"$LLVM_BIN/clang++" $("$LLVM_BIN/llvm-config" --cxxflags) -UNDEBUG \
  -Ibuild/native-c/llvm-project-ea7d852a70e8bdfaf601d6626a760f9771b2c4b4/llvm/lib \
  -Ibuild/native-c/clang-build-infinity-x86_64/include \
  tools/native-c-probe/host-triple-test.cpp \
  $("$LLVM_BIN/llvm-config" --ldflags --libs support targetparser --system-libs) \
  -o build/native-c/host-triple-test
build/native-c/host-triple-test
