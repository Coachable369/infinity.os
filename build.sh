#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir="$project_root/builds"

cd "$project_root"

echo "==> Building InfinityOS for x86, x86_64, and AArch64"
make ui-install-parity-test
make input-regression-test
make clean
make x86
make x86_64
make aarch64

mkdir -p "$output_dir"
find "$output_dir" -maxdepth 1 -type f -name 'InfinityOS-*.iso' -delete
find "$output_dir" -maxdepth 1 -type f -name 'SHA256SUMS' -delete
cp build/infinity-x86.iso "$output_dir/InfinityOS-x86.iso"
cp build/infinity-x86_64.iso "$output_dir/InfinityOS-x86_64.iso"
cp build/infinity-aarch64.iso "$output_dir/InfinityOS-aarch64.iso"
cp tools/configure-virtualbox-arm64.sh "$output_dir/configure-virtualbox-arm64.sh"
cp tools/start-virtualbox-arm64.sh "$output_dir/start-virtualbox-arm64.sh"
chmod +x "$output_dir/configure-virtualbox-arm64.sh" "$output_dir/start-virtualbox-arm64.sh"

for image in "$output_dir"/*.iso; do
    test -s "$image" || { echo "ERROR: missing image: $image" >&2; exit 1; }
    file "$image" | grep -q 'bootable' || { echo "ERROR: image is not bootable: $image" >&2; exit 1; }
done

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
echo "Intel/AMD images: builds/InfinityOS-x86.iso and builds/InfinityOS-x86_64.iso"
