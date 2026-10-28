#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
source "$repo_dir/tools/require-build-kit.sh"
studio_work="$repo_dir/builds/.studio-work"
swift_paths=(--scratch-path "$studio_work/swift"
    --cache-path "$studio_work/cache/swiftpm"
    --config-path "$studio_work/cache/swiftpm-config"
    --security-path "$studio_work/cache/swiftpm-security")
if (( $# > 1 )) || [[ "${1:-}" != "" && "${1:-}" != --clean && "${1:-}" != --skip-tests ]]; then
    echo "Usage: build-kit studio | build-kit studio-clean | build-kit studio-package" >&2
    exit 2
fi
if [[ "${1:-}" == --clean ]]; then
    swift package "${swift_paths[@]}" --package-path "$script_dir" clean
fi
app_dir="$repo_dir/builds/InfinityOS Installer Studio.app"
contents_dir="$app_dir/Contents"
binary_dir="$(swift build "${swift_paths[@]}" --package-path "$script_dir" -c release --show-bin-path)"

mkdir -p "$studio_work/behavior-tests"
if [[ ! -f "$repo_dir/assets/ui-design-kit/default/stop-audio-v1.bmp" || "$repo_dir/assets/ui-design-kit/default/stop-audio-v1.png" -nt "$repo_dir/assets/ui-design-kit/default/stop-audio-v1.bmp" ]]; then
    python3 "$repo_dir/tools/build-voice-artwork.py"
fi
rustc --edition=2021 --crate-name infinity_browser_core --crate-type rlib -C opt-level=z "$repo_dir/sdk/infinity-browser-core/lib.rs" -o "$studio_work/behavior-tests/libinfinity_browser_core.rlib"
rustc --edition=2021 -O -Awarnings --cfg studio_native_preview --extern infinity_browser_core="$studio_work/behavior-tests/libinfinity_browser_core.rlib" "$repo_dir/tools/active-painter-test.rs" -o "$studio_work/behavior-tests/infinity-studio-native-renderer"
rustc --edition=2021 -O "$repo_dir/tools/studio-precision-parity.rs" -o "$studio_work/behavior-tests/studio-precision-parity"
"$studio_work/behavior-tests/studio-precision-parity" "$studio_work/behavior-tests/studio-precision.parity"
python3 "$repo_dir/tools/studio-image-parity.py"
if [[ "${1:-}" != --skip-tests ]]; then
    swift test "${swift_paths[@]}" --package-path "$script_dir"
fi
swift build "${swift_paths[@]}" --package-path "$script_dir" -c release

rm -rf "$app_dir"
mkdir -p "$contents_dir/MacOS" "$contents_dir/Resources"
cp "$binary_dir/InfinityInstallerStudio" "$contents_dir/MacOS/InfinityInstallerStudio"
cp "$studio_work/behavior-tests/infinity-studio-native-renderer" "$contents_dir/MacOS/InfinityStudioNativeRenderer"
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

# ISO builds compile their own templates; building the editor never mutates them.
echo "$app_dir"
