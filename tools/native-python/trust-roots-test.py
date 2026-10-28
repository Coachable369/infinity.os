#!/usr/bin/env python3
"""Verify an exact bundled certifi trust request cannot broaden native TLS trust."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: main
# DESC: Extracts only the already-pinned bundled PEM and verifies actual parsed anchor identity and constraints against the native verifier's root set.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/trust-roots-test.py")
    wheel = ROOT / "third_party/python/Python-3.13.16/Lib/ensurepip/_bundled/pip-26.2.1-py3-none-any.whl"
    work = ROOT / "build/native-python-trust"
    work.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(wheel) as bundle:
        content = bundle.read("pip/_vendor/certifi/cacert.pem")
    candidate = work / "certifi.pem"
    candidate.write_bytes(content)
    environment = dict(os.environ)
    environment["CARGO_TARGET_DIR"] = str(ROOT / "build/cargo/python-trust")
    command = ["cargo", "run", "--quiet", "--manifest-path", "kernel/runtime/http/Cargo.toml",
        "--example", "python-trust-roots", "--", str(candidate)]
    result = subprocess.run(command, cwd=ROOT, env=environment, check=True,
                            stdout=subprocess.PIPE, text=True)
    proof = json.loads(result.stdout)
    assert proof["native_subset"] and proof["exact_constraints"]
    assert 0 < proof["selected_anchors"] <= min(proof["requested_anchors"], proof["native_anchors"])
    proof["pem_sha256"] = hashlib.sha256(content).hexdigest()
    with wheel.open("rb") as stream:
        proof["wheel_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
    proof["scope"] = "exact immutable bundled PEM selects pinned native intersection; not arbitrary custom trust roots"
    destination = ROOT / "builds/validation/native-python"
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "trust-root-subset.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(proof))


if __name__ == "__main__":
    main()
