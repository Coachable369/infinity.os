"""Behavioral checks for packaged developer commands."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: run
# DESC: Executes an installed-source tool with deterministic stdin and captures its process result.
# ------------------=
def run(name, data, *args):
    return subprocess.run([sys.executable, str(ROOT / 'sdk/system' / (name + '.py')), *args], input=data, capture_output=True)

# ------------------------=
# FUNC: main
# DESC: Verifies numerical counts, complete digests, JSON semantics and invalid-document failure.
# ------------------=
def main():
    data = b'one two\nthree\n'
    result = run('wc', data)
    assert result.returncode == 0
    assert list(map(int, result.stdout.split()[:3])) == [2, 3, len(data)]
    result = run('sha256sum', data)
    assert result.returncode == 0
    assert bytes.fromhex(result.stdout.split()[0].decode()) == hashlib.sha256(data).digest()
    result = run('jsonfmt', b'{"b": 2, "a": [true, null]}', '--compact', '--sort-keys')
    assert result.returncode == 0
    assert json.loads(result.stdout) == {'b': 2, 'a': [True, None]}
    assert run('jsonfmt', b'{broken').returncode != 0
    print('Power tool behavior passed')

if __name__ == '__main__':
    main()
