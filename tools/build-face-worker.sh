#!/bin/sh
set -eu
. "$(dirname "$0")/require-build-kit.sh"
cd "$INFINITY_PROJECT_ROOT"
face_target=${1:-aarch64}
case "$face_target" in
    aarch64) face_triple=aarch64-unknown-none; face_c_target=aarch64-none-elf ;;
    x86_64) face_triple=x86_64-unknown-none; face_c_target=x86_64-none-elf ;;
    *) echo "Unsupported native face target" >&2; exit 64 ;;
esac
face_output="build/face/$face_target"
mkdir -p "$face_output" build/face/models
python3 tools/face-assets-fetch.py
python3 tools/face-model-pack.py yunet model-cache/face/yunet.onnx build/face/models/yunet.iface
python3 tools/face-model-pack.py sface model-cache/face/sface.onnx build/face/models/sface.iface
face_clang=${CLANG:-clang}
face_linker=${LLD:-ld.lld}
if ! command -v "$face_clang" >/dev/null 2>&1; then echo "Native clang is required" >&2; exit 69; fi
if ! command -v "$face_linker" >/dev/null 2>&1; then
    if [ -x /opt/homebrew/opt/lld/bin/ld.lld ]; then face_linker=/opt/homebrew/opt/lld/bin/ld.lld;
    else echo "Native ELF linker is required" >&2; exit 69; fi
fi
for face_source in sdk/infinity-face-core/jpeg.c sdk/infinity-face-worker/memory.c; do
    face_name=$(basename "$face_source" .c)
    "$face_clang" --target="$face_c_target" -std=c11 -O2 -ffreestanding -fno-builtin \
        -fPIC -fno-stack-protector -ffunction-sections -fdata-sections \
        -Isdk/c/include -Ibuild/face/native-deps -c "$face_source" -o "$face_output/$face_name.o"
done
CARGO_TARGET_DIR="$INFINITY_PROJECT_ROOT/build/face/cargo" \
RUSTFLAGS="-C relocation-model=pic -C panic=abort" \
RUSTC_BOOTSTRAP=1 cargo build --release -Z build-std=core --manifest-path sdk/infinity-face-worker/Cargo.toml --target "$face_triple"
"$face_linker" -pie --no-dynamic-linker --gc-sections -z max-page-size=4096 \
    -T sdk/c/application.ld -o "$face_output/face-worker.elf" \
    "build/face/cargo/$face_triple/release/libinfinity_face_worker.a" "$face_output/jpeg.o" "$face_output/memory.o"
python3 tools/face-package-manifest.py "$face_target"
