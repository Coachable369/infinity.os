#!/usr/bin/env python3
"""Generate a guest-only, exact-source line-input smoke wrapper; never run the app here.

The downstream native-process probe records the wrapper's actual exit status.
Normal return uses 32 | the number of complete input lines consumed (0..15).
Abnormal return uses 128 | (exception category << 4) | consumed lines.
This is not a game-rules test, keyboard/device test, or installed-OS acceptance.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os

ROOT = Path(__file__).resolve().parents[2]
SOURCE_LIMIT = 128 * 1024
CATEGORIES = {
    "ValueError": 1, "TypeError": 2, "IndexError": 3, "EOFError": 4,
    "nonzero_SystemExit": 5, "other_Exception": 6, "other_BaseException": 7,
}

GUEST = '''"""Native-only smoke runner; embedded candidate bytes are not modified."""
import sys as _smoke_sys
if _smoke_sys.platform != "infinityos":
    raise SystemExit(126)
import hashlib as _smoke_hashlib
import io as _smoke_io
import traceback as _smoke_traceback
if _smoke_hashlib.sha256(_smoke_source).hexdigest() != _smoke_digest:
    raise SystemExit(127)
_smoke_stream = _smoke_io.StringIO(_smoke_input)
_smoke_original_stdin = _smoke_sys.stdin
_smoke_sys.stdin = _smoke_stream
_smoke_category = 0
try:
    exec(compile(_smoke_source, "/candidate.py", "exec"),
         {"__name__": "__main__", "__file__": "/candidate.py"})
except SystemExit as _smoke_error:
    if _smoke_error.code not in (None, 0):
        _smoke_category = 5
        _smoke_traceback.print_exc()
except BaseException as _smoke_error:
    _smoke_category = (
        1 if isinstance(_smoke_error, ValueError) else
        2 if isinstance(_smoke_error, TypeError) else
        3 if isinstance(_smoke_error, IndexError) else
        4 if isinstance(_smoke_error, EOFError) else
        6 if isinstance(_smoke_error, Exception) else 7)
    _smoke_traceback.print_exc()
finally:
    _smoke_sys.stdin = _smoke_original_stdin
try:
    _smoke_consumed = _smoke_input[:_smoke_stream.tell()].count("\\n")
except BaseException:
    raise SystemExit(127)
if not 0 <= _smoke_consumed <= 15:
    raise SystemExit(127)
raise SystemExit(
    (128 | (_smoke_category << 4) | _smoke_consumed)
    if _smoke_category else (32 | _smoke_consumed))
'''


# ------------------------=
# FUNC: wrapper
# DESC: Embeds immutable bytes and controlled input without parsing or evaluating candidate code on the host.
# ------------------=
def wrapper(source, inputs):
    if len(source) > SOURCE_LIMIT:
        raise ValueError("Candidate exceeds the bounded smoke source capacity")
    if not 1 <= len(inputs) <= 15 or any(
            not isinstance(line, str) or len(line) > 256 or "\n" in line or "\r" in line
            for line in inputs):
        raise ValueError("Supply 1..15 individual input lines, each at most 256 characters")
    stdin = "".join(line + "\n" for line in inputs)
    digest = hashlib.sha256(source).hexdigest()
    # repr creates data literals; it does not compile or evaluate source.
    script = (f"_smoke_source = {source!r}\n_smoke_digest = {digest!r}\n"
              f"_smoke_input = {stdin!r}\n" + GUEST).encode("utf-8")
    return script, dict(
        schema_version=1, candidate_sha256=digest, candidate_bytes=len(source),
        wrapper_sha256=hashlib.sha256(script).hexdigest(),
        stdin_sha256=hashlib.sha256(stdin.encode("utf-8")).hexdigest(),
        stdin_lines=inputs, exception_categories=CATEGORIES,
        exit_encoding="normal: 32 | consumed lines; abnormal: 128 | (category << 4) | consumed lines",
        reserved_exits={"126": "not native InfinityOS", "127": "harness integrity failure"},
        candidate_executed_on_host=False, guest_execution_pending=True,
        input_scope="guest StringIO stdin; no keyboard or console-device claim",
        source_scope="exact embedded bytes, fresh main namespace, /candidate.py diagnostic filename",
        installed_acceptance=False, domain_behavior_verified=False)


# ------------------------=
# FUNC: main
# DESC: Creates fresh repository-owned wrapper and binding receipts for a later native interpreter probe.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/application-smoke.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--input", action="append", required=True, dest="inputs")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    source, output = args.source.resolve(), args.output_dir.resolve()
    if not source.is_relative_to(ROOT) or not source.is_file() \
            or not output.is_relative_to(ROOT / "build"):
        raise ValueError("Source and smoke artifacts must remain within the repository")
    with source.open("rb") as stream:
        raw = stream.read(SOURCE_LIMIT + 1)
    script, report = wrapper(raw, args.inputs)
    report["candidate_path"] = str(source.relative_to(ROOT))
    output.mkdir(parents=True, exist_ok=False)
    (output / "guest.py").write_bytes(script)
    (output / "binding.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(dict(wrapper=str(output / "guest.py"), binding=report)))


if __name__ == "__main__":
    main()
