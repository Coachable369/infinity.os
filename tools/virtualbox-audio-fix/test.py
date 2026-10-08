"""Compile and execute actual pinned upstream capture functions before/after the patch."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request

REVISION = "32f5f1de3fe17a3df177d6b3259112bcbf79856e"
FILES = ("DrvAudio.cpp", "DrvHostAudioCoreAudio.cpp")

# ------------------------=
# FUNC: extract
# DESC: Selects an upstream function unchanged for compilation against deterministic backend fixtures.
# ------------------=
def extract(source, name):
    at = source.index(name + "(")
    start = source.rfind("\n", 0, at) + 1
    body = source.index("{", at)
    depth = 0
    for end in range(body, len(source)):
        depth += (source[end] == "{") - (source[end] == "}")
        if depth == 0:
            return source[start:end + 1]
    raise ValueError("Unterminated upstream function")

# ------------------------=
# FUNC: main
# DESC: Demonstrates the old non-progress failure and verifies patched byte transfers and termination.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1", "Use ./build-kit run"
    root = Path(__file__).resolve().parents[2]
    work = root / "build/virtualbox-audio-fix"
    upstream = work / "upstream/src/VBox/Devices/Audio"
    upstream.mkdir(parents=True, exist_ok=True)
    hashes = {}
    for name in FILES:
        target = upstream / name
        url = f"https://raw.githubusercontent.com/VirtualBox/virtualbox/{REVISION}/src/VBox/Devices/Audio/{name}"
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read()
        target.write_bytes(data)
        hashes[name] = hashlib.sha256(data).hexdigest()
    patched = work / "patched"
    shutil.copytree(work / "upstream", patched, dirs_exist_ok=True)
    subprocess.run(["patch", "--batch", "--fuzz=0", "-p1", "-i", str(root / "third_party/patches/virtualbox/audio-capture-progress.patch")], cwd=patched, check=True)
    binaries = {}
    for variant in ("upstream", "patched"):
        directory = work / variant
        audio = directory / "src/VBox/Devices/Audio"
        functions = extract((audio / FILES[0]).read_text(), "drvAudioStreamCaptureLocked")
        functions += "\n" + extract((audio / FILES[1]).read_text(), "drvHstAudCaHA_StreamCapture")
        (directory / "functions.inc").write_text(functions)
        executable = directory / "capture-test"
        subprocess.run(["clang++", "-std=c++17", "-O1", "-fsanitize=address,undefined", "-I", str(directory), str(Path(__file__).with_name("test.cpp")), "-o", str(executable)], check=True)
        binaries[variant] = executable
    old_single = subprocess.run([str(binaries["upstream"]), "single"], capture_output=True)
    assert old_single.returncode != 0, "Original single-frame defect was not reproduced"
    try:
        subprocess.run([str(binaries["upstream"]), "zero"], timeout=2, check=True, capture_output=True)
    except subprocess.TimeoutExpired:
        old_stalled = True
    else:
        raise AssertionError("Original zero-progress hang was not reproduced")
    try:
        subprocess.run([str(binaries["upstream"]), "integrated"], timeout=2, check=True, capture_output=True)
    except subprocess.TimeoutExpired:
        pass
    else:
        raise AssertionError("Original combined CoreAudio/connector hang was not reproduced")
    cases = ("zero", "partial", "error", "single", "tail", "empty", "disabled", "integrated")
    for case in cases:
        subprocess.run([str(binaries["patched"]), case], timeout=5, check=True)
    report = {"upstream_revision": REVISION, "sha256": hashes,
              "original_single_frame_failed": True, "original_zero_progress_timed_out": old_stalled,
              "original_combined_capture_timed_out": True,
              "patched_cases_passed": list(cases), "sanitizers": ["address", "undefined"],
              "installed_virtualbox_changed": False, "installed_vm_verified": False}
    (work / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
