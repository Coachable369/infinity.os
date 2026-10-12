"""Real native compiler/linker execution; explicitly not installed-desktop acceptance."""
from pathlib import Path
import argparse
import hashlib
import json
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: main
# DESC: Runs real user-mode compilation and linking, verifies child exit, and preserves structured evidence outside disposable build products.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["aarch64", "x86_64"], required=True)
    parser.add_argument("--gic", type=int, choices=[2, 3], default=3)
    args = parser.parse_args()
    arm = args.arch == "aarch64"
    name = args.arch + (f"-gic{args.gic}" if arm else "") + "-compiler"
    work = ROOT / "build/native-process-probe" / name
    evidence = ROOT / "builds/validation/native-apps" / name
    binaries = ROOT / "build/native-c" / ("aarch64" if arm else "") / f"clang-build-infinity-{args.arch}/bin"
    command = [sys.executable, str(ROOT / "tools/native-process-probe/run.py"), "--arch", args.arch, "--gic", str(args.gic)]
    subprocess.run(command + ["--compiler", str(binaries / "clang.elf"), "--compile-source"], cwd=ROOT, check=True)
    subprocess.run(command + ["--compiler", str(binaries / "lld.elf"), "--link-object", str(work / "compiled.o")], cwd=ROOT, check=True)
    compile_proof = json.loads((work / "compile-evidence.json").read_text())
    link_proof = json.loads((work / "link-evidence.json").read_text())
    assert compile_proof["compiled_source"] and compile_proof["exit_status"] == 0
    assert link_proof["native_linked"] and link_proof["guest_linked_app_exit"] == 54
    assert compile_proof["memory_reclaimed"] and link_proof["memory_reclaimed"]
    evidence.mkdir(parents=True, exist_ok=True)
    artifacts = {}
    for name in ["compiled.o", "linked.elf", "compile-evidence.json", "link-evidence.json"]:
        source = work / name
        artifacts[name] = hashlib.sha256(source.read_bytes()).hexdigest()
        shutil.copyfile(source, evidence / name)
    report = dict(architecture=args.arch, compile=compile_proof, link=link_proof,
        fixture_storage=True, host_compilation_of_test_source=False,
        installed_acceptance=False, artifact_sha256=artifacts)
    (evidence / "pipeline.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))

if __name__ == "__main__":
    main()
