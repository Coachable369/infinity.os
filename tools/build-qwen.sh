#!/bin/sh
# Builds the shared native model image for the selected architecture.
set -eu
cd "$(dirname "$0")/.."
. tools/require-build-kit.sh
if test "${INFINITY_ISO_BUILD_AUTHORITY:-}" != build.sh; then
    echo 'ERROR: ISO publication is restricted to ./build.sh' >&2
    exit 2
fi
arch=aarch64
if test "$#" -eq 2 && test "$1" = --target; then arch=$2; elif test "$#" -ne 0; then exit 2; fi
case "$arch" in
    aarch64) triple=aarch64-unknown-none-softfloat; boot=BOOTAA64.EFI; installed_fat=build/installed-fat-aarch64 ;;
    x86_64) triple=x86_64-unknown-none; boot=BOOTX64.EFI; installed_fat=build/installed-fat ;;
    *) echo 'Unsupported native model target' >&2; exit 2 ;;
esac
ministral=model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
hermes=${INFINITY_HERMES_MODEL:-}
if test -z "$hermes"; then exec sh tools/build-hermes.sh --target "$arch"; fi
test -f "$hermes"
mkdir -p model-cache build/qwen build/hermes build/tools builds
installer_output=builds/InfinityOS-$arch.iso
if test -n "${QWEN_ISO_OUTPUT:-}" && test "$QWEN_ISO_OUTPUT" != "$installer_output"; then
    echo "ERROR: installer output is fixed at $installer_output" >&2
    exit 1
fi
# A fresh staging tree cannot inherit removed model shards from an older build.
lock=builds/.$arch-build-lock
if ! mkdir "$lock" 2>/dev/null; then
    echo "ERROR: $arch ISO build already active ($lock)." >&2
    exit 1
fi
trap 'rmdir "$lock"' EXIT
payload_build=$(mktemp -d build/hermes/payload.XXXXXX)
# This private staging tree is disposable; keep published ISOs and model-cache
# files, but do not retain tens of GiB after every successful or failed build.
trap 'test ! -d "$payload_build" || rm -r -- "$payload_build"; rm -f -- "$installer_output.partial"; rmdir "$lock"' EXIT
trap 'exit 130' HUP INT TERM
if ! test -f "$ministral"; then
    curl -fL --retry 3 -o "$ministral.part" https://huggingface.co/mistralai/Ministral-3-3B-Instruct-2512-GGUF/resolve/eb599d408350ea2bb60452cb86be7c7b2fc28227/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf
    mv "$ministral.part" "$ministral"
fi
model_bytes=$(wc -c < "$hermes")
secondary_bytes=$(wc -c < "$ministral")
python3 tools/iso-staging.py check build "$((2 * (model_bytes + secondary_bytes) + 3221225472))"
make build/$arch/$boot build/$arch/installed-kernel.elf build/$arch/installed-esp.img
CARGO_TARGET_DIR=build/behavior-harness cargo build --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-pack
mkdir -p ${payload_build}/installed/EFI/INFINITY/PAYLOAD ${payload_build}/live/EFI/BOOT ${payload_build}/live/EFI/INFINITY/PAYLOAD ${payload_build}/iso
mkdir -p ${payload_build}/iso/EFI/BOOT
cp build/$arch/$boot ${payload_build}/iso/EFI/BOOT/
cp -R "$installed_fat/EFI/." ${payload_build}/installed/EFI/
build/behavior-harness/release/qwen-pack ministral "$ministral" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
if test -n "$hermes"; then
    build/behavior-harness/release/qwen-pack hermes "$hermes" ${payload_build}/installed/EFI/INFINITY/PAYLOAD
    cp docs/licenses/Hermes-Llama-3.2-LICENSE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-LICENSE.txt
    cp docs/licenses/Hermes-NOTICE.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/HERMES-NOTICE.txt
