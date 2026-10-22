#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
out=build/camera-probe
mkdir -p "$out"
/opt/homebrew/opt/llvm/bin/clang --target=aarch64-pc-windows-msvc -DINFINITY_AARCH64 -ffreestanding -fshort-wchar -fno-stack-protector -fno-builtin -ffunction-sections -fdata-sections -O2 -c tools/camera-probe/descriptors.c -o "$out/descriptors.obj"
/opt/homebrew/opt/llvm/bin/clang --target=aarch64-pc-windows-msvc -c boot/aarch64/handoff.S -o "$out/handoff.obj"
/opt/homebrew/opt/lld/bin/lld-link /subsystem:efi_application /entry:efi_main /nodefaultlib /machine:arm64 /opt:ref /out:"$out/BOOTAA64.EFI" "$out/descriptors.obj" "$out/handoff.obj"
