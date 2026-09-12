#!/bin/sh
# Host behavioral API tests plus target compilation; not native loader acceptance.
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/native-c
/opt/homebrew/opt/llvm/bin/clang -std=c17 -Wall -Wextra -Werror -pthread \
  -Isdk/compiler/include -Ddlopen=infinity_test_dlopen \
  -Ddlsym=infinity_test_dlsym -Ddlclose=infinity_test_dlclose \
  -Ddlerror=infinity_test_dlerror -Ddladdr=infinity_test_dladdr \
  sdk/compiler/dlfcn.c tools/native-c-probe/dlfcn-test.c \
  -o build/native-c/dlfcn-test
build/native-c/dlfcn-test
/opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
  --sysroot=build/native-c/sysroot/x86_64-unknown-elf \
  -std=c17 -Wall -Wextra -Werror -ffreestanding -fPIE -mno-red-zone \
  -include sdk/compiler/target.h -Isdk/compiler/include \
  -c sdk/compiler/dlfcn.c -o build/native-c/dlfcn-x86_64.o
