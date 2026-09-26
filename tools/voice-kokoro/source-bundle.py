#!/usr/bin/env python3
"""Preserve exact native dependency sources and build adaptations for release preparation."""
from pathlib import Path
import hashlib
import json
import os
import tarfile

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: source_filter
# DESC: Excludes VCS internals and Python caches while retaining actual patched sources and notices.
# ------------------=
def source_filter(member):
    if any(part in (".git", "__pycache__") for part in Path(member.name).parts):
        return None
    member.uid = member.gid = member.mtime = 0
    member.uname = member.gname = ""
    return member


# ------------------------=
# FUNC: main
# DESC: Archives dependency trees plus native build recipes; explicitly does not certify whole-OS source compliance.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/source-bundle.py")
    paths = ["build/kokopop-port-audit", "build/voice-kokoro/llvm-src",
             "build/newlib-4.6.0.20260123", "sdk/compiler", "tools/voice-kokoro",
             "tools/voice-pocketsphinx", "third_party/patches/voice-kokoro"]
    paths += ["build/voice-kokoro/reference/_deps/" + name + "-src"
              for name in ("espeak", "ggml", "yyjson", "sonic-git", "doctest")]
    for path in paths:
        if not (ROOT / path).is_dir():
            raise RuntimeError(f"Missing required source tree: {path}")
    output = ROOT / "builds/voice-kokoro"
    output.mkdir(parents=True, exist_ok=True)
    archive = output / "native-dependency-sources.tar.gz"
    pending = archive.with_suffix(".partial")
    with tarfile.open(pending, "w:gz", compresslevel=6) as bundle:
        for path in paths:
            bundle.add(ROOT / path, arcname=path, filter=source_filter)
    with tarfile.open(pending, "r:gz") as bundle:
        # Verify archived source and license bytes against the actual build inputs.
        for path in ("tools/voice-kokoro/port.c", "tools/voice-kokoro/private.ld",
                     "build/voice-kokoro/reference/_deps/espeak-src/COPYING"):
            assert bundle.extractfile(path).read() == (ROOT / path).read_bytes()
    pending.replace(archive)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    evidence = dict(archive=archive.name, sha256=digest, source_roots=paths,
                    scope="native speech dependencies and port only",
                    whole_os_corresponding_source_complete=False,
                    installed_verified=False)
    (output / "source-bundle.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
