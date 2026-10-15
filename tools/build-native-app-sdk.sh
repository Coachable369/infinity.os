#!/bin/sh
# Produce the same small isolated application CRT for both installed targets.
set -eu
cd "$(dirname "$0")/.."
. tools/require-build-kit.sh
architecture=${1:?architecture required}
case "$architecture" in
  aarch64) cpu=-mno-outline-atomics; work=build/native-c/aarch64 ;;
  x86_64) cpu=-mno-red-zone; work=build/native-c ;;
  *) exit 2 ;;
esac
llvm=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
linker=${LD_LLD:-/opt/homebrew/bin/ld.lld}
destination=builds/toolchains/$architecture
scratch=$work/application-sdk
mkdir -p "$scratch" "$destination"
for unit in runtime libc format stdio isolated; do
  "$llvm/clang" --target="$architecture-none-elf" -ffreestanding -fno-stack-protector \
    -fno-unwind-tables -fno-asynchronous-unwind-tables -fPIE "$cpu" -Os \
    -Isdk/c/include -isystem "$work/sysroot/$architecture-unknown-elf/include" \
    -c "sdk/c/$unit.c" -o "$scratch/$unit.o"
done
"$linker" -r "$scratch/runtime.o" "$scratch/libc.o" "$scratch/format.o" "$scratch/stdio.o" "$scratch/isolated.o" -o "$destination/application-crt.o.part"
mv "$destination/application-crt.o.part" "$destination/application-crt.o"
(cd "$destination" && shasum -a 256 clang.elf lld.elf application-crt.o > SHA256SUMS)
