#!/bin/bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
app_dir="$repo_dir/builds/InfinityOS Installer Studio.app"
launch=true

case "${1:-}" in
    "") ;;
    --no-launch) launch=false ;;
    --help|-h)
        echo "Usage: $0 [--no-launch]"
        echo "Clean and rebuild Infinity Studio through the build kit, then launch it."
        echo "Preserves existing ISOs, models, and OS build caches."
        exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
    echo "Expected at most one option." >&2
    exit 2
fi
if [[ "$(uname -s)" != Darwin ]]; then
    echo "Infinity Studio requires macOS." >&2
    exit 1
fi

cd -- "$repo_dir"
echo "Cleaning and rebuilding Infinity Studio..."
"$repo_dir/build-kit" studio-clean
if [[ ! -x "$app_dir/Contents/MacOS/InfinityInstallerStudio" ]]; then
    echo "Build finished without the expected Studio executable: $app_dir" >&2
    exit 1
fi

if [[ "$launch" == true ]]; then
    echo "Launching Infinity Studio..."
    open "$app_dir"
else
    echo "Built: $app_dir"
fi
