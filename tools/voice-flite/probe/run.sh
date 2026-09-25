#!/bin/sh
set -eu
cd "$(dirname "$0")/../../.."
python3 tools/voice-flite/build.py
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=build/voice-flite/probe-target cargo build --manifest-path tools/voice-flite/probe/Cargo.toml --release -Z build-std=core --target aarch64-unknown-none-softfloat
/opt/homebrew/opt/lld/bin/ld.lld -nostdlib --gc-sections -T tools/attention-softfloat-probe.ld -o build/voice-flite/aarch64/probe.elf build/voice-flite/probe-target/aarch64-unknown-none-softfloat/release/libinfinity_voice_probe.a build/voice-flite/aarch64/libflite.a
python3 tools/voice-flite/probe/verify.py --accel "${1:-tcg}"
