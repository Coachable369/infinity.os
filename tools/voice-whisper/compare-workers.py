#!/usr/bin/env python3
"""Compare measured native speech-worker runs without claiming end-to-end latency."""
import argparse
import json
from pathlib import Path
from statistics import mean


# ------------------------=
# FUNC: compare
# DESC: Requires identical recognized bytes, synthesized PCM, CPU allocation and bounded resident memory before calculating recognition latency improvement.
# ------------------=
def compare(before, after):
    if before["cpus"] != after["cpus"] or before["online_workers"] != after["online_workers"]:
        raise ValueError("CPU allocation changed between measurements")
    for kind in (11, 1):
        old = [row for row in before["cases"] if row["kind"] == kind]
        new = [row for row in after["cases"] if row["kind"] == kind]
        if not old or len(old) != len(new):
            raise ValueError("Missing paired native outcomes")
        for left, right in zip(old, new):
            for field in ("case", "status", "frames", "pcm_fnv64"):
                if left[field] != right[field]:
                    raise ValueError("Recognition or complete synthesis output changed")
            if left["status"] != 0 or min(left["seconds"], right["seconds"]) <= 0:
                raise ValueError("Native operation failed or omitted its timer")
            if right["memory"]["committed"] > left["memory"]["committed"]:
                raise ValueError("Optimization increased the native heap high-water mark")
    if after["helper_completions"] <= before["helper_completions"]:
        raise ValueError("Recognition did not exercise additional helper work")
    old = [row["seconds"] for row in before["cases"] if row["kind"] == 11]
    new = [row["seconds"] for row in after["cases"] if row["kind"] == 11]
    return dict(scope="native recorded-audio recognition, not installed end-to-end conversation",
                before_seconds=old, after_seconds=new,
                mean_reduction_percent=100 * (1 - mean(new) / mean(old)),
                transcript_identical=True, synthesized_pcm_identical=True,
                heap_high_water_increased=False, cpus=after["cpus"])


# ------------------------=
# FUNC: main
# DESC: Reads the structured firmware-probe results and emits only comparisons that passed binary-output and resource checks.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    args = parser.parse_args()
    print(json.dumps(compare(json.loads(args.before.read_text()),
                             json.loads(args.after.read_text())), indent=2))


if __name__ == "__main__":
    main()
