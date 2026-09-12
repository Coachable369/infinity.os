#!/bin/sh
# VFS fixture behavior, not installed ObjectStore provider acceptance.
set -eu
cd "$(dirname "$0")/../.."
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
mkdir -p build/native-c
"$LLVM_BIN/clang++" $("$LLVM_BIN/llvm-config" --cxxflags) -UNDEBUG \
  -Isdk/compiler/include tools/native-c-probe/seed-file-test.cpp \
  $("$LLVM_BIN/llvm-config" --ldflags --libs support --system-libs) \
  -o build/native-c/seed-file-test
build/native-c/seed-file-test
