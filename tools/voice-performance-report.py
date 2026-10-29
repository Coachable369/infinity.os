#!/usr/bin/env python3
"""Decode content-free InfinityOS voice telemetry from a VM serial log."""
import argparse
import json
from pathlib import Path

PHASES = {
    1: "recognition_admission_wait", 2: "recognition_work",
    3: "speech_admission_wait", 4: "speech_synthesis", 5: "pcm_conversion",
    6: "observed_playback", 7: "observed_playback_gap", 8: "speech_engine_warmup",
    9: "intent_routing", 10: "personal_memory_update", 11: "personal_memory_context",
    12: "prompt_build_and_tokenize",
}
MILESTONES = ["endpoint", "transcript", "submitted", "first_text", "speech_queued",
              "first_audio", "drained", "capture_started", "recognition_ready",
              "first_display", "generation_done"]

# ------------------------=
# FUNC: duration
# DESC: Returns milliseconds only for present, correctly ordered clock observations.
# ------------------=
def duration(start, end):
    return (end - start) / 1e6 if start is not None and end is not None and end >= start else None

# ------------------------=
# FUNC: decode
# DESC: Parses versioned numeric records, retaining partial turns and rejecting malformed or unsupported rows.
# ------------------=
def decode(lines):
    turns, spans, malformed = {}, [], 0
    widths = {"PHASES": 22, "LINKS": 6, "LLM": 14, "SPAN": 9}
    for line in lines:
        for kind, width in widths.items():
            marker = f"[VOICE {kind}] "
            if marker not in line:
                continue
            try:
                row = [int(value, 16) for value in line.split(marker, 1)[1].split()]
                if len(row) != width or row[0] != 1:
                    raise ValueError("unsupported record")
            except ValueError:
                malformed += 1
                break
            if kind == "SPAN":
                _, phase, job, part, start, end, units, status, valid = row
                spans.append(dict(phase=PHASES.get(phase, f"unknown_{phase}"), job=job,
                    part=part, start_ns=start, end_ns=end, units=units, status=status,
                    milliseconds=duration(start, end) if valid == 1 and start != 0 else None))
            else:
                turn = turns.setdefault(row[1], {"turn": row[1]})
                if kind == "PHASES":
                    flags = row[2]
                    times = {name: row[3 + i] if flags & (1 << i) else None
                             for i, name in enumerate(MILESTONES)}
                    turn["timestamps_ns"] = times
                    intervals = {
                        "capture_with_endpoint_tail": ("capture_started", "endpoint"),
                        "endpoint_to_recognition_observed": ("endpoint", "recognition_ready"),
                        "recognition_to_prompt_accepted": ("recognition_ready", "transcript"),
                        "routing_submission": ("transcript", "submitted"),
                        "submission_to_first_text": ("submitted", "first_text"),
                        "text_to_display": ("first_text", "first_display"),
                        "endpoint_to_display": ("endpoint", "first_display"),
                        "phrase_buffering": ("first_text", "speech_queued"),
                        "speech_queue_to_first_audio": ("speech_queued", "first_audio"),
                        "endpoint_to_first_audio": ("endpoint", "first_audio"),
                        "generation_elapsed": ("submitted", "generation_done"),
                        "playback_envelope": ("first_audio", "drained"),
                    }
                    turn["milliseconds"] = {name: duration(times[a], times[b])
                                             for name, (a, b) in intervals.items()}
                    turn["milliseconds"]["microphone_rearm"] = duration(times["drained"], row[14] if row[15] else None)
                    turn["owner_work"] = {
                        name: dict(calls=row[i], total_ms=row[i+1]/1e6, max_ms=row[i+2]/1e6)
                        for name, i in (("service", 16), ("presentation", 19))}
                elif kind == "LINKS":
                    turn.update(recognition_job=row[2], chat_turn=row[3], speech_job=row[4], state=row[5])
                else:
                    names = ["resident_model_load_ns", "first_token_ns", "first_visible_ns",
                             "response_elapsed_ns", "bsp_prefill_work_ns", "bsp_decode_work_ns",
                             "max_slice_ns", "max_pump_ns", "reused_tokens", "prefill_tokens", "output_tokens"]
                    turn["llm"] = dict(zip(names, row[3:]))
            break
    for turn in turns.values():
        turn["spans"] = [span for span in spans if span["job"] != 0 and (
            span["phase"].startswith("recognition_") and span["job"] == turn.get("recognition_job")
            or span["phase"] in ("speech_admission_wait", "speech_synthesis", "pcm_conversion", "observed_playback", "observed_playback_gap")
            and span["job"] == turn.get("speech_job"))]
        turn["spans"].extend(span for span in spans if span["phase"] in (
            "intent_routing", "personal_memory_update", "personal_memory_context", "prompt_build_and_tokenize")
            and span["job"] == turn["turn"])
    return dict(schema_version=1, turns=list(turns.values()), spans=spans, malformed_records=malformed,
                warning="Intervals overlap. Do not sum them. Playback timestamps are driver observations, not acoustic measurements.")

# ------------------------=
# FUNC: main
# DESC: Reads an existing serial log without touching the VM and prints a structured performance report.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("serial_log", type=Path)
    args = parser.parse_args()
    with args.serial_log.open(errors="replace") as source:
        print(json.dumps(decode(source), indent=2))

if __name__ == "__main__":
    main()
