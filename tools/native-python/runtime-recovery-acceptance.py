#!/usr/bin/env python3
"""Bounded native failure -> actual Hermes repair -> native functional acceptance.

Invoke through build-kit. Candidates and trusted scenarios never execute on the
host. Native process proof is not installed-OS, microphone, or GUI acceptance.
"""
from pathlib import Path
import argparse
import datetime
import hashlib
import json
import os
import signal
import struct
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tools/native-python/runtime-recovery-testcases"
PROBE_WORK = ROOT / "build/native-process-probe/aarch64-gic2-compiler"
PROBE_EVIDENCE = ROOT / "builds/validation/native-python/aarch64"
CASES = (
    dict(name="invoice", exception="value", exception_class=1, line=15,
         objective="Create a Python invoice CSV calculator app that totals quantities and decimal "
         "unit prices exactly, accepts quoted descriptions, and rejects invalid numeric input."),
    dict(name="calendar", exception="type", exception_class=2, line=10,
         objective="Create a Python calendar sorting app that orders dated events chronologically, "
         "puts undated events last, preserves tie order and input data, and rejects invalid dates."),
)


# ------------------------=
# FUNC: require
# DESC: Rejects invalid structured acceptance evidence independently of console prose.
# ------------------=
def require(condition, message):
    if not condition:
        raise ValueError(message)


# ------------------------=
# FUNC: digest
# DESC: Hashes exact retained bytes without compiling or executing application source.
# ------------------=
def digest(data):
    return hashlib.sha256(data).hexdigest()


# ------------------------=
# FUNC: file_digest
# DESC: Streams a repository artifact into its identity hash without loading a large interpreter into memory.
# ------------------=
def file_digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


# ------------------------=
# FUNC: relative
# DESC: Records repository-local artifact locations without exposing an unrelated filesystem path.
# ------------------=
def relative(path):
    return str(path.resolve().relative_to(ROOT))


# ------------------------=
# FUNC: save_json
# DESC: Atomically updates structured evidence inside the active unique build directory.
# ------------------=
def save_json(path, value):
    pending = path.with_name(path.name + ".pending")
    pending.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    pending.replace(path)


# ------------------------=
# FUNC: snapshot
# DESC: Writes a fresh read-only evidence snapshot and returns its byte identity.
# ------------------=
def snapshot(path, data):
    with path.open("xb") as stream:
        stream.write(data)
    path.chmod(0o444)
    return dict(path=relative(path), sha256=digest(data), bytes=len(data))


# ------------------------=
# FUNC: stamp
# DESC: Distinguishes current probe artifacts from stale receipts left by earlier runs.
# ------------------=
def stamp(path):
    if not path.is_file():
        return None
    state = path.stat()
    return (state.st_ino, state.st_mtime_ns, state.st_ctime_ns, state.st_size)


# ------------------------=
# FUNC: run_step
# DESC: Runs one bounded build-kit child, retaining output for diagnostics without using its text as an oracle.
# ------------------=
def run_step(command, directory, timeout):
    directory.mkdir(parents=True, exist_ok=False)
    result = dict(command=[str(part) for part in command], timeout_seconds=timeout,
                  started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  finished=False, timed_out=False, returncode=None)
    save_json(directory / "command.json", result)
    environment = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "build/behavior-harness"),
                       PYTHONDONTWRITEBYTECODE="1")
    started = time.monotonic()
    with (directory / "stdout.log").open("xb") as output, (directory / "stderr.log").open("xb") as errors:
        process = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=output,
                                   stderr=errors, start_new_session=True)
        try:
            result["returncode"] = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            result["timed_out"] = True
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            result["returncode"] = process.returncode
        finally:
            result["finished"] = True
            result["elapsed_seconds"] = round(time.monotonic() - started, 3)
            save_json(directory / "command.json", result)
    return result


