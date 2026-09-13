#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
for architecture in x86_64 aarch64; do
  /opt/homebrew/opt/llvm/bin/clang --target="$architecture-none-elf" \
    -std=c17 -ffreestanding -fno-stack-protector -fno-unwind-tables \
    -fno-asynchronous-unwind-tables -fPIE -ftls-model=local-exec -Os \
    -c tools/native-c-probe/tls-app.c -o "build/native-c/tls-$architecture.o"
  /opt/homebrew/opt/lld/bin/ld.lld -static -pie --strip-all --no-dynamic-linker \
    --build-id=none -z max-page-size=4096 -T sdk/c/application.ld \
    "build/native-c/tls-$architecture.o" -o "build/native-c/tls-$architecture.elf"
done
