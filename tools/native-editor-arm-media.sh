#!/bin/sh
# QEMU-only acceptance media. Published VirtualBox ISOs remain untouched.
set -eu
cd "$(dirname "$0")/.."
. tools/require-build-kit.sh
stage=$(mktemp -d build/native-editor-media.XXXXXX)
trap 'rm -r -- "$stage"' EXIT HUP INT TERM
mkdir -p "$stage/iso" "$stage/original" "$stage/payload"
xorriso -osirrox on -indev builds/InfinityOS-aarch64.iso -extract /efi.img "$stage/iso/efi.img"
mcopy -i "$stage/iso/efi.img" '::/EFI/INFINITY/PAYLOAD/P0-*.BIN' "$stage/original/"
cat "$stage"/original/P0-*.BIN > "$stage/installed-esp.img"
# Same packaged models/assets, same installed kernel source, different native
# load-address adapter. No installed-disk patch or synthetic model response.
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static --strip-debug \
    --undefined=infinity_browser_private_infinity_browser_run -T linker/aarch64-qemu.ld \
    -o "$stage/installed-kernel.elf" build/aarch64/libinstalled-kernel.a \
    build/aarch64/qwen-math.o build/voice-kokoro/aarch64/private-native.o \
    build/servo-platform-probe/browser-private-aarch64.o
build/behavior-harness/release/qwen-pack install "$stage/installed-esp.img" \
    "$stage/installed-kernel.elf" "$stage/payload" build/qwen/payload-manifest.rs
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR="$stage/cargo" cargo build --release -Z build-std=core \
    --target aarch64-unknown-none-softfloat --features streamed-payload,native-browser
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static --strip-debug \
    --undefined=infinity_browser_private_infinity_browser_run -T linker/aarch64-qemu.ld \
    -o "$stage/kernel.elf" "$stage/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a" \
    build/aarch64/qwen-math.o build/voice-kokoro/aarch64/private-native.o \
    build/servo-platform-probe/browser-private-aarch64.o
mcopy -o -i "$stage/iso/efi.img" "$stage/kernel.elf" ::/EFI/INFINITY/KERNEL.ELF
mcopy -o -i "$stage/iso/efi.img" "$stage"/payload/P1-*.BIN ::/EFI/INFINITY/PAYLOAD/
mkdir -p build/test-media
xorriso -as mkisofs -iso-level 3 -R -V INFINITY_NATIVE_TEST -e efi.img -no-emul-boot \
    -o build/test-media/native-editor-aarch64.bootmedia "$stage/iso"
