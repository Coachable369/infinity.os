#!/bin/sh
# Host LLVM compatibility behavior, not installed native networking acceptance.
set -eu
cd "$(dirname "$0")/../.."
LLVM_BIN=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
mkdir -p build/native-c
"$LLVM_BIN/clang++" $("$LLVM_BIN/llvm-config" --cxxflags) -UNDEBUG \
  -Isdk/compiler/include tools/native-c-probe/socket-unavailable-test.cpp \
  $("$LLVM_BIN/llvm-config" --ldflags --libs support --system-libs) \
  -o build/native-c/socket-unavailable-test
build/native-c/socket-unavailable-test
