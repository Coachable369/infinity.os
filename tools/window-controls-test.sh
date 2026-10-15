#!/bin/sh
set -eu
test -n "${INFINITY_BUILD_KIT_ACTIVE:-}"
mkdir -p build/behavior-tests
rustc --edition=2021 --test tools/window-controls-test.rs -o build/behavior-tests/window-controls-test
build/behavior-tests/window-controls-test --test-threads=1
rustc --edition=2021 --test tools/minimized-shelf-test.rs -o build/behavior-tests/minimized-shelf-test
build/behavior-tests/minimized-shelf-test
rustc --edition=2021 -A warnings tools/active-painter-test.rs -o build/behavior-tests/window-controls-painter
build/behavior-tests/window-controls-painter --window-controls-only
sh tools/infinity-ui-test.sh
