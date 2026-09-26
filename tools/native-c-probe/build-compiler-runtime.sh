#!/bin/sh
# Builds the native service bridge objects consumed by the Clang and LLD links.
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
compiler=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}/clang
sysroot=${INFINITY_COMPILER_SYSROOT:-build/native-c/sysroot/x86_64-unknown-elf}
if test ! -f "$sysroot/lib/libc.a"; then
  echo "native compiler sysroot is missing: $sysroot" >&2
  exit 2
fi
mkdir -p build/native-c
for unit in start entry platform dlfcn files heap time serial_sync serial_tls pthread namespace process; do
  "$compiler" --target=x86_64-unknown-elf --sysroot="$sysroot" \
    -std=c17 -O2 -fPIE -mno-red-zone -ffunction-sections -fdata-sections \
    -Wall -Wextra -Werror -include sdk/compiler/target.h -Isdk/compiler/include \
    -c "sdk/compiler/$unit.c" -o "build/native-c/compiler-$unit.o"
done
