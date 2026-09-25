"""Alternating real-model comparison using binary results, not console prose."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import struct
import subprocess


# ------------------------=
# FUNC: read_result
# DESC: Validates the native timing artifact and returns numerical metrics plus exact generated content bytes.
# ------------------=
def read_result(path):
    raw = path.read_bytes()
    schema, count, total, length = struct.unpack_from("<4Q", raw)
    assert schema == 1 and 0 < count <= 32 and length > 0
    assert len(raw) == 32 + count * 8 + length
    times = struct.unpack_from(f"<{count}Q", raw, 32)
    assert all(a <= b for a, b in zip(times, times[1:])) and total >= times[-1] > 0
    content = raw[32 + count * 8:]
    return {"tokens": count, "first_ns": times[0], "last_ns": times[-1], "total_ns": total,
            "decode_ns": times[-1] - times[0], "sha256": hashlib.sha256(content).hexdigest()}, content


# ------------------------=
# FUNC: speed_ratio
# DESC: Reports no decode speedup when a one-token response has no steady-state interval.
# ------------------=
def speed_ratio(before, after):
    return before / after if after > 0 else None


# ------------------------=
# FUNC: main
# DESC: Alternates baseline and candidate with identical prompts and fresh caches, comparing complete binary generated results before reporting medians.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--trials", type=int, default=5)
    parser.add_argument("--prompt", default="hello")
    parser.add_argument("--baseline-pump", choices=("legacy", "deadline"))
    parser.add_argument("--candidate-pump", choices=("legacy", "deadline"))
    parser.add_argument("--baseline-workers", type=int, default=0)
    parser.add_argument("--candidate-workers", type=int, default=0)
    parser.add_argument("--io-delay-us", type=int, default=0)
    args = parser.parse_args()
    assert args.trials >= 3
    assert 0 <= args.io_delay_us <= 10000
    assert 0 <= args.baseline_workers <= 16 and 0 <= args.candidate_workers <= 16
    args.output.mkdir(parents=True, exist_ok=False)
    samples = {"baseline": [], "candidate": []}
    reference = None
    for trial in range(args.trials):
        order = ("baseline", "candidate") if trial % 2 == 0 else ("candidate", "baseline")
        for name in order:
            path = (args.output / f"{name}-{trial}.bin").resolve()
            env = dict(os.environ, HERMES_TEST_REPORT=str(path), HERMES_TEST_PROMPT=args.prompt)
            env["HERMES_TEST_WORKERS"] = str(getattr(args, f"{name}_workers"))
            for key in ("HERMES_TEST_PUMP", "HERMES_TEST_IO_US"):
                env.pop(key, None)
            pump = getattr(args, f"{name}_pump")
            if pump:
                env.update(HERMES_TEST_PUMP=pump, HERMES_TEST_IO_US=str(args.io_delay_us))
            with path.with_suffix(".log").open("wb") as log:
                subprocess.run([str(getattr(args, name).resolve()), "--forward"], env=env,
                               stdout=log, stderr=subprocess.STDOUT, check=True, timeout=180)
            measured, content = read_result(path)
            if reference is None:
                reference = content
            assert content == reference
            assert not samples["baseline"] or measured["tokens"] == samples["baseline"][0]["tokens"]
            samples[name].append(measured)
            print(json.dumps({"trial": trial, "variant": name, **measured}), flush=True)
    medians = {name: {key: statistics.median(item[key] for item in values)
                     for key in ("first_ns", "last_ns", "decode_ns", "total_ns")}
               for name, values in samples.items()}
    result = {"boundary": "controlled native ARM64 host; not installed guest timing",
              "workers": {"baseline": args.baseline_workers, "candidate": args.candidate_workers},
              "scheduler_experiment": {"baseline": args.baseline_pump, "candidate": args.candidate_pump,
                                       "injected_event_delay_us": args.io_delay_us},
              "trials": args.trials, "samples": samples, "medians": medians,
              "identical_generated_bytes": True,
              "speedups": {key: speed_ratio(medians["baseline"][key], medians["candidate"][key])
                           for key in medians["baseline"]}}
    (args.output / "result.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result["speedups"]), flush=True)


if __name__ == "__main__":
    main()
