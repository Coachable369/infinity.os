#!/bin/sh
# Builds the native service bridge objects consumed by the Clang and LLD links.
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
compiler=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}/clang
target=${INFINITY_COMPILER_TARGET:-x86_64-unknown-elf}
output=${INFINITY_COMPILER_OUTPUT:-build/native-c}
case "$target" in
  x86_64-unknown-elf) cpu_flags=-mno-red-zone ;;
  aarch64-unknown-elf) cpu_flags=-mno-outline-atomics ;;
  *) echo "unsupported compiler runtime target: $target" >&2; exit 2 ;;
esac
sysroot=${INFINITY_COMPILER_SYSROOT:-build/native-c/sysroot/x86_64-unknown-elf}
if test ! -f "$sysroot/lib/libc.a"; then
  echo "native compiler sysroot is missing: $sysroot" >&2
  exit 2
fi
mkdir -p "$output"
for unit in start entry platform dlfcn files heap time serial_sync serial_tls pthread namespace process newlib; do
  "$compiler" --target="$target" --sysroot="$sysroot" \
    -std=c17 -O2 -fPIE "$cpu_flags" -ffunction-sections -fdata-sections \
    -Wall -Wextra -Werror -include sdk/compiler/target.h -Isdk/compiler/include \
    -c "sdk/compiler/$unit.c" -o "$output/compiler-$unit.o"
done
