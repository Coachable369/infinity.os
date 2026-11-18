#!/bin/sh
set -eu
. "$(dirname "$0")/require-build-kit.sh"
cd "$INFINITY_PROJECT_ROOT"
speaker_target=${1:-aarch64}
case "$speaker_target" in
 aarch64) speaker_triple=aarch64-unknown-none; speaker_c_target=aarch64-none-elf ;;
 x86_64) speaker_triple=x86_64-unknown-none; speaker_c_target=x86_64-none-elf ;;
 *) exit 64 ;;
esac
speaker_output="build/speaker/$speaker_target"
mkdir -p "$speaker_output"
python3 tools/speaker-assets-fetch.py
python3 tools/speaker-model-pack.py
speaker_linker=${LLD:-ld.lld}
if ! command -v "$speaker_linker" >/dev/null 2>&1; then speaker_linker=/opt/homebrew/opt/lld/bin/ld.lld; fi
"${CLANG:-clang}" --target="$speaker_c_target" -std=c11 -O2 -ffreestanding -fno-builtin -fPIC -fno-stack-protector -c sdk/infinity-face-worker/memory.c -o "$speaker_output/memory.o"
CARGO_TARGET_DIR="$INFINITY_PROJECT_ROOT/build/speaker/cargo" RUSTFLAGS="-C relocation-model=pic -C panic=abort" RUSTC_BOOTSTRAP=1 cargo build --release -Z build-std=core --manifest-path sdk/infinity-speaker-worker/Cargo.toml --target "$speaker_triple"
"$speaker_linker" -pie --no-dynamic-linker --gc-sections -z max-page-size=4096 -T sdk/c/application.ld -o "$speaker_output/speaker-worker.elf" "build/speaker/cargo/$speaker_triple/release/libinfinity_speaker_worker.a" "$speaker_output/memory.o"
