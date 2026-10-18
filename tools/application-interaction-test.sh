#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
. tools/require-build-kit.sh
rustc --edition=2021 -A warnings tools/application-interaction-test.rs -o build/application-interaction-test
build/application-interaction-test
rustc --edition=2021 tools/native-compile-cancel-test.rs -o build/native-compile-cancel-test
build/native-compile-cancel-test
rustc --edition=2021 --test -A dead_code tools/native-tokenizer-test.rs -o build/native-tokenizer-test
build/native-tokenizer-test
mkdir -p build/behavior-tests
cc -std=c11 -Wall -Wextra -Werror -Isdk/c/include tools/native-app-sdk-test.c \
  -o build/behavior-tests/native-app-sdk-test
build/behavior-tests/native-app-sdk-test
compiler=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}/clang
for architecture in x86_64 aarch64; do
  "$compiler" --target="$architecture-none-elf" -std=c17 -ffreestanding -fno-builtin \
    -Wall -Wextra -Werror -Isdk/c/include -c tools/application-interaction-abi-test.c \
    -o "build/application-interaction-abi-$architecture.o"
done
