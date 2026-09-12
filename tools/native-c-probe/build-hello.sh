#!/bin/sh
# Cross-compilation bootstrap only. This is not an on-device cc implementation.
set -eu
cd "$(dirname "$0")/../.."
clang_path=${INFINITY_CLANG:-/opt/homebrew/opt/llvm/bin/clang}
linker_path=${INFINITY_LLD:-/opt/homebrew/opt/lld/bin/ld.lld}
mkdir -p build/native-c
for architecture in x86_64 aarch64; do
    architecture_flags=-mno-red-zone
    if test "$architecture" = aarch64; then architecture_flags=-mno-outline-atomics; fi
    "$clang_path" --target="$architecture-none-elf" -std=c17 -ffreestanding -fno-builtin \
        -fno-stack-protector -fno-unwind-tables -fno-asynchronous-unwind-tables -fPIE \
        "$architecture_flags" -Wall -Wextra -Werror -Os -Isdk/c/include \
        -c sdk/c/runtime.c -o "build/native-c/runtime-$architecture.o"
    "$clang_path" --target="$architecture-none-elf" -std=c17 -ffreestanding -fno-builtin \
        -fno-stack-protector -fno-unwind-tables -fno-asynchronous-unwind-tables -fPIE \
        "$architecture_flags" -Wall -Wextra -Werror -Os -Isdk/c/include \
        -c sdk/c/examples/hello.c -o "build/native-c/hello-$architecture.o"
    "$linker_path" -static -pie --no-dynamic-linker --build-id=none -z max-page-size=4096 \
        -T sdk/c/application.ld "build/native-c/runtime-$architecture.o" "build/native-c/hello-$architecture.o" \
        -o "build/native-c/hello-$architecture.elf"
done
