#!/bin/sh
# Fast bootstrap test; no ISO rebuild or user VM mutation.
set -eu
cd "$(dirname "$0")/../.."
sh tools/native-c-probe/build-hello.sh
rustc --edition=2021 tools/native-c-probe/image-test.rs -o build/native-c/image-test
build/native-c/image-test
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/native-c/cargo cargo build --release \
    -Z build-std=core --target x86_64-unknown-none --manifest-path tools/native-c-probe/Cargo.toml
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/x86_64.ld \
    -o build/native-c/probe.elf build/native-c/cargo/x86_64-unknown-none/release/libinfinity_native_c_probe.a
make build/x86_64/BOOTX64.EFI
python3 tools/native-c-probe/run.py
