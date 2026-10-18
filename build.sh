#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir="$project_root/builds"
temporary_dir="$project_root/build/tmp"

cd "$project_root"
if [ "${INFINITY_BUILD_KIT_ACTIVE:-0}" != "1" ]; then
    exec "$project_root/build-kit" incremental "$@"
fi
test "${INFINITY_PROJECT_ROOT:-}" = "$project_root"
test "${TMPDIR:-}" = "$temporary_dir"

target=all
package_only=false
machine=native
while [ "$#" -gt 0 ]; do
    case "$1" in
        --package-only) package_only=true; shift ;;
        --target) [ "$#" -ge 2 ] || exit 2; target=$2; shift 2 ;;
        --machine) [ "$#" -ge 2 ] || exit 2; machine=$2; shift 2 ;;
        *) echo "usage: ./build.sh [--package-only] [--target aarch64|x86_64|all] [--machine native|qemu]" >&2; exit 2 ;;
    esac
done
case "$target" in aarch64|x86_64|all) ;; *) echo "unsupported build target: $target" >&2; exit 2 ;; esac
case "$machine" in native|qemu) ;; *) echo "unsupported machine: $machine" >&2; exit 2 ;; esac
if [ "$machine" = qemu ] && { [ "$target" != aarch64 ] || [ "$package_only" != true ]; }; then
    echo "QEMU model media requires --package-only --target aarch64 --machine qemu" >&2
    exit 2
fi

# Fast test-ISO handoff after a build: retain native dependency products and
# defer the broad regression suite, never the publication/parity gates.
if [ "$package_only" = true ]; then
    case "$target" in aarch64|x86_64) ;; *) echo "--package-only requires --target aarch64|x86_64" >&2; exit 2 ;; esac
    test -s "build/$target/installed-kernel.elf" || {
        echo "No built $target kernel; run ./build-kit $target first." >&2
        exit 1
    }
    export INFINITY_ISO_BUILD_AUTHORITY=build.sh
    if [ "$machine" = qemu ]; then
        python3 tools/kernel-layout-test.py
        sh tools/build-hermes.sh --target "$target" --machine qemu
        (cd "$project_root/build/test-media" && shasum -a 256 InfinityOS-aarch64-qemu-models.bootmedia > InfinityOS-aarch64-qemu-models.bootmedia.sha256)
        (cd "$project_root/build/test-media" && shasum -a 256 -c InfinityOS-aarch64-qemu-models.bootmedia.sha256)
        echo "Verified ARM QEMU model test media: $project_root/build/test-media/InfinityOS-aarch64-qemu-models.bootmedia (canonical ISOs unchanged; broad regressions deferred)"
        exit 0
    fi
    sh tools/build-hermes.sh --target "$target"
    (cd "$output_dir" && shasum -a 256 "InfinityOS-$target.iso" > "InfinityOS-$target.iso.sha256")
    (cd "$output_dir" && shasum -a 256 -c "InfinityOS-$target.iso.sha256")
    checksum_index=$(mktemp "$temporary_dir/iso-checksums.XXXXXX")
    if [ -f "$output_dir/SHA256SUMS" ]; then
        awk -v image="InfinityOS-$target.iso" '$2 != image && $2 != "./" image' "$output_dir/SHA256SUMS" > "$checksum_index"
    fi
    cat "$output_dir/InfinityOS-$target.iso.sha256" >> "$checksum_index"
    mv "$checksum_index" "$output_dir/SHA256SUMS"
    if [ "$target" = aarch64 ]; then
        cp tools/configure-virtualbox-arm64.sh tools/start-virtualbox-arm64.sh "$output_dir/"
        chmod +x "$output_dir/configure-virtualbox-arm64.sh" "$output_dir/start-virtualbox-arm64.sh"
    fi
    echo "Verified $target test ISO: $output_dir/InfinityOS-$target.iso (broad regression suite deferred)"
    exit 0
fi

echo "==> FULL COMMAND LIST ($target)"
if [ "$target" = aarch64 ]; then
    cat <<'COMMANDS'
