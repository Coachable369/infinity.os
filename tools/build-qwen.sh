#!/bin/sh
# Builds the first ARM64 streamed-payload ISO; the ordinary ISO remains unchanged.
set -eu
cd "$(dirname "$0")/.."
model=model-cache/Qwen3-8B-Q4_K_M.gguf
ministral=model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
mkdir -p model-cache
if ! test -f "$model"; then
    curl -fL --retry 3 -o "$model.part" https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/7c41481f57cb95916b40956ab2f0b139b296d974/Qwen3-8B-Q4_K_M.gguf
    mv "$model.part" "$model"
fi
if ! test -f "$ministral"; then
    curl -fL --retry 3 -o "$ministral.part" https://huggingface.co/mistralai/Ministral-3-3B-Instruct-2512-GGUF/resolve/eb599d408350ea2bb60452cb86be7c7b2fc28227/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
    mv "$ministral.part" "$ministral"
fi
make build/aarch64/BOOTAA64.EFI build/aarch64/installed-kernel.elf build/aarch64/installed-esp.img
CARGO_TARGET_DIR=build/behavior-harness cargo build --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-pack
mkdir -p build/qwen/installed/EFI/INFINITY/PAYLOAD build/qwen/live/EFI/BOOT build/qwen/live/EFI/INFINITY/PAYLOAD build/qwen/iso
mkdir -p build/qwen/iso/EFI/BOOT
cp build/aarch64/BOOTAA64.EFI build/qwen/iso/EFI/BOOT/
cp -R build/installed-fat-aarch64/EFI/. build/qwen/installed/EFI/
build/behavior-harness/release/qwen-pack model "$model" build/qwen/installed/EFI/INFINITY/PAYLOAD
build/behavior-harness/release/qwen-pack ministral "$ministral" build/qwen/installed/EFI/INFINITY/PAYLOAD
cp docs/licenses/Ministral-Apache-2.0.txt build/qwen/installed/EFI/INFINITY/PAYLOAD/MINISTRAL-LICENSE.txt
curl -fsSL https://huggingface.co/Qwen/Qwen3-8B-GGUF/raw/7c41481f57cb95916b40956ab2f0b139b296d974/LICENSE -o build/qwen/installed/EFI/INFINITY/PAYLOAD/LICENSE
mkfile -n 9g build/qwen/installed-esp.img
mformat -F -i build/qwen/installed-esp.img -v INFINITYEFI ::
mcopy -i build/qwen/installed-esp.img -s build/qwen/installed/EFI ::
build/behavior-harness/release/qwen-pack install build/qwen/installed-esp.img build/aarch64/installed-kernel.elf build/qwen/live/EFI/INFINITY/PAYLOAD build/qwen/payload-manifest.rs
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/qwen/cargo cargo build --release -Z build-std=core --target aarch64-unknown-none-softfloat --features streamed-payload
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/aarch64.ld -o build/qwen/live/EFI/INFINITY/KERNEL.ELF build/qwen/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a build/aarch64/qwen-math.o
cp build/aarch64/BOOTAA64.EFI build/qwen/live/EFI/BOOT/
mkfile -n 10g build/qwen/iso/efi.img
mformat -F -i build/qwen/iso/efi.img -v INFINITYOS ::
mcopy -i build/qwen/iso/efi.img -s build/qwen/live/EFI ::
xorriso -as mkisofs -iso-level 3 -R -V INFINITY_QWEN -e efi.img -no-emul-boot -o build/qwen/InfinityOS-Qwen3-8B-aarch64.iso build/qwen/iso
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-install-parity -- build/qwen/installed-esp.img --ministral
