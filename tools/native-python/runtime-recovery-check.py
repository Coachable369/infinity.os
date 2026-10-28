#!/usr/bin/env python3
"""Construct exact-source, guest-only functional checks; never execute candidates here.

The scenario is independently trusted test code, not generated application code.
This is an acceptance harness, not an adversarial same-interpreter Python sandbox.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os

ROOT = Path(__file__).resolve().parents[2]
SOURCE_LIMIT = 16 * 1024
SCENARIO_LIMIT = 16 * 1024
WRAPPER_LIMIT = 128 * 1024

GUEST = '''
import sys as _recovery_sys
if _recovery_sys.platform != "infinityos":
    raise SystemExit(126)

# ------------------------=
# FUNC: _recovery_run
# DESC: Runs immutable app bytes as main, then independent trusted checks in a separate namespace, exclusively in the native guest.
# ------------------=
def _recovery_run(source, source_digest, scenario, scenario_digest):
    import hashlib
    import types
    import builtins
    sha256 = hashlib.sha256
    compile_code = builtins.compile
    execute = builtins.exec
    system_exit = builtins.SystemExit
    if sha256(source).hexdigest() != source_digest or sha256(scenario).hexdigest() != scenario_digest:
        raise system_exit(127)
    trusted = {"__name__": "_infinity_recovery_verifier", "__file__": "/trusted-verifier.py"}
    execute(compile_code(scenario, "/trusted-verifier.py", "exec"), trusted)
    verify = trusted["verify"]
    minimum = trusted["MINIMUM_CHECKS"]
    if not callable(verify) or type(minimum) is not int or not 1 <= minimum <= 256:
        raise system_exit(127)
    checks = [0]

    # ------------------------=
    # FUNC: check
    # DESC: Counts only satisfied trusted functional predicates; failed or absent checks cannot return native success.
    # ------------------=
    def check(condition):
        if type(condition) is not bool or not condition:
            raise AssertionError("Application functional check failed")
        checks[0] += 1
        if checks[0] > 256:
            raise system_exit(127)

    application = types.ModuleType("__main__")
    application.__file__ = "/candidate.py"
    previous_main = _recovery_sys.modules.get("__main__")
    previous_argv = _recovery_sys.argv
    _recovery_sys.modules["__main__"] = application
    _recovery_sys.argv = ["/candidate.py"]
    try:
        try:
            execute(compile_code(source, "/candidate.py", "exec"), application.__dict__)
        except system_exit as exit_request:
            if exit_request.code not in (None, 0):
                raise
        if sha256(source).hexdigest() != source_digest or sha256(scenario).hexdigest() != scenario_digest:
            raise system_exit(127)
        verify(application.__dict__, check)
        if checks[0] < minimum:
            raise system_exit(127)
    finally:
        _recovery_sys.argv = previous_argv
        if previous_main is None:
            _recovery_sys.modules.pop("__main__", None)
        else:
            _recovery_sys.modules["__main__"] = previous_main

_recovery_run(_recovery_source, _recovery_source_digest, _recovery_scenario, _recovery_scenario_digest)
raise SystemExit(0)
'''


# ------------------------=
# FUNC: wrapper
# DESC: Binds exact source and separately trusted verification bytes without compiling, importing or executing either on the host.
# ------------------=
def wrapper(source, scenario):
    if not isinstance(source, bytes) or not 1 <= len(source) <= SOURCE_LIMIT:
        raise ValueError("Candidate must be bounded nonempty bytes")
    if not isinstance(scenario, bytes) or not 1 <= len(scenario) <= SCENARIO_LIMIT:
        raise ValueError("Trusted scenario must be bounded nonempty bytes")
    source_digest = hashlib.sha256(source).hexdigest()
    scenario_digest = hashlib.sha256(scenario).hexdigest()
    script = (f"_recovery_source = {source!r}\n"
              f"_recovery_source_digest = {source_digest!r}\n"
              f"_recovery_scenario = {scenario!r}\n"
              f"_recovery_scenario_digest = {scenario_digest!r}\n" + GUEST).encode("utf-8")
    if len(script) > WRAPPER_LIMIT:
        raise ValueError("Encoded native wrapper exceeds its bounded input capacity")
    return script, dict(
        schema_version=1, candidate_sha256=source_digest, candidate_bytes=len(source),
        scenario_sha256=scenario_digest, scenario_bytes=len(scenario),
        wrapper_sha256=hashlib.sha256(script).hexdigest(), wrapper_bytes=len(script),
        candidate_executed_on_host=False, scenario_executed_on_host=False,
        guest_execution_pending=True, domain_behavior_verified=False,
        installed_acceptance=False, trusted_verifier_separate=True,
        candidate_receives_verifier_namespace=False,
        source_namespace="__main__", source_filename="/candidate.py",
        verifier_filename="/trusted-verifier.py",
        native_success="exit 0 only after the independently trusted checks complete",
        reserved_exits={"126": "not native InfinityOS", "127": "harness integrity or check-count failure"},
        isolation_scope="separate immutable inputs and namespaces, not an adversarial Python sandbox")


# ------------------------=
# FUNC: read_input
# DESC: Reads one bounded repository-owned input snapshot and rejects files outside the project.
# ------------------=
def read_input(path, limit):
    path = path.resolve()
    if not path.is_relative_to(ROOT) or not path.is_file():
        raise ValueError("Acceptance inputs must be repository-owned files")
    with path.open("rb") as stream:
        data = stream.read(limit + 1)
    if not 1 <= len(data) <= limit:
        raise ValueError("Acceptance input is empty or exceeds its bound")
    return path, data


# ------------------------=
# FUNC: write_bundle
# DESC: Saves fresh read-only source, verifier and guest snapshots with hashes; none is executed during construction.
# ------------------=
def write_bundle(source_path, scenario_path, output):
    source_path, source = read_input(source_path, SOURCE_LIMIT)
    scenario_path, scenario = read_input(scenario_path, SCENARIO_LIMIT)
    output = output.resolve()
    if source_path == scenario_path or not output.is_relative_to(ROOT / "build"):
        raise ValueError("Use separate candidate/verifier files and a repository-local build directory")
    script, report = wrapper(source, scenario)
    report["candidate_path"] = str(source_path.relative_to(ROOT))
    report["scenario_path"] = str(scenario_path.relative_to(ROOT))
    output.mkdir(parents=True, exist_ok=False)
    for name, data in (("candidate.py", source), ("trusted-verifier.py", scenario), ("guest.py", script)):
        path = output / name
        with path.open("xb") as stream:
            stream.write(data)
        path.chmod(0o444)
    (output / "binding.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


# ------------------------=
# FUNC: main
# DESC: Creates a hash-bound guest verification bundle for a later native interpreter run.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/runtime-recovery-check.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--scenario", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    report = write_bundle(args.source, args.scenario, args.output_dir)
    print(json.dumps(dict(wrapper=str(args.output_dir.resolve() / "guest.py"), binding=report)))


if __name__ == "__main__":
    main()
