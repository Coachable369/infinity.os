"""Shared native speech cross-compilation target; never selects host libraries."""
import os

ARCH = os.environ.get("INFINITY_VOICE_TARGET", "aarch64")
if ARCH not in ("aarch64", "x86_64"):
    raise ValueError("Unsupported native speech architecture")
TRIPLE = ARCH + "-none-elf"
FLAGS = ["-mstrict-align"] if ARCH == "aarch64" else ["-mno-red-zone", "-mno-avx"]


# ------------------------=
# FUNC: syscall_aliases
# DESC: Maps the x86 newlib ABI onto the same native resource adapters used by ARM.
# ------------------=
def syscall_aliases(*names):
    return [f"--defsym={name}=_{name}" for name in names] if ARCH == "x86_64" else []
