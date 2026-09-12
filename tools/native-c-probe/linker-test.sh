#!/bin/sh
# Cross-link artifact verification, not native execution acceptance.
set -eu
cd "$(dirname "$0")/../.."
build/native-c/build-tools/bin/cmake \
  -S tools/native-c-probe/linker-fixture -B build/native-c/linker-fixture \
  -DCMAKE_SYSTEM_NAME=InfinityOS \
  -DCMAKE_MODULE_PATH="$PWD/sdk/compiler/cmake" \
  -DCMAKE_C_COMPILER=/opt/homebrew/opt/llvm/bin/clang \
  -DCMAKE_C_COMPILER_TARGET=x86_64-unknown-elf \
  -DCMAKE_LINKER=/opt/homebrew/bin/ld.lld \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
  -DCMAKE_EXE_LINKER_FLAGS=
build/native-c/build-tools/bin/cmake --build build/native-c/linker-fixture
python3 - <<'PY'
import struct
from pathlib import Path
data = Path('build/native-c/linker-fixture/probe.elf').read_bytes()
assert data[:6] == b'\x7fELF\x02\x01'
kind, machine = struct.unpack_from('<HH', data, 16)
assert kind == 2 and machine == 62
assert struct.unpack_from('<Q', data, 24)[0] != 0
PY
