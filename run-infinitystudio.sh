#!/bin/zsh
set -euo pipefail

repo_dir="${0:A:h}"
studio_dir="$repo_dir/tools/installer-designer"
app_dir="$repo_dir/builds/InfinityOS Installer Studio.app"

echo "Cleaning InfinityStudio..."
swift package --package-path "$studio_dir" clean
rm -rf "$app_dir"

echo "Building InfinityStudio..."
"$studio_dir/build.sh"

echo "Launching InfinityStudio..."
open "$app_dir"
