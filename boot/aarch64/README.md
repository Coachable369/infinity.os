# AArch64 bootstrap

The AArch64 target enters through UEFI, shares only firmware-neutral loading
logic with x86_64, then uses `handoff.S` to validate the firmware's EL1 or EL2
entry state, mask exceptions, establish the stack, and branch to the Rust kernel
using AAPCS64.
