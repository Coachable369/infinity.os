#!/usr/bin/env python3
"""Compare complete PCM and native timer measurements, not diagnostic prose."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import struct

ROOT = Path(__file__).resolve().parents[3]
STATUSES = (0, 0, 2, 1, 2, 0, 0, 0, 0, 0)


# ------------------------=
# FUNC: decode
# DESC: Validates the native probe wire format and retains every measured output sample.
# ------------------=
def decode(data, allow_profile=False):
    offset = 0
    cases = []
    for case, expected in enumerate(STATUSES):
        if len(data) - offset < 144:
            raise ValueError("Truncated native case header")
        version, index, status, count, ticks, frequency, phase, heap, failed, fault = struct.unpack_from("<10Q", data, offset)
        offset += 144  # Includes eight diagnostic caller addresses.
        if (version, index, status) != (2, case, expected) or not frequency:
            raise ValueError(f"Invalid native result for case {case}")
        if failed or fault or not 0 < heap <= 1536 * 1024 * 1024:
            raise ValueError(f"Native memory failure for case {case}")
        if (expected == 0 and not 2400 <= count <= 720000) or (expected != 0 and count != 0):
            raise ValueError(f"Invalid frame count for case {case}")
        pcm = data[offset:offset + count * 2]
        if len(pcm) != count * 2 or (count and (not any(pcm) or not ticks)):
            raise ValueError(f"Truncated or silent PCM for case {case}")
        offset += len(pcm)
        cases.append(dict(case=case, frames=count, seconds=ticks / frequency,
                          heap_bytes=heap, pcm=pcm))
    if len(data) - offset < 8:
        raise ValueError("Missing native dependency record")
    notice_length, = struct.unpack_from("<Q", data, offset)
    offset += 8
    if not 0 < notice_length <= 1048576 or len(data) - offset != notice_length + 2072:
        raise ValueError("Invalid native trailer")
    notice = data[offset:offset + notice_length]
    offset += notice_length
    profile = struct.unpack_from("<256Q", data, offset)
    if any(profile) and not allow_profile:
        raise ValueError("Use uninstrumented release objects for latency comparison")
    dot, tiles, stack = struct.unpack_from("<3Q", data, offset + 2048)
    if dot != 8448 or tiles != 165 or not 0 <= stack < 1024 * 1024 - 4096:
        raise ValueError("Native numerical or worker stack acceptance failed")
    if any(cases[i]["pcm"] != cases[0]["pcm"] for i in (1, 5, 8)):
        raise ValueError("Non-deterministic short synthesis")
    if cases[7]["pcm"] != cases[9]["pcm"] or cases[7]["heap_bytes"] != cases[9]["heap_bytes"]:
        raise ValueError("Paragraph repeat or bounded lifetime failed")
    return cases, notice, stack


# ------------------------=
# FUNC: compare
# DESC: Requires byte-identical complete replies before reporting native synthesis speed or real-time factors.
# ------------------=
def compare(baseline, candidate):
    before, before_notice, before_stack = decode(baseline)
    after, after_notice, after_stack = decode(candidate)
    if before_notice != after_notice:
        raise ValueError("Dependency resources changed between measurements")
    for a, b in zip(before, after):
        if a["pcm"] != b["pcm"]:
            raise ValueError(f"Complete PCM changed in case {a['case']}")
        if b["heap_bytes"] > a["heap_bytes"]:
            raise ValueError(f"Native heap grew in case {a['case']}")
    groups = []
    for name, indices in (("short_warm", (1, 5, 8)), ("greeting", (6,)), ("paragraph", (7, 9))):
        old = statistics.median(before[i]["seconds"] for i in indices)
        new = statistics.median(after[i]["seconds"] for i in indices)
        frames = after[indices[0]]["frames"]
        duration = frames / 24000
        groups.append(dict(fixture=name, repetitions=len(indices), frames=frames,
                           audio_seconds=duration, baseline_seconds=old, candidate_seconds=new,
                           speedup=old / new, reduction_percent=100 * (1 - new / old),
                           realtime_factor=new / duration,
                           pcm_sha256=hashlib.sha256(after[indices[0]]["pcm"]).hexdigest()))
    return dict(environment="freestanding ARM64 guest", installed_verified=False,
                complete_pcm_identical=True, baseline_stack_bytes=before_stack,
                candidate_stack_bytes=after_stack, stack_measured=bool(before_stack and after_stack), fixtures=groups)


# ------------------------=
# FUNC: main
# DESC: Saves artifact-backed performance evidence and optionally enforces below-real-time synthesis for all measured phrases.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--require-realtime", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(ROOT / "build"):
        parser.error("Performance evidence must remain in the repository build directory")
    before, after = args.baseline.read_bytes(), args.candidate.read_bytes()
    evidence = compare(before, after)
    evidence.update(baseline_sha256=hashlib.sha256(before).hexdigest(),
                    candidate_sha256=hashlib.sha256(after).hexdigest())
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))
    if args.require_realtime and any(row["realtime_factor"] >= 1 for row in evidence["fixtures"]):
        raise SystemExit("Synthesis remains slower than real time for one or more fixtures")


if __name__ == "__main__":
    main()
