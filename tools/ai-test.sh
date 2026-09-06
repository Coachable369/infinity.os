#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output="$project_root/build/ai-test"

mkdir -p "$project_root/build"
rustc --edition=2021 -C opt-level=2 "$project_root/tools/ai-test.rs" -o "$output"
"$output"
