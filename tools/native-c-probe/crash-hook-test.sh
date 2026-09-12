#!/bin/sh
# Explicit provider contract test, not installed kernel fault recovery.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror \
  -Isdk/compiler/include sdk/compiler/platform.c tools/native-c-probe/crash-hook-test.c \
  -o build/native-c/crash-hook-test
build/native-c/crash-hook-test
