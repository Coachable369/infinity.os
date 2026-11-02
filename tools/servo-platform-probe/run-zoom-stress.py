"""Run native ARM zoom/re-layout stress, or behaviorally validate fault-frame capture."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import shutil


# ------------------------=
# FUNC: main
# DESC: Executes the native fixture through its active build kit and asserts typed results, geometry and fault registers.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fault-capture-test", action="store_true")
    options = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    evidence = root / "builds/evidence/browser-zoom-crash-20261102" / (
        "fault-capture" if options.fault_capture_test else "native-stress")
    evidence.mkdir(parents=True, exist_ok=True)
    source_result = evidence / "result.json"
    source_result.unlink(missing_ok=True)
    environment = os.environ.copy()
    # Explicitly select this fixture even when another focused run set selectors.
    for key in tuple(environment):
        if key.startswith("INFINITY_BROWSER_") and key.endswith("_ONLY"):
            environment.pop(key)
    environment["INFINITY_BROWSER_INTERACTION_ONLY"] = "1"
    environment["INFINITY_BROWSER_ZOOM_STRESS"] = "1"
    environment["INFINITY_BROWSER_FAULT_CAPTURE_TEST"] = "1" if options.fault_capture_test else "0"
    result = subprocess.run([
        "python3", str(Path(__file__).with_name("run-engine-guest.py")),
        "--page-probe", "--evidence-dir", str(evidence),
    ], env=environment)
    if not source_result.is_file():
        print(json.dumps({"passed": False, "fixture_exit": result.returncode,
                          "reason": "Native fixture produced no fresh result"}))
        return 1
    report = json.loads(source_result.read_text())
    shutil.copyfile(root / "build/servo-platform-probe/engine-boot.elf", evidence / "engine-boot.elf")
    records = report["records"]
    registers = {row[2] - 0x100: row[3] for row in records
                 if row[:2] == [9, 2] and 0x100 <= row[2] < 0x124}
    if options.fault_capture_test:
        passed = (result.returncode != 0 and not report["timed_out"] and
                  len(registers) == 36 and registers[9] == 7 << 32 and
                  registers[10] == 0x1234 and registers[34] == 7 << 32 and
                  registers[33] & 0x40 != 0 and registers[32] != 0 and
                  registers[30] != 0 and registers[31] & 15 == 0 and
                  records[-1] == [9, 1, registers[32], registers[33]])
    else:
        completed = [row[3] for row in records if row[:3] == [9, 2, 0x601]]
        expected = [110, 125, 150, 175, 200, 250, 300, 300, 100, 90, 80, 75, 67, 50, 50, 100]
        passed = (result.returncode == 0 and report["passed"] and not registers and
                  completed == [(index << 32) | value for index, value in enumerate(expected)])
    summary = {"passed": passed, "fault_capture_test": options.fault_capture_test,
               "registers": registers, "native_result": str(evidence / "result.json")}
    (evidence / "zoom-result.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
