#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
app_dir="$repo_dir/builds/InfinityOS Installer Studio.app"
contents_dir="$app_dir/Contents"
binary_dir="$(swift build --package-path "$script_dir" -c release --show-bin-path)"

swift test --package-path "$script_dir"
swift build --package-path "$script_dir" -c release

rm -rf "$app_dir"
mkdir -p "$contents_dir/MacOS" "$contents_dir/Resources"
cp "$binary_dir/InfinityInstallerStudio" "$contents_dir/MacOS/InfinityInstallerStudio"
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

"$contents_dir/MacOS/InfinityInstallerStudio" --export-default "$repo_dir/assets/boot"
echo "$app_dir"
