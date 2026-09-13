#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir="$project_root/builds"

cd "$project_root"

# Catch provisioning contract failures before expensive builds or cleanup.
sh tools/select-install-iso-test.sh
python3 tools/re-provision-tpm-test.py
make install-boot-handoff-test

echo "==> Building InfinityOS for x86_64 and AArch64"
make clean

# The legacy BIOS loader must place its complete 32-bit payload below the
# conventional-memory/video boundary. The current graphical kernel is larger
# than that physical window, so legacy x86 is opt-in until its staged loader is
# implemented. Never let that unsupported target prevent Studio users from
# producing the supported UEFI images.
legacy_x86_built=false
if [ "${INFINITY_BUILD_LEGACY_X86:-0}" = "1" ]; then
    echo "==> Building opt-in legacy BIOS x86 image"
    make x86
    legacy_x86_built=true
else
    echo "==> Skipping legacy BIOS x86 (set INFINITY_BUILD_LEGACY_X86=1 to attempt it)"
fi
make x86_64
make aarch64
make installer-capacity-test
make installer-entropy-test
make editor-window-test
make active-painter-test
make video-driver-test
python3 tools/installed-kernel-parity-test.py
make crash-screen-test
make component-manifest-test
make settings-color-test
make installer-template-test
make milestone-7x-test
make ai-test
make object-test
make fabric-test
make network-test
make network-wire-test
make http-transport-test
make native-https-test
make native-https-arm-test
make native-tls-test
make milestone-9-test
make milestone-9-service-test
make performance-test
make resource-policy-test
tools/ui-install-parity-test.sh \
    build/infinity-x86_64.img \
    build/infinity-aarch64.img \
    build/infinity-aarch64-qemu.img \
    build/x86_64/installed-esp.img \
    build/aarch64/installed-esp.img
make input-regression-test
make app-launcher-interaction-test
sh tools/build-qwen.sh

mkdir -p "$output_dir"
find "$output_dir" -maxdepth 1 -type f -name 'InfinityOS-*.iso' -delete
find "$output_dir" -maxdepth 1 -type f -name 'SHA256SUMS' -delete
if [ "$legacy_x86_built" = true ]; then
    cp build/infinity-x86.iso "$output_dir/InfinityOS-x86.iso"
fi
cp build/infinity-x86_64.iso "$output_dir/InfinityOS-x86_64.iso"
cp build/qwen/InfinityOS-Qwen3-8B-aarch64.iso "$output_dir/InfinityOS-aarch64.iso"
# Assert binary parity: never publish the legacy ARM image under the release name.
cmp build/qwen/InfinityOS-Qwen3-8B-aarch64.iso "$output_dir/InfinityOS-aarch64.iso"
cp tools/configure-virtualbox-arm64.sh "$output_dir/configure-virtualbox-arm64.sh"
cp tools/start-virtualbox-arm64.sh "$output_dir/start-virtualbox-arm64.sh"
chmod +x "$output_dir/configure-virtualbox-arm64.sh" "$output_dir/start-virtualbox-arm64.sh"

verification_dir=$(mktemp -d)
trap 'rm -rf "$verification_dir"' EXIT HUP INT TERM
for image in "$output_dir"/*.iso; do
    test -s "$image" || { echo "ERROR: missing image: $image" >&2; exit 1; }
    case "$image" in
        *aarch64*) boot_payload=/EFI/BOOT/BOOTAA64.EFI ;;
        *x86_64*) boot_payload=/EFI/BOOT/BOOTX64.EFI ;;
        *) boot_payload=/x86-boot.img ;;
    esac
    extracted_payload="$verification_dir/$(basename "$image").boot"
    xorriso -osirrox on -indev "$image" -extract "$boot_payload" "$extracted_payload" >/dev/null 2>&1 || {
        echo "ERROR: boot payload cannot be extracted from $image" >&2
        exit 1
    }
    test -s "$extracted_payload" || { echo "ERROR: empty boot payload in $image" >&2; exit 1; }
done
rm -rf "$verification_dir"
trap - EXIT HUP INT TERM

(cd "$output_dir" && shasum -a 256 ./*.iso > SHA256SUMS)

# A freshly created VirtualBox ARM VM defaults to PS/2 input, which its ARM
# machine cannot deliver to InfinityOS. Correct every powered-off ARM VM already
# attached to this build, keep the tablet on xHCI instead of legacy OHCI, and
# fail loudly if an incompatible VM is running.
tools/guard-virtualbox-arm64-input.sh

echo
echo "InfinityOS images are ready in $output_dir"
echo "Apple Silicon VirtualBox image: builds/InfinityOS-aarch64.iso"
echo "Start an ARM64 VM safely: builds/start-virtualbox-arm64.sh '<VM name>'"
echo "Intel/AMD UEFI image: builds/InfinityOS-x86_64.iso"
if [ "$legacy_x86_built" = true ]; then
    echo "Legacy Intel/AMD BIOS image: builds/InfinityOS-x86.iso"
fi
