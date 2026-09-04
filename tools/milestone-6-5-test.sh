#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p build
rustc --edition=2021 -C opt-level=2 tools/milestone-6-5-test.rs -o build/milestone-6-5-test
build/milestone-6-5-test
if rg -n 'std::fs|File::open|OpenOptions|Command::new|/bin/sh|UnixStream|serde_json' kernel/runtime/console_language.rs kernel/storage/organization.rs; then
    echo "FAIL: traditional filesystem, process, socket, or JSON shortcut detected" >&2
    exit 1
fi
echo "PASS source audit: native object/relationship model and typed Console graph contain no filesystem, shell, socket, or JSON shortcut"
