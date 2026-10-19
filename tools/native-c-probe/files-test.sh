#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dopen=infinity_test_open -Dread=infinity_test_read -Dwrite=infinity_test_write \
  -Dlseek=infinity_test_lseek -Dclose=infinity_test_close \
  -Dpread=infinity_test_pread -Dstat=infinity_test_stat -Dfstat=infinity_test_fstat -Daccess=infinity_test_access \
  -Dfcntl=infinity_test_fcntl -Dftruncate=infinity_test_ftruncate \
  sdk/compiler/files.c tools/native-c-probe/files-test.c -o build/native-c/files-test
build/native-c/files-test
set -- --sysroot=build/native-c/sysroot/x86_64-unknown-elf
if ! test -f build/native-c/sysroot/x86_64-unknown-elf/include/errno.h; then
  set -- -isystem build/voice-newlib-x86_64/x86_64-none-elf/newlib \
    -isystem build/newlib-4.6.0.20260123/newlib/libc/include
fi
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  "$@" \
  -include sdk/compiler/target.h -Isdk/compiler/include -ffreestanding \
  -Wall -Wextra -Werror -fPIE -mno-red-zone \
  -c sdk/compiler/files.c -o build/native-c/compiler-files.o
