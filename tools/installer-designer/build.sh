#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
source "$repo_dir/tools/require-build-kit.sh"
swift_paths=(--cache-path "$repo_dir/build/cache/swiftpm"
    --config-path "$repo_dir/build/cache/swiftpm-config"
    --security-path "$repo_dir/build/cache/swiftpm-security")
if (( $# > 1 )) || [[ "${1:-}" != "" && "${1:-}" != --clean ]]; then
    echo "Usage: build-kit studio | build-kit studio-clean" >&2
    exit 2
fi
if [[ "${1:-}" == --clean ]]; then
    swift package "${swift_paths[@]}" --package-path "$script_dir" clean
fi
app_dir="$repo_dir/builds/InfinityOS Installer Studio.app"
contents_dir="$app_dir/Contents"
binary_dir="$(swift build "${swift_paths[@]}" --package-path "$script_dir" -c release --show-bin-path)"

mkdir -p "$repo_dir/build/behavior-tests"
make -C "$repo_dir" studio-native-renderer
rustc --edition=2021 -O "$repo_dir/tools/studio-precision-parity.rs" -o "$repo_dir/build/behavior-tests/studio-precision-parity"
"$repo_dir/build/behavior-tests/studio-precision-parity" "$repo_dir/build/behavior-tests/studio-precision.parity"
python3 "$repo_dir/tools/studio-image-parity.py"
swift test "${swift_paths[@]}" --package-path "$script_dir"
swift build "${swift_paths[@]}" --package-path "$script_dir" -c release

rm -rf "$app_dir"
mkdir -p "$contents_dir/MacOS" "$contents_dir/Resources"
cp "$binary_dir/InfinityInstallerStudio" "$contents_dir/MacOS/InfinityInstallerStudio"
cp "$repo_dir/build/behavior-tests/infinity-studio-native-renderer" "$contents_dir/MacOS/InfinityStudioNativeRenderer"
cp "$script_dir/Info.plist" "$contents_dir/Info.plist"

resource_bundle="$binary_dir/InfinityInstallerStudio_InfinityInstallerStudio.bundle"
if [[ -d "$resource_bundle" ]]; then
    cp -R "$resource_bundle" "$contents_dir/Resources/"
fi

icon_source="$script_dir/Sources/InfinityInstallerStudio/Resources/InstallerStudioIcon-v1.png"
iconset_dir="$(mktemp -d)/InstallerStudioIcon.iconset"
mkdir -p "$iconset_dir"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$icon_source" --out "$iconset_dir/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    sips -z "$retina" "$retina" "$icon_source" --out "$iconset_dir/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset_dir" -o "$contents_dir/Resources/InstallerStudioIcon.icns"
codesign --force --deep --sign - "$app_dir" >/dev/null

"$script_dir/compile-template.sh"
"$script_dir/compile-template.sh" \
    "$repo_dir/assets/boot/configuration-screens.infinityui" \
    "$repo_dir/assets/boot/configuration-screens.iuit"
"$script_dir/compile-template.sh" \
    "$repo_dir/assets/boot/settings-screens.infinityui" \
    "$repo_dir/assets/boot/settings-screens.iuit"
echo "$app_dir"