python3 tools/build-kit-test.py
python3 tools/build-workspace-test.py
python3 tools/installer-output-test.py
python3 tools/boot-loader-parity-test.py
python3 tools/iso-kernel-freshness-test.py
python3 tools/voice-pipeline-test.py
sh tools/font-library-test.sh
make install-boot-handoff-test
make aarch64-bootstrap
sh tools/build-hermes.sh --target aarch64
python3 tools/full-bundle-iso-test.py builds/InfinityOS-aarch64.iso
cp tools/configure-virtualbox-arm64.sh builds/configure-virtualbox-arm64.sh
cp tools/start-virtualbox-arm64.sh builds/start-virtualbox-arm64.sh
chmod +x builds/configure-virtualbox-arm64.sh builds/start-virtualbox-arm64.sh
tools/guard-virtualbox-arm64-input.sh
(cd builds && shasum -a 256 InfinityOS-aarch64.iso > SHA256SUMS)
python3 tools/browser-artwork-parity.py
COMMANDS
elif [ "$target" = x86_64 ]; then
    cat <<'COMMANDS'
python3 tools/build-kit-test.py
python3 tools/build-workspace-test.py
python3 tools/installer-output-test.py
python3 tools/boot-loader-parity-test.py
python3 tools/iso-kernel-freshness-test.py
python3 tools/voice-pipeline-test.py
sh tools/font-library-test.sh
make install-boot-handoff-test
make build/test-media/InfinityOS-x86_64.bootmedia
sh tools/build-hermes.sh --target x86_64
python3 tools/full-bundle-iso-test.py builds/InfinityOS-x86_64.iso
(cd builds && shasum -a 256 InfinityOS-x86_64.iso > SHA256SUMS)
python3 tools/browser-artwork-parity.py
COMMANDS
else
    cat <<'COMMANDS'
python3 tools/build-kit-test.py
python3 tools/build-workspace-test.py
python3 tools/log-retention-test.py
sh tools/select-install-iso-test.sh
python3 tools/installer-output-test.py
python3 tools/boot-loader-parity-test.py
python3 tools/iso-kernel-freshness-test.py
python3 tools/voice-pipeline-test.py
sh tools/font-library-test.sh
python3 tools/iso-staging-test.py
python3 tools/re-provision-tpm-test.py
make install-boot-handoff-test
cargo test --manifest-path sdk/infinity-browser-core/Cargo.toml
cargo test --manifest-path tools/file-preview-harness/Cargo.toml
cargo test --manifest-path tools/behavior-harness/Cargo.toml --bin browser-entropy-test
[optional legacy smoke media] make x86
make build/test-media/InfinityOS-x86_64.bootmedia
make aarch64-bootstrap
make system-sound-test system-sound-install-parity-test authentication-motion-test installer-capacity-test installer-entropy-test editor-window-test active-painter-test video-driver-test
make build/test-media/InfinityOS-x86_64.bootmedia aarch64-bootstrap
python3 tools/installed-kernel-parity-test.py
python3 tools/native-compiler-parity-test.py
sh tools/native-c-production-test.sh
sh tools/application-interaction-test.sh
make editor-assistant-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin application-interaction-model-test -- --preview-only
python3 tools/native-application-case-test.py
make crash-screen-test component-manifest-test settings-color-test installer-template-test milestone-7x-test ai-test object-test fabric-test network-test network-wire-test http-transport-test https-service-test native-https-test native-https-arm-test native-https-rsa-test native-tls-test milestone-9-test milestone-9-service-test performance-test resource-policy-test
tools/ui-install-parity-test.sh build/infinity-x86_64.img build/infinity-aarch64.img build/infinity-aarch64-qemu.img build/x86_64/installed-esp.img build/aarch64/installed-esp.img
make input-regression-test app-launcher-interaction-test
sh tools/build-hermes.sh --target x86_64
sh tools/build-hermes.sh --target aarch64
python3 tools/full-bundle-iso-test.py builds/InfinityOS-x86_64.iso builds/InfinityOS-aarch64.iso
python3 tools/browser-artwork-parity.py
tools/guard-virtualbox-arm64-input.sh
shasum -a 256 builds/InfinityOS-x86_64.iso builds/InfinityOS-aarch64.iso > builds/SHA256SUMS
COMMANDS
fi
echo "==> END COMMAND LIST"

# Deprecated incomplete ISO names are never retained as release artifacts.
rm -f "$output_dir/InfinityOS-x86.iso" \
      "$output_dir/InfinityOS-x86_64-bootstrap-test.iso" \
      "$output_dir/InfinityOS-aarch64-bootstrap-test.iso" \
      "$output_dir/InfinityOS-aarch64-qemu-test.iso"

export INFINITY_ISO_BUILD_AUTHORITY=build.sh

# The firmware controller contract is shared by live and installed loaders.
cc -std=c11 -Wall -Wextra -Werror tools/native-controller-test.c -o build/native-controller-test
build/native-controller-test

