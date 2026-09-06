#!/bin/zsh
set -euo pipefail

script_dir="${0:A:h}"
"$script_dir/build.sh"
open "${script_dir:h:h}/builds/InfinityOS Installer Studio.app"
