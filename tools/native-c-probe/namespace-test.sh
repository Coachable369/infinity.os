#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dgetcwd=infinity_test_getcwd -Drealpath=infinity_test_realpath -Dchdir=infinity_test_chdir \
  -Dmkdir=infinity_test_mkdir -Dunlink=infinity_test_unlink \
  sdk/compiler/namespace.c tools/native-c-probe/namespace-test.c -o build/native-c/namespace-test
build/native-c/namespace-test
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
  -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
  -c sdk/compiler/namespace.c -o build/native-c/compiler-namespace.o
