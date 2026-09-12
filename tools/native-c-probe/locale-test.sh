#!/bin/sh
# Native library probe; not native compilation or installed-system acceptance.
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang++ --target=x86_64-unknown-elf \
    --sysroot=build/native-c/sysroot/x86_64-unknown-elf \
    -nostdinc++ -isystem build/native-c/sysroot/x86_64-unknown-elf/include/c++/v1 \
    -include sdk/compiler/target.h -Isdk/compiler/include \
    -std=c++17 -O2 -ffreestanding -fno-exceptions -fno-rtti -mno-red-zone \
    -ffunction-sections -fdata-sections \
    -c tools/native-c-probe/locale-test.cpp -o build/native-c/locale-test.o
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static --gc-sections -T linker/x86_64.ld \
    -o build/native-c/locale-probe.elf build/native-c/locale-test.o \
    build/native-c/sysroot/x86_64-unknown-elf/lib/libc++.a \
    build/native-c/sysroot/x86_64-unknown-elf/lib/libc.a
python3 tools/native-c-probe/run.py --locale