# ------------------------=
# FUNC: decode_native
# DESC: Independently checks the native binary exit, exception envelope, resource reclamation and diagnostic bytes.
# ------------------=
def decode_native(data, expected_exit, case):
    require(len(data) >= 72, "Native receipt is incomplete")
    marker, status, written, calls, free, initial, length = struct.unpack_from("<7Q", data)
    require(marker == 0x434C414E47454C30 and status == expected_exit,
            "Unexpected native execution marker or exit status")
    require(calls > 1 and 0 < free == initial and length <= 65536,
            "Native execution did not yield or reclaim its resources")
    offset = 56 + (length + 7) // 8 * 8
    require(len(data) >= offset + 16, "Native output envelope is incomplete")
    output_length, = struct.unpack_from("<Q", data, offset)
    extra = 40 if case else 0
    require(output_length <= 65536
            and len(data) == offset + 16 + (output_length + 7) // 8 * 8 + extra,
            "Unexpected native Python output size or trailing evidence")
    app_status, = struct.unpack_from("<Q", data, len(data) - extra - 8)
    require(app_status == (1 << 64) - 1,
            "Python receipt unexpectedly includes a linked native application result")
    require(written == length + output_length,
            "Native console byte count differs from its stdout and stderr payloads")
    if case:
        magic, count, category, kind, line = struct.unpack_from("<5Q", data, len(data) - 40)
        require((magic, count, category, kind, line) ==
                (0x5059455843455031, 1, 1, case["exception_class"], case["line"]),
                "Native exception does not match the original failure")
        require(length > 0, "Native exception has no retained diagnostics")
    return dict(exit_status=status, console_bytes=written, poll_visits=calls,
                memory_reclaimed=True), data[56:56 + length], data[offset + 8:offset + 8 + output_length]


# ------------------------=
# FUNC: native_probe
# DESC: Runs exact immutable Python bytes in the ARM guest and immediately preserves and validates all native proof.
# ------------------=
def native_probe(source, directory, compiler, compiler_sha256, case=None):
    require(file_digest(compiler) == compiler_sha256, "Native Python interpreter changed during acceptance")
    source_data = source.read_bytes()
    stage = "python-runtime-exception-" + case["exception"] if case else "python-script"
    paths = {
        "receipt.json": PROBE_EVIDENCE / (stage + ".json"),
        "receipt.bin": PROBE_EVIDENCE / (stage + ".bin"),
        "compiler-console.log": PROBE_WORK / "compiler-console.log",
        "raw-result.bin": PROBE_WORK / "result.bin",
        "qemu.log": PROBE_WORK / "qemu.log",
    }
    before = {name: stamp(path) for name, path in paths.items()}
    command = [sys.executable, str(ROOT / "tools/native-process-probe/run.py"),
               "--arch", "aarch64", "--compiler", str(compiler), "--python-script",
               "--python-source", str(source), "--python-exit", "1" if case else "0"]
    if case:
        command += ["--python-runtime-exception", case["exception"],
                    "--python-exception-line", str(case["line"])]
    result = run_step(command, directory, 900)
    artifacts = {}
    for name, path in paths.items():
        if stamp(path) is not None and stamp(path) != before[name]:
            artifacts[name] = snapshot(directory / name, path.read_bytes())
    save_json(directory / "artifacts.json", artifacts)
    require(result["returncode"] == 0 and not result["timed_out"], "Native probe failed; retained step evidence")
    require(all(name in artifacts for name in ("receipt.json", "receipt.bin", "compiler-console.log", "raw-result.bin")),
            "Native proof is missing or stale")
    report = json.loads((directory / "receipt.json").read_bytes())
    data = (directory / "receipt.bin").read_bytes()
    require(data == (directory / "raw-result.bin").read_bytes(), "Published native receipt differs from the guest binary")
    observed, diagnostics, stdout = decode_native(data, 1 if case else 0, case)
    require(diagnostics == (directory / "compiler-console.log").read_bytes(),
            "Retained diagnostics do not match the binary native receipt")
    artifacts["stdout.bin"] = snapshot(directory / "stdout.bin", stdout)
    save_json(directory / "artifacts.json", artifacts)
    for key in ("real_native_toolchain", "hardware_user_mode", "memory_reclaimed", "native_python_script"):
        require(report.get(key) is True, "Native receipt lacks " + key)
    require(report.get("architecture") == "aarch64" and report.get("installed_acceptance") is False,
            "Unexpected native execution scope")
    for key, value in observed.items():
        require(report.get(key) == value, "Native JSON and binary disagree: " + key)
    require(report.get("python_source_sha256") == digest(source_data)
            and report.get("python_source_bytes") == len(source_data),
            "Native proof is not bound to the supplied exact source")
    native_snapshot = (ROOT / report["python_source_snapshot"]).resolve()
    require(native_snapshot.is_relative_to(PROBE_WORK) and native_snapshot.read_bytes() == source_data,
            "Compiled Python snapshot changed or escaped its native probe directory")
    if case:
        require(report.get("runtime_exception") == dict(reported=True, category=1,
                exception_class=case["exception_class"], source_line=case["line"], exact_source_bound=True),
                "Typed exception receipt mismatches the observed native failure")
    require(source.read_bytes() == source_data and file_digest(compiler) == compiler_sha256,
            "An acceptance input changed during native execution")
    return dict(source_sha256=digest(source_data), source_bytes=len(source_data),
                receipt=artifacts["receipt.json"], binary=artifacts["receipt.bin"],
                diagnostics=artifacts["compiler-console.log"], stdout=artifacts["stdout.bin"],
                stdout_bytes=len(stdout), stderr_bytes=len(diagnostics), native=observed)


