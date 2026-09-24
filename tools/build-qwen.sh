#!/bin/sh
# Builds the first ARM64 streamed-payload ISO; the ordinary ISO remains unchanged.
set -eu
cd "$(dirname "$0")/.."
ministral=model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
hermes=${INFINITY_HERMES_MODEL:-}
if test -z "$hermes"; then exec sh tools/build-hermes.sh; fi
test -f "$hermes"
mkdir -p model-cache build/qwen build/hermes builds
installer_output=builds/InfinityOS-aarch64.iso
if test -n "${QWEN_ISO_OUTPUT:-}" && test "$QWEN_ISO_OUTPUT" != "$installer_output"; then
    echo 'ERROR: installer output is fixed at builds/InfinityOS-aarch64.iso' >&2
    exit 1
fi
# A fresh staging tree cannot inherit removed model shards from an older build.
payload_build=$(mktemp -d build/hermes/payload.XXXXXX)
# This private staging tree is disposable; keep published ISOs and model-cache
# files, but do not retain tens of GiB after every successful or failed build.
trap 'test ! -d "$payload_build" || rm -r -- "$payload_build"' EXIT
esp_size=7g
iso_size=8g
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
build/behavior-harness/release/qwen-pack ministral "$ministral" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
if test -n "$hermes"; then
    build/behavior-harness/release/qwen-pack hermes "$hermes" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
    cp docs/licenses/Hermes-Llama-3.2-LICENSE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-LICENSE.txt
    cp docs/licenses/Hermes-NOTICE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-NOTICE.txt
fi
cp docs/licenses/Ministral-Apache-2.0.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/MINISTRAL-LICENSE.txt
mkfile -n "$esp_size" ${payload_build}/installed-esp.img
mformat -F -i ${payload_build}/installed-esp.img -v INFINITYEFI ::
mcopy -i ${payload_build}/installed-esp.img -s ${payload_build}/installed/EFI ::
# The packed ESP owns these bytes now; release the disposable duplicate tree.
rm -r -- "${payload_build}/installed"
build/behavior-harness/release/qwen-pack install ${payload_build}/installed-esp.img build/aarch64/installed-kernel.elf ${payload_build}/live/EFI/INFINITY/PAYLOAD build/qwen/payload-manifest.rs
# Behavioral fresh-install parity: the reassembled kernel payload must be byte
# identical to the installed kernel, including native UI and transport changes.
cat "${payload_build}"/live/EFI/INFINITY/PAYLOAD/P1-*.BIN | cmp - build/aarch64/installed-kernel.elf
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=${payload_build}/cargo cargo build --release -Z build-std=core --target aarch64-unknown-none-softfloat --features streamed-payload
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/aarch64.ld -o ${payload_build}/live/EFI/INFINITY/KERNEL.ELF ${payload_build}/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a build/aarch64/qwen-math.o
cp build/aarch64/BOOTAA64.EFI ${payload_build}/live/EFI/BOOT/
mkfile -n "$iso_size" ${payload_build}/iso/efi.img
mformat -F -i ${payload_build}/iso/efi.img -v INFINITYOS ::
mcopy -i ${payload_build}/iso/efi.img -s ${payload_build}/live/EFI ::
# The ISO's EFI image now owns the live payload. Release this private mktemp
# duplicate before allocating the final ISO alongside the previous release.
rm -r -- "${payload_build}/live"
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-install-parity -- ${payload_build}/installed-esp.img --ministral
if test -n "$hermes"; then
    CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin hermes-install-parity -- ${payload_build}/installed-esp.img
fi
# Publish only after model and installed-kernel parity have passed. Keep a failed
# image out of the canonical filename used by provisioning.
xorriso -as mkisofs -iso-level 3 -R -V INFINITY_LOCAL -e efi.img -no-emul-boot -o builds/InfinityOS-aarch64.iso.partial ${payload_build}/iso
mv builds/InfinityOS-aarch64.iso.partial "$installer_output"
