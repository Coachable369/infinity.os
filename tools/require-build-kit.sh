#!/bin/sh
# Shared precondition for subordinate build entrypoints.

if [ "${INFINITY_BUILD_KIT_ACTIVE:-0}" != "1" ]; then
    echo "ERROR: direct subordinate builds are disabled; use ./build-kit run $0 $*" >&2
    exit 64
fi
if [ -z "${INFINITY_PROJECT_ROOT:-}" ] || [ ! -f "$INFINITY_PROJECT_ROOT/build-kit.toml" ]; then
    echo "ERROR: build-kit project context is missing or invalid" >&2
    exit 64
fi
if [ "${TMPDIR:-}" != "$INFINITY_PROJECT_ROOT/build/tmp" ]; then
    echo "ERROR: build temporary storage is not repository-local" >&2
    exit 64
fi
