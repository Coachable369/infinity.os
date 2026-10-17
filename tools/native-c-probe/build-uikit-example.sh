#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
output=build/native-c/uikit-example
mkdir -p "$output"
compiler=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}/clang
for architecture in x86_64 aarch64; do
  cpu=-mno-red-zone
  [ "$architecture" = aarch64 ] && cpu=-mno-outline-atomics
  "$compiler" --target="$architecture-none-elf" -std=c17 -ffreestanding -fno-builtin \
    -fPIE "$cpu" -Wall -Wextra -Werror -Os -Isdk/c/include \
    -c sdk/c/examples/calculator-ui.c -o "$output/calculator-ui-$architecture.o"
done
