#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
image=build/x86_64/installed-esp.img
test -s "$image"
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
mcopy -i "$image" '::EFI/InfinityOS/Applications/app.infinity.file-navigator.manifest' "$temporary/manifest"
mcopy -i "$image" '::EFI/InfinityOS/Applications/file-navigator-icon-v1.png' "$temporary/icon.png"
cmp assets/apps/app.infinity.file-navigator.manifest "$temporary/manifest"
cmp assets/apps/file-navigator-icon-v1.png "$temporary/icon.png"
python3 - "$temporary/icon.png" <<'PY'
import struct
import sys

payload = open(sys.argv[1], "rb").read(33)
assert payload[:8] == b"\x89PNG\r\n\x1a\n"
assert struct.unpack(">II", payload[16:24]) == (256, 256)
assert payload[25] == 6
PY
echo "PASS installed System Generation contains exact File Navigator manifest and alpha icon payload"
