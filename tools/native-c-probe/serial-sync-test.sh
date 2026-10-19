#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror -Isdk/compiler/include \
  sdk/compiler/serial_sync.c sdk/compiler/serial_tls.c tools/native-c-probe/serial-sync-test.c -o build/native-c/serial-sync-test
build/native-c/serial-sync-test
set -- --sysroot=build/native-c/sysroot/x86_64-unknown-elf
if ! test -f build/native-c/sysroot/x86_64-unknown-elf/include/errno.h; then
  set -- -isystem build/voice-newlib-x86_64/x86_64-none-elf/newlib \
    -isystem build/newlib-4.6.0.20260123/newlib/libc/include
fi
for unit in serial_sync serial_tls pthread; do
  /opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
    "$@" -include sdk/compiler/target.h \
    -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
    -c sdk/compiler/$unit.c -o build/native-c/compiler-$unit.o
done
