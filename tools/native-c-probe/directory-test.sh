#!/bin/sh
# Host adapter tests; callbacks are controlled fixtures, not installed providers.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror \
  -Isdk/compiler/include sdk/compiler/platform.c tools/native-c-probe/directory-test.c \
  -o build/native-c/directory-test
build/native-c/directory-test
