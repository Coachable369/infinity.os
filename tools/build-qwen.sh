#!/bin/sh
# Builds the first ARM64 streamed-payload ISO; the ordinary ISO remains unchanged.
set -eu
cd "$(dirname "$0")/.."
model=model-cache/Qwen3-8B-Q4_K_M.gguf
ministral=model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
hermes=${INFINITY_HERMES_MODEL:-}
payload_build=build/qwen
esp_size=9g
iso_size=10g
if test -n "$hermes"; then
    test -f "$hermes"
    payload_build=build/hermes
    esp_size=11g
    iso_size=12g
fi
mkdir -p model-cache build/qwen
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
mkdir -p ${payload_build}/installed/EFI/INFINITY/PAYLOAD ${payload_build}/live/EFI/BOOT ${payload_build}/live/EFI/INFINITY/PAYLOAD ${payload_build}/iso
mkdir -p ${payload_build}/iso/EFI/BOOT
cp build/aarch64/BOOTAA64.EFI ${payload_build}/iso/EFI/BOOT/
cp -R build/installed-fat-aarch64/EFI/. ${payload_build}/installed/EFI/
build/behavior-harness/release/qwen-pack model "$model" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
build/behavior-harness/release/qwen-pack ministral "$ministral" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
if test -n "$hermes"; then
    build/behavior-harness/release/qwen-pack hermes "$hermes" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
    cp docs/licenses/Hermes-Llama-3.2-LICENSE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-LICENSE.txt
    cp docs/licenses/Hermes-NOTICE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-NOTICE.txt
fi
cp docs/licenses/Ministral-Apache-2.0.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/MINISTRAL-LICENSE.txt
curl -fsSL https://huggingface.co/Qwen/Qwen3-8B-GGUF/raw/7c41481f57cb95916b40956ab2f0b139b296d974/LICENSE -o ${payload_build}/installed/EFI/INFINITY/PAYLOAD/LICENSE
mkfile -n "$esp_size" ${payload_build}/installed-esp.img
mformat -F -i ${payload_build}/installed-esp.img -v INFINITYEFI ::
mcopy -i ${payload_build}/installed-esp.img -s ${payload_build}/installed/EFI ::
build/behavior-harness/release/qwen-pack install ${payload_build}/installed-esp.img build/aarch64/installed-kernel.elf ${payload_build}/live/EFI/INFINITY/PAYLOAD build/qwen/payload-manifest.rs
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=${payload_build}/cargo cargo build --release -Z build-std=core --target aarch64-unknown-none-softfloat --features streamed-payload
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/aarch64.ld -o ${payload_build}/live/EFI/INFINITY/KERNEL.ELF ${payload_build}/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a build/aarch64/qwen-math.o
cp build/aarch64/BOOTAA64.EFI ${payload_build}/live/EFI/BOOT/
mkfile -n "$iso_size" ${payload_build}/iso/efi.img
mformat -F -i ${payload_build}/iso/efi.img -v INFINITYOS ::
mcopy -i ${payload_build}/iso/efi.img -s ${payload_build}/live/EFI ::
xorriso -as mkisofs -iso-level 3 -R -V INFINITY_QWEN -e efi.img -no-emul-boot -o "${QWEN_ISO_OUTPUT:-${payload_build}/InfinityOS-Qwen3-8B-aarch64.iso}" ${payload_build}/iso
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-install-parity -- ${payload_build}/installed-esp.img --ministral
if test -n "$hermes"; then
    CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin hermes-install-parity -- ${payload_build}/installed-esp.img
fi
