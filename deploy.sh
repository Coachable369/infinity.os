#!/usr/bin/env bash
set -euo pipefail

# This entry point deploys only the public website, never the operating system.
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$ROOT/meetinfinityos.com/deploy.sh" "$@"
