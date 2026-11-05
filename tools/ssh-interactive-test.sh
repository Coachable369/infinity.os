#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
test "${INFINITY_BUILD_KIT_ACTIVE:-0}" = 1
work=build/ssh-interactive-$(date +%s)
mkdir -p "$work"
build/ssh-pty-env/bin/python tools/ssh-pty-server.py "$work" --port 22444 > "$work/server.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT INT TERM
for attempt in 1 2 3 4 5; do
    test ! -f "$work/ready.json" || break
    sleep 1
done
test -f "$work/ready.json"
cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin ssh-interactive-client-test -- "$work" 22444
wait "$server"
python3 - "$work" <<'PY'
import json,sys
from pathlib import Path
work=Path(sys.argv[1]);state=json.loads((work/'pty-events.json').read_text())
assert state['password'] and state['shell']
assert state['pty']==[91,37]
assert [103,41] in state['resizes']
assert state['input_bytes']>30
print('External SSH PTY, real shell execution, editing, interrupt and resize passed:',work)
PY
