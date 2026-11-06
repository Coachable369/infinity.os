#!/bin/sh
set -eu
mkdir -p build/tests
rustc --edition=2021 --test kernel/runtime/ai/pipeline_log.rs -o build/tests/audio-pipeline-log-test
build/tests/audio-pipeline-log-test
rustc --edition=2021 -O tools/object-store-test.rs -o build/tests/object-store-test
build/tests/object-store-test --audio-log-only
