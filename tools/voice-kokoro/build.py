#!/usr/bin/env python3
"""Reconstruct and verify the native speech prototype under the repository build lock."""
from pathlib import Path
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: stage
# DESC: Runs a reproducible stage and retains its diagnostics inside the managed workspace.
# ------------------=
def stage(name, script):
    log = ROOT / "build/logs" / ("kokoro-" + name + ".log")
    log.parent.mkdir(parents=True, exist_ok=True)
    print("Kokoro stage: " + name, flush=True)
    with log.open("w") as output:
        result = subprocess.run([sys.executable, str(ROOT / script)], cwd=ROOT,
                                stdout=output, stderr=subprocess.STDOUT)
    if result.returncode:
        raise RuntimeError(f"Kokoro {name} failed; diagnostics: {log}")


# ------------------------=
# FUNC: main
# DESC: Holds one build-kit invocation across dependencies, linking and actual native synthesis verification.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/voice-kokoro/build.py")
    with ThreadPoolExecutor(max_workers=2) as pool:
        jobs = [pool.submit(stage, "reference", "tools/voice-kokoro/reference.py"),
                pool.submit(stage, "newlib", "tools/voice-pocketsphinx/build.py")]
        for job in jobs:
            job.result()
    for name, script in [("cxx", "prepare-native.py"), ("engine", "native-build.py"),
                         ("link", "link-native.py"), ("guest", "probe/run.py")]:
        stage(name, "tools/voice-kokoro/" + script)


if __name__ == "__main__":
    main()
