#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
source_template="${1:-$repo_dir/assets/boot/installer-screens.infinityui}"
runtime_template="${2:-$repo_dir/assets/boot/installer-screens.iuit}"

swift run \
    --package-path "$script_dir" \
    -c release \
    InfinityInstallerStudio \
    --compile-template "$source_template" "$runtime_template"
