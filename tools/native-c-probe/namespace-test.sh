#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dgetcwd=infinity_test_getcwd -Drealpath=infinity_test_realpath -Dchdir=infinity_test_chdir \
  -Dmkdir=infinity_test_mkdir -Dunlink=infinity_test_unlink -Dlink=infinity_test_link \
  -Dsymlink=infinity_test_symlink -Dreadlink=infinity_test_readlink \
  sdk/compiler/namespace.c tools/native-c-probe/namespace-test.c -o build/native-c/namespace-test
build/native-c/namespace-test
set -- --sysroot=build/native-c/sysroot/x86_64-unknown-elf
if ! test -f build/native-c/sysroot/x86_64-unknown-elf/include/unistd.h; then
  set -- -isystem build/voice-newlib-x86_64/x86_64-none-elf/newlib \
    -isystem build/newlib-4.6.0.20260123/newlib/libc/include
fi
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  "$@" -include sdk/compiler/target.h \
  -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
  -c sdk/compiler/namespace.c -o build/native-c/compiler-namespace.o
