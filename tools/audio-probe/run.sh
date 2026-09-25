#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
mkdir -p build/audio-probe
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/audio-probe/target cargo build --manifest-path tools/audio-probe/Cargo.toml --release -Z build-std=core --target aarch64-unknown-none-softfloat
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib --gc-sections -T tools/attention-softfloat-probe.ld -o build/audio-probe/probe.elf build/audio-probe/target/aarch64-unknown-none-softfloat/release/libinfinity_audio_probe.a
python3 tools/audio-probe/verify.py "${1:-wav}"
