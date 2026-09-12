#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dsbrk=infinity_test_sbrk sdk/compiler/heap.c tools/native-c-probe/heap-test.c \
  -o build/native-c/heap-test
build/native-c/heap-test
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
  -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
  -c sdk/compiler/heap.c -o build/native-c/compiler-heap.o
