#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
. tools/require-build-kit.sh
cmake -S tools/hermes-reference -B build/hermes-reference -DCMAKE_BUILD_TYPE=Release
cmake --build build/hermes-reference --parallel 6
rustc --edition=2021 -O tools/hermes-reference/tokenize.rs -o build/hermes-reference/native-tokenize
