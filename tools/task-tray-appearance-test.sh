#!/bin/sh
set -eu
. tools/require-build-kit.sh
mkdir -p builds/.studio-work/behavior-tests
python3 tools/uikit-document-test.py
rustc --edition=2021 -Awarnings tools/task-tray-style-test.rs -o builds/.studio-work/behavior-tests/task-tray-style-test
builds/.studio-work/behavior-tests/task-tray-style-test
swift test --scratch-path builds/.studio-work/swift --cache-path builds/.studio-work/cache/swiftpm --config-path builds/.studio-work/cache/swiftpm-config --security-path builds/.studio-work/cache/swiftpm-security --package-path tools/installer-designer --filter UIKitStoreTests
