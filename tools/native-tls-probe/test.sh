#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
sh tools/native-c-probe/build-tls.sh
rustc --edition=2021 -A warnings tools/native-c-probe/tls-test.rs -o build/native-c/tls-test
build/native-c/tls-test
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/native-tls/cargo cargo build --release \
  -Z build-std=core --target x86_64-unknown-none --manifest-path tools/native-tls-probe/Cargo.toml
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib -static -T linker/x86_64.ld \
  -o build/native-c/tls-probe.elf build/native-tls/cargo/x86_64-unknown-none/release/libinfinity_native_tls_probe.a
make build/x86_64/BOOTX64.EFI
python3 tools/native-c-probe/run.py --tls
