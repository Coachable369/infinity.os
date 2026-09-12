#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror -Isdk/compiler/include \
  sdk/compiler/serial_sync.c sdk/compiler/serial_tls.c tools/native-c-probe/serial-sync-test.c -o build/native-c/serial-sync-test
build/native-c/serial-sync-test
for unit in serial_sync serial_tls pthread; do
  /opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
    --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
    -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
    -c sdk/compiler/$unit.c -o build/native-c/compiler-$unit.o
done
