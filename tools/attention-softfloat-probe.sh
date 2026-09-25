#!/bin/sh
# Controlled instruction-emulation diagnostic. Not installed VM or UI timings.
set -eu
cd "$(dirname "$0")/.."
probe=build/attention-softfloat-probe
mkdir -p "$probe"
llvm=${LLVM:-/opt/homebrew/opt/llvm/bin}
ld=${LD_LLD:-/opt/homebrew/opt/lld/bin/ld.lld}
"$llvm/clang" --target=aarch64-none-elf -ffreestanding -fno-builtin -ffp-contract=off -O3 -c tools/attention-softfloat-probe.c -o "$probe/probe.o"
"$llvm/clang" --target=aarch64-none-elf -ffreestanding -fno-builtin -ffp-contract=off -O3 -c kernel/runtime/ai/qwen/cpu_math.c -o "$probe/math.o"
"$ld" -nostdlib --gc-sections -T tools/attention-softfloat-probe.ld "$probe/probe.o" "$probe/math.o" build/aarch64/libinstalled-kernel.a -o "$probe/probe.elf"
python3 - "$probe/probe.elf" "${ATTENTION_ACCEL:-tcg}" <<'PY'
import subprocess
import sys
accel = sys.argv[2]
assert accel in ("tcg", "hvf")
subprocess.run(["qemu-system-aarch64", "-machine", "virt", "-cpu",
                "host" if accel == "hvf" else "cortex-a72", "-accel", accel,
                "-m", "256M", "-nographic", "-monitor", "none",
                "-semihosting-config", "enable=on,target=native", "-kernel", sys.argv[1]],
               check=True, timeout=30)
PY