# ------------------------=
# FUNC: model_repair
# DESC: Uses actual Hermes and production atomic repair with bounded validation repairs, never executing candidate source on the host.
# ------------------=
def model_repair(case, source, diagnostics, directory):
    model_output = directory / "model"
    command = ["cargo", "run", "--release", "--quiet", "--manifest-path", "tools/behavior-harness/Cargo.toml",
               "--bin", "application-interaction-model-test", "--",
               "--repair-runtime-source", str(source), "--runtime-diagnostics", str(diagnostics),
               "--generate", case["objective"], "--model", "hermes", "--target", "aarch64",
               "--deadline-seconds", "600", "--repair-rounds", "1", "--output-dir", str(model_output)]
    result = run_step(command, directory / "model-command", 1500)
    require(result["returncode"] == 0 and not result["timed_out"], "Bounded model repair failed; retained step evidence")
    report = json.loads((model_output / "result.json").read_bytes())
    for key in ("generation_complete", "validation_passed", "lease_released", "next_turn_admitted",
                "next_turn_cancelled", "no_auto_launch", "atomic_replacement_verified"):
        require(report.get(key) is True, "Model repair did not prove " + key)
    require(report.get("failure") is None and report.get("mode") == "runtime-repair"
            and report.get("model") == "hermes" and report.get("source_language") == "python"
            and report.get("diagnostics_provenance") == "supplied-external-evidence"
            and report.get("host_fixture_source_identity") is True,
            "Unexpected runtime repair evidence")
    require(report.get("native_runtime_verified") is False and report.get("domain_behavior_verified") is False
            and report.get("installed_acceptance") is False, "Model probe overclaimed native functional verification")
    require(report.get("repair_limit") == 1 and type(report.get("attempts")) is int
            and 1 <= report["attempts"] <= 2 and type(report.get("runtime_repair_attempts")) is int
            and 1 <= report["runtime_repair_attempts"] <= 2, "Model repair exceeded its bounded attempts")
    require((model_output / "source-before.py").read_bytes() == source.read_bytes(),
            "Model repair changed the original saved source identity")
    require((model_output / "runtime-diagnostics.bin").read_bytes() == diagnostics.read_bytes(),
            "Model did not receive the retained actual native diagnostics")
    candidate = model_output / "source-after.py"
    data = candidate.read_bytes()
    require(0 < len(data) <= 16384 and data.rstrip(b" \t\r\n\v\f") != source.read_bytes().rstrip(b" \t\r\n\v\f"),
            "Model produced no bounded changed source")
    retained = directory / "repaired.py"
    identity = snapshot(retained, data)
    return retained, dict(source=identity, report=relative(model_output / "result.json"),
                          attempts=report["attempts"], runtime_repair_attempts=report["runtime_repair_attempts"],
                          atomic_replacement_verified=True, no_auto_launch=True, lease_released=True,
                          next_turn_admitted=True, next_turn_cancelled=True)


