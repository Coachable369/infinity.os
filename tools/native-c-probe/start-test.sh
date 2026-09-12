#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
/opt/homebrew/opt/llvm/bin/clang -Wall -Wextra -Werror -Isdk/compiler/include \
  sdk/compiler/start.c sdk/compiler/serial_tls.c tools/native-c-probe/start-test.c -o build/native-c/start-test
build/native-c/start-test
for unit in start entry platform dlfcn serial_tls; do
  /opt/homebrew/opt/llvm/bin/clang --target=x86_64-unknown-elf \
    --sysroot=build/native-c/sysroot/x86_64-unknown-elf \
    -include sdk/compiler/target.h -Isdk/compiler/include -ffreestanding \
    -fPIE -mno-red-zone -c sdk/compiler/$unit.c -o build/native-c/compiler-$unit.o
done
PYTHONPATH= python3 - <<'PY'
import struct
from pathlib import Path
objects = list(Path('build/native-c/clang-build-infinity-x86_64/lib/Support/BLAKE3/CMakeFiles/LLVMSupportBlake3.dir').glob('*.S.o'))
assert len(objects) >= 4
for path in objects:
    data = path.read_bytes()
    assert data[:6] == b'\x7fELF\x02\x01', path
    assert struct.unpack_from('<HH', data, 16) == (1, 62), path
PY
