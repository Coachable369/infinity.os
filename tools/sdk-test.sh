#!/bin/sh
set -eu
python3 tools/package-sdk.py
mkdir -p build/tests
rustc --edition=2021 -O -A warnings tools/object-store-test.rs -o build/tests/sdk-object-test
build/tests/sdk-object-test --sdk-only
python3 tools/sdk-package-test.py
