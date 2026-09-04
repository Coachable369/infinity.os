#!/bin/sh
set -eu

mkdir -p build/tools
rustc --edition 2021 tools/skin-compiler.rs -o build/tools/skin-compiler
build/tools/skin-compiler assets/skins/infinity.default.dark assets/skins/infinity.default.dark/infinity.default.dark.iskin
build/tools/skin-compiler assets/skins/infinity.diagnostic.light assets/skins/infinity.diagnostic.light/infinity.diagnostic.light.iskin
build/tools/skin-compiler assets/skins/infinity.safe assets/skins/infinity.safe/infinity.safe.iskin
test "$(dd if=assets/skins/infinity.default.dark/infinity.default.dark.iskin bs=1 count=8 2>/dev/null)" = "INFSKIN1"
test "$(grep -ao '<svg' assets/skins/infinity.default.dark/infinity.default.dark.iskin | wc -l | tr -d ' ')" = "0"
test "$(grep -c '<symbol id=' assets/skins/infinity.default.dark/icons/semantic-icons.svg)" -ge 50
for icon in app-home app-console app-settings app-ai app-voice app-security app-trash; do
    rg -q "<symbol id=\"$icon\"" assets/skins/infinity.default.dark/icons/semantic-icons.svg || { echo "FAIL: missing $icon" >&2; exit 1; }
done
echo "InfinityUI skin compiler: PASS"