fi
cp docs/licenses/Ministral-Apache-2.0.txt ${payload_build}/installed/EFI/INFINITY/PAYLOAD/MINISTRAL-LICENSE.txt
esp_size=$(python3 tools/iso-staging.py size "${payload_build}/installed")
# Consumptive copies need headroom for one model shard, not a second whole tree.
python3 tools/iso-staging.py check "$payload_build" 1073741824
mkfile -n "$esp_size" ${payload_build}/installed-esp.img
mformat -F -i ${payload_build}/installed-esp.img -v INFINITYEFI ::
python3 tools/iso-staging.py consume "${payload_build}/installed" "${payload_build}/installed-esp.img"
# The packed ESP owns these bytes now; release the disposable duplicate tree.
rm -r -- "${payload_build}/installed"
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-install-parity -- ${payload_build}/installed-esp.img --ministral
if test -n "$hermes"; then
    CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin hermes-install-parity -- ${payload_build}/installed-esp.img
fi
python3 tools/iso-staging.py check "$payload_build" "$((esp_size + 1073741824))"
build/behavior-harness/release/qwen-pack install ${payload_build}/installed-esp.img build/$arch/installed-kernel.elf ${payload_build}/live/EFI/INFINITY/PAYLOAD build/qwen/payload-manifest.rs
# Behavioral fresh-install parity: the reassembled kernel payload must be byte
# identical to the installed kernel, including native UI and transport changes.
cat "${payload_build}"/live/EFI/INFINITY/PAYLOAD/P1-*.BIN | cmp - build/$arch/installed-kernel.elf
# Its validated bytes now belong to the payload shards. Release only this
# invocation's disposable ESP before allocating another full EFI image.
rm -- "${payload_build}/installed-esp.img"
payload_features=streamed-payload
set --
# Match the already-built installed kernel's browser configuration. The model
# installer has its own streamed manifest, but uses the same native component.
if test "$(cat build/browser-mode)" = 1; then
    payload_features=streamed-payload,native-browser
    set -- --strip-debug --undefined=infinity_browser_private_infinity_browser_run "build/servo-platform-probe/browser-private-$arch.o"
fi
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=${payload_build}/cargo cargo build --release -Z build-std=core --target "$triple" --features "$payload_features"
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/$arch.ld -o ${payload_build}/live/EFI/INFINITY/KERNEL.ELF ${payload_build}/cargo/$triple/release/libinfinity_kernel.a build/$arch/qwen-math.o build/voice-kokoro/$arch/private-native.o "$@"
rustc --edition=2021 -O tools/cursor-install-parity.rs -o build/tools/cursor-install-parity
python3 tools/voice-kokoro/install-parity.py build/$arch/installed-kernel.elf ${payload_build}/live/EFI/INFINITY/KERNEL.ELF
build/tools/cursor-install-parity build/$arch/installed-kernel.elf ${payload_build}/live/EFI/INFINITY/KERNEL.ELF
cp build/$arch/$boot ${payload_build}/live/EFI/BOOT/
rm -r -- "${payload_build}/cargo"
iso_size=$(python3 tools/iso-staging.py size "${payload_build}/live")
python3 tools/iso-staging.py check "$payload_build" 1073741824
mkfile -n "$iso_size" ${payload_build}/iso/efi.img
mformat -F -i ${payload_build}/iso/efi.img -v INFINITYOS ::
python3 tools/iso-staging.py consume "${payload_build}/live" "${payload_build}/iso/efi.img"
# The ISO's EFI image now owns the live payload. Release this private mktemp
# duplicate before allocating the final ISO alongside the previous release.
rm -r -- "${payload_build}/live"
# Publish only after model and installed-kernel parity have passed. Keep a failed
# image out of the canonical filename used by provisioning.
python3 tools/iso-staging.py check builds "$((iso_size + 1073741824))"
xorriso -as mkisofs -iso-level 3 -R -V INFINITY_LOCAL -e efi.img -no-emul-boot -o "$installer_output.partial" ${payload_build}/iso
python3 tools/full-bundle-iso-test.py "$installer_output.partial"
python3 tools/audio-install-parity.py "$installer_output.partial" "$arch"
mv "$installer_output.partial" "$installer_output"
