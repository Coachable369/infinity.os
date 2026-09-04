#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output="$project_root/build/ai-test"

mkdir -p "$project_root/build"
rustc --edition=2021 -C opt-level=2 "$project_root/tools/ai-test.rs" -o "$output"
"$output"

if rg -n 'std::fs|File::|OpenOptions|file descriptor|Unix socket|serde_json|fork\(|exec\(' "$project_root/kernel/runtime/ai"; then
    echo 'FAIL: native AI runtime contains a traditional filesystem/process shortcut' >&2
    exit 1
fi

rg -Fq '/system/models/local-intent-v1' "$project_root/kernel/storage/object.rs"
rg -Fq 'model_object_valid' "$project_root/kernel/storage/object.rs"
rg -q 'SYSTEM_COMPONENT_REGISTRY: \[ComponentRegistration; (10|11)\]' "$project_root/kernel/storage/format.rs"
echo 'PASS architecture: AI state is native typed runtime state and native System objects, not files or subprocesses'