# ------------------------=
# FUNC: functional_bundle
# DESC: Constructs an independent immutable guest verifier and checks its provenance without executing its candidate or scenario.
# ------------------=
def functional_bundle(candidate, scenario, directory):
    bundle = directory / "functional-bundle"
    command = [sys.executable, str(ROOT / "tools/native-python/runtime-recovery-check.py"),
               "--source", str(candidate), "--scenario", str(scenario), "--output-dir", str(bundle)]
    result = run_step(command, directory / "bundle-command", 30)
    require(result["returncode"] == 0 and not result["timed_out"], "Guest verification bundle construction failed")
    binding = json.loads((bundle / "binding.json").read_bytes())
    for key, path in (("candidate", candidate), ("scenario", scenario), ("wrapper", bundle / "guest.py")):
        data = path.read_bytes()
        require(binding.get(key + "_sha256") == digest(data) and binding.get(key + "_bytes") == len(data),
                "Functional verifier input binding changed: " + key)
    require((bundle / "candidate.py").read_bytes() == candidate.read_bytes()
            and (bundle / "trusted-verifier.py").read_bytes() == scenario.read_bytes(),
            "Functional checks are not separate exact snapshots")
    for key in ("candidate_executed_on_host", "scenario_executed_on_host",
                "candidate_receives_verifier_namespace", "domain_behavior_verified", "installed_acceptance"):
        require(binding.get(key) is False, "Unexpected verifier execution scope: " + key)
    require(binding.get("trusted_verifier_separate") is True and binding.get("guest_execution_pending") is True
            and binding.get("source_namespace") == "__main__", "Functional verifier execution contract changed")
    return bundle / "guest.py", binding


# ------------------------=
# FUNC: main
# DESC: Runs each generic application once through native failure, real bounded repair and independent native functional verification.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/native-python/runtime-recovery-acceptance.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", type=Path, default=ROOT / "builds/toolchains/aarch64/python.elf")
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    require(compiler.is_relative_to(ROOT) and compiler.is_file(), "Use a repository-owned native Python ELF")
    stamp_name = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output = ROOT / "build/runtime-recovery" / (stamp_name + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, exist_ok=False)
    compiler_identity = dict(path=relative(compiler), sha256=file_digest(compiler), bytes=compiler.stat().st_size)
    aggregate = dict(schema_version=1, success=False, architecture="aarch64", model="hermes",
                     compiler=compiler_identity, native_functional_acceptance=False,
                     installed_acceptance=False, microphone_acceptance=False, gui_acceptance=False,
                     candidate_executed_on_host=False, automatic_application_launch=False,
                     failure=None, cases=[])
    report_path = output / "acceptance.json"
    save_json(report_path, aggregate)
    try:
        for case in CASES:
            directory = output / case["name"]
            directory.mkdir()
            original = directory / "original.py"
            scenario = directory / "trusted-scenario.py"
            source_identity = snapshot(original, (FIXTURES / (case["name"] + ".py")).read_bytes())
            verifier_identity = snapshot(scenario, (FIXTURES / (case["name"] + "-scenario.py")).read_bytes())
            evidence = dict(name=case["name"], objective=case["objective"], success=False, phase="native-original",
                            original=source_identity, verifier=verifier_identity)
            aggregate["cases"].append(evidence)
            save_json(report_path, aggregate)
            evidence["original_failure"] = native_probe(original, directory / "native-original",
                                                         compiler, compiler_identity["sha256"], case)
            evidence["phase"] = "model-repair"
            save_json(report_path, aggregate)
            candidate, evidence["repair"] = model_repair(
                case, original, directory / "native-original/compiler-console.log", directory)
            evidence["phase"] = "native-functional-verification"
            save_json(report_path, aggregate)
            guest, binding = functional_bundle(candidate, scenario, directory)
            evidence["binding"] = binding
            save_json(report_path, aggregate)
            evidence["functional_verification"] = native_probe(
                guest, directory / "native-functional", compiler, compiler_identity["sha256"])
            require(evidence["functional_verification"]["source_sha256"] == binding["wrapper_sha256"]
                    and file_digest(candidate) == binding["candidate_sha256"]
                    and file_digest(scenario) == binding["scenario_sha256"],
                    "Native functional proof does not match the repaired source and trusted scenario")
            evidence.update(success=True, phase="complete", native_functional_acceptance=True,
                            installed_acceptance=False)
            save_json(report_path, aggregate)
        require(len(aggregate["cases"]) == len(CASES) and all(case["success"] for case in aggregate["cases"]),
                "Not all independent applications passed")
        aggregate.update(success=True, native_functional_acceptance=True)
    except Exception as error:
        aggregate["failure"] = dict(type=type(error).__name__, detail=str(error))
    finally:
        save_json(report_path, aggregate)
        print(json.dumps(dict(success=aggregate["success"], report=str(report_path),
                              evidence_directory=str(output)), sort_keys=True))
    return 0 if aggregate["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
