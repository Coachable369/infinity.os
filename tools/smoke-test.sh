#!/bin/sh
set -eu

arch=${1:-x86_64}
case "$arch" in
  x86)
    log=$(mktemp -t infinityos-x86-boot.XXXXXX)
    trap 'rm -f "$log"' EXIT INT TERM
    set +e
    timeout 12 qemu-system-i386 -machine pc -m 128M \
      -cdrom builds/InfinityOS-x86.iso -boot d -serial stdio -display none -no-reboot >"$log" 2>&1
    status=$?
    set -e
    if grep -Fq 'Kernel online.' "$log" && grep -Fq 'keyboard0 ready' "$log" &&
       grep -Fq 'mouse0 ready' "$log" && grep -Fq 'input/display foundation online' "$log"; then
      cat "$log"
      echo 'PASS: x86 reached the Rust kernel'
      exit 0
    fi
    cat "$log"
    echo "FAIL: x86 did not reach the Rust kernel (QEMU status $status)" >&2
    exit 1
    ;;
  x86_64)
    firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
    log=$(mktemp -t infinityos-boot.XXXXXX)
    trap 'rm -f "$log"' EXIT INT TERM
    set +e
    timeout 25 qemu-system-x86_64 -machine q35 -m 4096M \
      -drive if=pflash,format=raw,readonly=on,file="$firmware" \
      -cdrom builds/InfinityOS-x86_64.iso \
      -serial stdio -display none -no-reboot >"$log" 2>&1
    status=$?
    set -e
    if grep -Fq 'Kernel online.' "$log" && grep -Fq 'display0 ready' "$log" &&
       grep -Fq 'keyboard0 ready' "$log" && grep -Fq 'mouse0 ready' "$log" &&
       grep -Fq 'input/display foundation online' "$log"; then
      cat "$log"
      echo 'PASS: x86_64 reached the Rust kernel'
      exit 0
    fi
    cat "$log"
    echo "FAIL: x86_64 did not reach the Rust kernel (QEMU status $status)" >&2
    exit 1
    ;;
  aarch64)
    firmware=${AAVMF_CODE:-/opt/homebrew/share/qemu/edk2-aarch64-code.fd}
    log=$(mktemp -t infinityos-arm64-boot.XXXXXX)
    trap 'rm -f "$log"' EXIT INT TERM
    set +e
    timeout 30 qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
      -bios "$firmware" -device ramfb -device virtio-scsi-pci \
      -drive if=none,id=cd,format=raw,media=cdrom,file=builds/InfinityOS-aarch64-qemu-test.iso \
      -device scsi-cd,drive=cd,bootindex=0 -serial stdio -display none -no-reboot >"$log" 2>&1
    status=$?
    set -e
    if grep -Fq 'Kernel online.' "$log" && grep -Fq 'display0 ready' "$log" &&
       grep -Fq 'input/display foundation online' "$log"; then
      cat "$log"
      echo 'PASS: AArch64 reached the Rust kernel'
      exit 0
    fi
    cat "$log"
    echo "FAIL: AArch64 did not reach the Rust kernel (QEMU status $status)" >&2
    exit 1
    ;;
  *) echo "ERROR: no smoke test implemented for $arch" >&2; exit 2 ;;
esac
