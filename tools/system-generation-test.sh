#!/bin/sh
set -eu

source_disk=build/infinity-test-disk.raw
firmware=${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}
test_dir=$(mktemp -d -t infinityos-generation.XXXXXX)
trap 'rm -rf "$test_dir"' EXIT INT TERM

test -s "$source_disk" || { echo "ERROR: install-boot-test disk is missing" >&2; exit 1; }

# ------------------------=
# FUNC: run_rejection_case
# DESC: Starts and validates run rejection case.
# ------------------=
run_rejection_case() {
    mode=$1
    disk="$test_dir/$mode.raw"
    log="$test_dir/$mode.log"
    cp "$source_disk" "$disk"
    python3 - "$disk" "$mode" <<'PY'
import struct, sys, zlib
path, mode = sys.argv[1:]
with open(path, "r+b") as image:
    image.seek(2 * 512 + 128 + 32)
    container = struct.unpack("<Q", image.read(8))[0]
    if mode == "incomplete":
        image.seek((container + 4) * 512)
        record = bytearray(image.read(512))
        assert record[:8] == b"INFSYSM1"
        struct.pack_into("<I", record, 16, 1)  # INSTALLING
        struct.pack_into("<I", record, 508, 0)
        struct.pack_into("<I", record, 508, zlib.crc32(record) & 0xffffffff)
        image.seek((container + 4) * 512)
        image.write(record)
    elif mode == "corrupt-kernel":
        image.seek((container + 2048) * 512 + 64)
        byte = image.read(1)
        image.seek(-1, 1)
        image.write(bytes([byte[0] ^ 0x5a]))
    else:
        raise AssertionError(mode)
PY
    set +e
    timeout 12 qemu-system-x86_64 -machine pc -m 256M \
        -drive if=pflash,format=raw,readonly=on,file="$firmware" \
        -drive if=ide,index=0,format=raw,file="$disk" -boot order=c \
        -serial file:"$log" -display none -no-reboot
    set -e
    grep -Fq 'No valid ACTIVE system generation found.' "$log" || {
        cat "$log"; echo "FAIL: $mode generation was not explicitly rejected" >&2; exit 1;
    }
    if grep -Fq 'Kernel online.' "$log"; then
        cat "$log"; echo "FAIL: $mode generation reached the kernel" >&2; exit 1
    fi
    echo "PASS: $mode generation rejected before kernel handoff"
}

run_rejection_case incomplete
run_rejection_case corrupt-kernel
