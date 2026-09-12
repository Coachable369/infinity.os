#!/bin/sh
# Host-side ABI contract test, not proof of native clocks or page tables.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror \
    -Isdk/compiler/include -Dclock_gettime=infinity_test_clock_gettime \
    -Dmmap=infinity_test_mmap -Dmunmap=infinity_test_munmap \
    -Dmprotect=infinity_test_mprotect -Dmsync=infinity_test_msync \
    -Dgetentropy=infinity_test_getentropy \
    sdk/compiler/platform.c tools/native-c-probe/compiler-platform-test.c \
    -o build/native-c/compiler-platform-test
build/native-c/compiler-platform-test
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
    --sysroot=build/native-c/sysroot/x86_64-unknown-elf \
    -std=c17 -Wall -Wextra -Werror -ffreestanding -fPIE -mno-red-zone \
    -include sdk/compiler/target.h -Isdk/compiler/include \
    -c sdk/compiler/platform.c -o build/native-c/compiler-platform-x86_64.o
