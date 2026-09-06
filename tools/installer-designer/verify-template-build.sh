#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_dir="${script_dir:h:h}"
source_template="$repo_dir/assets/boot/installer-screens.infinityui"
runtime_template="$repo_dir/assets/boot/installer-screens.iuit"
configuration_source="$repo_dir/assets/boot/configuration-screens.infinityui"
configuration_runtime="$repo_dir/assets/boot/configuration-screens.iuit"
scratch_dir="$(mktemp -d)"
trap 'rm -rf "$scratch_dir"' EXIT HUP INT TERM

"$script_dir/compile-template.sh" "$source_template" "$scratch_dir/installer-screens.iuit" >/dev/null
cmp "$runtime_template" "$scratch_dir/installer-screens.iuit"
"$script_dir/compile-template.sh" "$configuration_source" "$scratch_dir/configuration-screens.iuit" >/dev/null
cmp "$configuration_runtime" "$scratch_dir/configuration-screens.iuit"
echo "Saved installer and configuration templates are the exact ISO runtime inputs: PASS"
