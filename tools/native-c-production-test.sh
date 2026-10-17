#!/bin/sh
set -eu
. "$(dirname "$0")/require-build-kit.sh"
cd "$INFINITY_PROJECT_ROOT"

out=build/native-c-production
mkdir -p "$out"

cc -std=c11 -Wall -Wextra -Werror -fno-builtin -Isdk/c/include \
    sdk/c/libc.c sdk/c/format.c tools/native-c-probe/libc-test.c \
    -o "$out/libc-test"
"$out/libc-test"

rustc --edition=2021 -A warnings tools/native-development-command-test.rs \
    -o "$out/command-test"
"$out/command-test"

rustc --edition=2021 -A warnings tools/native-workflow-voice-test.rs \
    -o "$out/workflow-voice-test"
"$out/workflow-voice-test"

sh tools/native-c-probe/build-hello.sh
