#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
for unit in serial_sync serial_tls pthread; do
  /opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
    --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
    -Isdk/compiler/include -ffreestanding -fno-builtin -fPIE -mno-red-zone -Os \
    -c sdk/compiler/$unit.c -o build/native-c/sync-$unit.o
done
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
  -Isdk/compiler/include -ffreestanding -fno-builtin -fPIE -mno-red-zone -Os \
  -c tools/native-c-probe/sync-app.c -o build/native-c/sync-app.o
/opt/homebrew/opt/lld/bin/ld.lld -static -pie --strip-all --no-dynamic-linker \
  --build-id=none -z max-page-size=4096 -T sdk/c/application.ld \
  build/native-c/sync-app.o build/native-c/sync-serial_sync.o \
  build/native-c/sync-serial_tls.o build/native-c/sync-pthread.o \
  -o build/native-c/sync-x86_64.elf
