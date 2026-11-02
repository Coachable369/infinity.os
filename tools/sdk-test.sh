#!/bin/sh
set -eu
python3 tools/package-sdk.py
mkdir -p build/tests
rustc --edition=2021 -O -A warnings tools/object-store-test.rs -o build/tests/sdk-object-test
build/tests/sdk-object-test --sdk-only
python3 tools/sdk-package-test.py
python3 tools/sdk-examples-test.py
cc -std=c11 -Wall -Wextra -Werror -Isdk/c/include tools/sdk-services-test.c -o build/tests/sdk-services-test
python3 tools/sdk-services-test.py
rustc --edition=2021 tools/native-development-command-test.rs -o build/tests/sdk-command-test
build/tests/sdk-command-test
for arch in aarch64 x86_64; do
  for example in logging_demo network_head; do
    "${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}/clang" --target="$arch-none-elf" -std=c17 -ffreestanding -fno-builtin -Wall -Wextra -Werror -Isdk/c/include -Isdk/c/compiler-headers -c "sdk/c/examples/$example.c" -o "build/tests/$example-$arch.o"
  done
done
