#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  -Dgettimeofday=infinity_test_gettimeofday -Dnanosleep=infinity_test_nanosleep \
  -Dusleep=infinity_test_usleep -Dgetpagesize=infinity_test_getpagesize \
  sdk/compiler/time.c tools/native-c-probe/time-test.c -o build/native-c/time-test
build/native-c/time-test
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  --sysroot=build/native-c/sysroot/x86_64-unknown-elf -include sdk/compiler/target.h \
  -Isdk/compiler/include -ffreestanding -fPIE -mno-red-zone -Wall -Wextra -Werror \
  -c sdk/compiler/time.c -o build/native-c/compiler-time.o
