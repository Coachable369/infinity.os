#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p build
rustc --edition=2021 -C opt-level=2 tools/milestone-7-test.rs -o build/milestone-7-test
build/milestone-7-test
if rg -n 'std::fs|File::open|OpenOptions|Command::new|/bin/sh|UnixStream|serde_json|serde_yaml' kernel/runtime/identity.rs kernel/runtime/console_language.rs; then
    echo "FAIL: filesystem, process, socket, JSON, or YAML shortcut detected" >&2
    exit 1
fi
if rg -n 'password.*write_line|secret.*write_line|verifier.*pub|salt.*pub' kernel/runtime/identity.rs kernel/core/console.rs; then
    echo "FAIL: credential secret or verifier exposure detected" >&2
    exit 1
fi
echo "PASS source audit: native object state, typed operations, and private credential handling"