if [ "$target" != all ]; then
    python3 tools/build-kit-test.py
    python3 tools/build-workspace-test.py
    python3 tools/installer-output-test.py
    python3 tools/boot-loader-parity-test.py
    python3 tools/iso-kernel-freshness-test.py
    python3 tools/voice-pipeline-test.py
    sh tools/font-library-test.sh
    make install-boot-handoff-test
    if [ "$target" = aarch64 ]; then
        make aarch64-bootstrap
        sh tools/build-hermes.sh --target aarch64
        python3 tools/full-bundle-iso-test.py "$output_dir/InfinityOS-aarch64.iso"
        cp tools/configure-virtualbox-arm64.sh "$output_dir/configure-virtualbox-arm64.sh"
        cp tools/start-virtualbox-arm64.sh "$output_dir/start-virtualbox-arm64.sh"
        chmod +x "$output_dir/configure-virtualbox-arm64.sh" "$output_dir/start-virtualbox-arm64.sh"
        tools/guard-virtualbox-arm64-input.sh
        (cd "$output_dir" && shasum -a 256 InfinityOS-aarch64.iso > SHA256SUMS)
    else
        make build/test-media/InfinityOS-x86_64.bootmedia
        sh tools/build-hermes.sh --target x86_64
        python3 tools/full-bundle-iso-test.py "$output_dir/InfinityOS-x86_64.iso"
        (cd "$output_dir" && shasum -a 256 InfinityOS-x86_64.iso > SHA256SUMS)
    fi
    python3 tools/browser-artwork-parity.py
    python3 tools/native-compiler-parity-test.py --arch "$target"
    echo "InfinityOS $target full-bundle ISO is ready in $output_dir"
    exit 0
fi

# Catch workspace and provisioning contract failures before expensive builds.
python3 tools/build-kit-test.py
python3 tools/build-workspace-test.py
python3 tools/log-retention-test.py
sh tools/select-install-iso-test.sh
python3 tools/installer-output-test.py
python3 tools/boot-loader-parity-test.py
python3 tools/iso-kernel-freshness-test.py
python3 tools/voice-pipeline-test.py
python3 tools/iso-staging-test.py
python3 tools/re-provision-tpm-test.py
make install-boot-handoff-test
cargo test --manifest-path sdk/infinity-browser-core/Cargo.toml
CARGO_TARGET_DIR=build/behavior-harness cargo test --manifest-path tools/behavior-harness/Cargo.toml --bin browser-entropy-test
sh tools/font-library-test.sh

echo "==> Building InfinityOS for x86_64 and AArch64"

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
make build/test-media/InfinityOS-x86_64.bootmedia
make aarch64-bootstrap
make system-sound-test
make system-sound-install-parity-test
make authentication-motion-test
make installer-capacity-test
make installer-entropy-test
make editor-window-test
make active-painter-test
make video-driver-test
# Native dependency preparation can refresh linked kernels after the first
# architecture pass. Re-stage the actual media before comparing its payloads.
make build/test-media/InfinityOS-x86_64.bootmedia aarch64-bootstrap
python3 tools/installed-kernel-parity-test.py
python3 tools/native-compiler-parity-test.py
python3 tools/kernel-layout-test.py
# Bounded native-generation regressions run real executable entrypoints, not
# empty Rust test harnesses. Preview-only returns before loading model weights.
sh tools/native-c-production-test.sh
sh tools/application-interaction-test.sh
make editor-assistant-test
CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release \
    --manifest-path tools/behavior-harness/Cargo.toml \
    --bin application-interaction-model-test -- --preview-only
python3 tools/native-application-case-test.py
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
make https-service-test
make native-https-test
make native-https-arm-test
make native-https-rsa-test
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
sh tools/build-hermes.sh --target x86_64
sh tools/build-hermes.sh
python3 tools/browser-artwork-parity.py

# Build targets publish directly into builds; there is no second installer copy.
cp tools/configure-virtualbox-arm64.sh "$output_dir/configure-virtualbox-arm64.sh"
cp tools/start-virtualbox-arm64.sh "$output_dir/start-virtualbox-arm64.sh"
chmod +x "$output_dir/configure-virtualbox-arm64.sh" "$output_dir/start-virtualbox-arm64.sh"

verification_dir=$(mktemp -d "$temporary_dir/release-verification.XXXXXX")
trap 'rm -rf "$verification_dir"' EXIT HUP INT TERM
for image in "$output_dir"/*.iso; do
    test -s "$image" || { echo "ERROR: missing image: $image" >&2; exit 1; }
    python3 tools/full-bundle-iso-test.py "$image"
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
    echo "Legacy Intel/AMD BIOS smoke media: build/test-media/InfinityOS-x86.bootmedia"
fi
