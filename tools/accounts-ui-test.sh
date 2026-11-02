#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
make build/behavior-tests/libinfinity_browser_core.rlib
rustc --edition=2021 --test tools/accounts-ui-state-test.rs \
    -o build/behavior-tests/accounts-ui-state-test
build/behavior-tests/accounts-ui-state-test
rustc --edition=2021 -C opt-level=z \
    --extern infinity_browser_core=build/behavior-tests/libinfinity_browser_core.rlib \
    tools/active-painter-test.rs -o build/behavior-tests/accounts-painter-test
build/behavior-tests/accounts-painter-test --settings-only
