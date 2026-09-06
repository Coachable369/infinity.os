#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
source_template="$repo_dir/assets/boot/installer-screens.infinityui"
runtime_template="$repo_dir/assets/boot/installer-screens.iuit"
scratch_dir="$(mktemp -d)"
trap 'rm -rf "$scratch_dir"' EXIT HUP INT TERM

"$script_dir/compile-template.sh" "$source_template" "$scratch_dir/installer-screens.iuit" >/dev/null
cmp "$runtime_template" "$scratch_dir/installer-screens.iuit"
echo "Saved installer template is the exact ISO runtime input: PASS"
