"""Paired complete-response measurements retaining real model history across turns."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import struct
import subprocess


# ------------------------=
# FUNC: receipt
# DESC: Reads structured native timing evidence and validates completion artifact dimensions.
# ------------------=
def receipt(path):
    raw = path.read_bytes()
    schema, count, total, length = struct.unpack_from("<4Q", raw)
    assert schema == 1 and 0 < count <= 32 and length > 0
    assert len(raw) == 32 + count * 8 + length
    times = struct.unpack_from(f"<{count}Q", raw, 32)
    assert total >= times[-1] > 0 and all(a <= b for a, b in zip(times, times[1:]))
    content = raw[32+count*8:]
    return {"tokens": count, "ttft_ns": times[0], "total_ns": total,
            "output_sha256": hashlib.sha256(content).hexdigest()}, content


# ------------------------=
# FUNC: main
# DESC: Alternates unchanged and optimized binaries, preserving conversation and checking exact output for every turn.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--turns", type=int, default=4)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--workers", type=int, default=0)
    args = parser.parse_args()
    assert 2 <= args.turns <= 16 and args.trials >= 3 and 0 <= args.workers <= 16
    args.output.mkdir(parents=True, exist_ok=False)
    samples = {"baseline": [], "candidate": []}
    reference = {}
    for trial in range(args.trials):
        for name in (("baseline", "candidate") if trial % 2 == 0 else ("candidate", "baseline")):
            path = (args.output / f"{name}-{trial}").resolve()
            env = dict(os.environ, HERMES_TEST_TURNS=str(args.turns),
                       HERMES_TEST_PROMPT="Reply with only the word Hello.",
                       HERMES_TEST_WORKERS=str(args.workers), HERMES_TEST_REPORT=str(path))
            env.pop("HERMES_TEST_PUMP", None)
            env.pop("HERMES_TEST_IO_US", None)
            with path.with_suffix(".log").open("wb") as log:
                subprocess.run([str(getattr(args, name).resolve()), "--forward"],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600)
            turns = []
            for turn in range(args.turns):
                result, content = receipt(Path(str(path)+f".turn{turn}"))
                assert reference.setdefault(turn, (content, result["tokens"])) == (content, result["tokens"])
                turns.append(result)
            samples[name].append(turns)
    summary = []
    for turn in range(args.turns):
        row = {name: {key: statistics.median(run[turn][key] for run in samples[name])
                      for key in ("ttft_ns", "total_ns")} for name in samples}
        row["total_reduction_percent"] = 100*(1-row["candidate"]["total_ns"]/row["baseline"]["total_ns"])
        summary.append(row)
    (args.output/"results.json").write_text(json.dumps({
        "boundary": "native host; not installed guest or final UI presentation",
        "prompt": env["HERMES_TEST_PROMPT"], "workers": args.workers,
        "model": "Hermes-3-Llama-3.2-3B.Q4_K_M", "samples": samples, "turn_medians": summary}, indent=2)+"\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
