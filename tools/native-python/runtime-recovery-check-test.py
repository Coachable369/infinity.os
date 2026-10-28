#!/usr/bin/env python3
"""Constructor/provenance behavior only; never execute candidate or verifier code on the host."""
from pathlib import Path
import ast
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: rejected
# DESC: Requires typed constructor rejection without interpreting diagnostic prose.
# ------------------=
def rejected(function, *arguments):
    try:
        function(*arguments)
    except (ValueError, FileExistsError):
        return
    raise AssertionError("Invalid recovery verification input accepted")


# ------------------------=
# FUNC: data_literals
# DESC: Extracts only literal byte/hash fields from the generated wrapper, never evaluating candidate or scenario code.
# ------------------=
def data_literals(script):
    tree = ast.parse(script)
    return [ast.literal_eval(node.value) for node in tree.body[:4]]


# ------------------------=
# FUNC: main
# DESC: Checks immutable snapshots, independent hashes, size/path bounds, input replacement independence and native-only execution rejection.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    spec = importlib.util.spec_from_file_location(
        "recovery_constructor", Path(__file__).with_name("runtime-recovery-check.py"))
    generator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(generator)
    source = b"invalid source syntax\\\x00\xff"
    scenario = b"invalid verifier syntax\\\x00\xfe"
    script, binding = generator.wrapper(source, scenario)
    assert data_literals(script) == [
        source, hashlib.sha256(source).hexdigest(),
        scenario, hashlib.sha256(scenario).hexdigest()]
    assert binding["wrapper_sha256"] == hashlib.sha256(script).hexdigest()
    assert binding["candidate_bytes"] == len(source)
    assert binding["scenario_bytes"] == len(scenario)
    assert not binding["candidate_executed_on_host"]
    assert not binding["scenario_executed_on_host"]
    assert not binding["domain_behavior_verified"]
    assert binding["guest_execution_pending"]
    _, changed_source = generator.wrapper(source + b"x", scenario)
    _, changed_scenario = generator.wrapper(source, scenario + b"x")
    assert changed_source["candidate_sha256"] != binding["candidate_sha256"]
    assert changed_source["scenario_sha256"] == binding["scenario_sha256"]
    assert changed_scenario["candidate_sha256"] == binding["candidate_sha256"]
    assert changed_scenario["scenario_sha256"] != binding["scenario_sha256"]
    assert changed_source["wrapper_sha256"] != binding["wrapper_sha256"]
    assert changed_scenario["wrapper_sha256"] != binding["wrapper_sha256"]
    bounded, maximum = generator.wrapper(b"x" * generator.SOURCE_LIMIT, b"y" * generator.SCENARIO_LIMIT)
    assert maximum["wrapper_bytes"] == len(bounded) <= generator.WRAPPER_LIMIT
    for candidate, verifier in [
        (b"", b"x"), (b"x", b""), ("x", b"x"), (b"x", None),
        (b"x" * (generator.SOURCE_LIMIT + 1), b"x"),
        (b"x", b"x" * (generator.SCENARIO_LIMIT + 1)),
        (b"\xff" * generator.SOURCE_LIMIT, b"\xfe" * generator.SCENARIO_LIMIT),
    ]:
        rejected(generator.wrapper, candidate, verifier)
    parent = ROOT / "build/behavior-tests/runtime-recovery-construction"
    parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="case-", dir=parent) as directory:
        work = Path(directory)
        candidate_path = work / "input.py"
        verifier_path = work / "checks.py"
        candidate_marker = work / "candidate-must-not-run"
        verifier_marker = work / "verifier-must-not-run"
        candidate = f"open({str(candidate_marker)!r}, 'w').write('unexpected')\n".encode()
        verifier = f"open({str(verifier_marker)!r}, 'w').write('unexpected')\n".encode()
        candidate_path.write_bytes(candidate)
        verifier_path.write_bytes(verifier)
        output = work / "bundle"
        report = generator.write_bundle(candidate_path, verifier_path, output)
        assert json.loads((output / "binding.json").read_text()) == report
        assert (output / "candidate.py").read_bytes() == candidate
        assert (output / "trusted-verifier.py").read_bytes() == verifier
        wrapper = (output / "guest.py").read_bytes()
        assert data_literals(wrapper) == [
            candidate, report["candidate_sha256"], verifier, report["scenario_sha256"]]
        for name in ("candidate.py", "trusted-verifier.py", "guest.py"):
            assert (output / name).stat().st_mode & 0o222 == 0
        candidate_path.write_bytes(b"changed candidate")
        verifier_path.write_bytes(b"changed verifier")
        assert (output / "candidate.py").read_bytes() == candidate
        assert (output / "trusted-verifier.py").read_bytes() == verifier
        # Only the wrapper's native-platform guard runs on the host. Never
        # spoof sys.platform or execute either embedded input to test behavior.
        assert sys.platform != "infinityos"
        guarded = subprocess.run([sys.executable, "-I", "-B", str(output / "guest.py")],
                                 capture_output=True, check=False)
        assert guarded.returncode == 126
        assert not candidate_marker.exists() and not verifier_marker.exists()
        rejected(generator.write_bundle, candidate_path, verifier_path, output)
        rejected(generator.write_bundle, candidate_path, candidate_path, work / "same-input")
        assert not (work / "same-input").exists()
        rejected(generator.write_bundle, candidate_path, verifier_path, Path("/"))
        rejected(generator.read_input, Path("/dev/null"), generator.SOURCE_LIMIT)
        for name in ("invoice", "calendar"):
            fixtures = Path(__file__).with_name("runtime-recovery-testcases")
            case = work / name
            proof = generator.write_bundle(fixtures / f"{name}.py", fixtures / f"{name}-scenario.py", case)
            assert proof["candidate_sha256"] == hashlib.sha256((fixtures / f"{name}.py").read_bytes()).hexdigest()
            assert proof["scenario_sha256"] == hashlib.sha256((fixtures / f"{name}-scenario.py").read_bytes()).hexdigest()
            assert not proof["domain_behavior_verified"] and proof["guest_execution_pending"]


if __name__ == "__main__":
    main()
