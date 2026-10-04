"""Behavioral tests for native latency evidence validation."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("latency", Path(__file__).parent / "probe/compare-latency.py")
latency = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(latency)
RUNNER_SPEC = importlib.util.spec_from_file_location("runner", Path(__file__).parent / "probe/latency.py")
runner = importlib.util.module_from_spec(RUNNER_SPEC)
RUNNER_SPEC.loader.exec_module(runner)


# ------------------------=
# FUNC: fixture
# DESC: Creates bounded binary probe fixtures with deterministic complete PCM and native timer results.
# ------------------=
def fixture(ticks=100, profiled=False, changed_case=None, heap=1000):
    result = bytearray()
    for index, status in enumerate(latency.STATUSES):
        frames = 2400 if status == 0 else 0
        result.extend(struct.pack("<18Q", 2, index, status, frames, ticks, 1000, 4, heap, 0, 0, *([0] * 8)))
        result.extend(struct.pack("<h", 2 if index == changed_case else 1) * frames)
    result.extend(struct.pack("<Q", 4) + b"test")
    result.extend(struct.pack("<256Q", int(profiled), *([0] * 255)))
    result.extend(struct.pack("<3Q", 8448, 165, 60000))
    return bytes(result)


# ------------------------=
# FUNC: lifetime_fixture
# DESC: Constructs complete numeric lifetime transitions for parser validation without native log-text or source oracles.
# ------------------=
def lifetime_fixture():
    transitions = [(0, 10, 0)]
    for turn in range(2):
        transitions.append((turn, 11, 0))
        for index in range(6):
            transitions.extend(((turn * 6 + index, 0, 0), (turn * 6 + index, 1, 0)))
    transitions.extend(((12, 12, 7), (13, 13, 7)))
    result = bytearray()
    for case, kind, status in transitions:
        frames, ticks, pcm_hash = 0, 0, 0
        if kind == 10:
            frames = 1_000_000_000
        elif kind == 11:
            frames = 106
        elif kind == 1:
            index = case % 6
            frames = [18273, 43486, 90000, 30000, 95000, 40000][index]
            pcm_hash = [0x5838af8f1e7ca150, 0xbefdcb6522e7d813, 102, 103, 104, 105][index]
            ticks = 1_000_000
        diagnostics = [3, 1_000_000_000, 0, 0] + [0] * 8
        memory = [1_000_000_000, 800_000_000, 200_000_000, 1_000_000, 16_777_216, 2_000_000, 0]
        if status == 7:
            diagnostics[3] = 0x1234
            memory[-1] = 0xffffffff14000400
        result.extend(struct.pack("<27Q", 0x494e464c49464531, case, kind, status,
                                  frames, ticks, 1_000_000, *diagnostics, pcm_hash, *memory))
    return result


class LatencyComparisonTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_baseline_artifact_boundary
    # DESC: Accepts only an existing build artifact and rejects absent, directory, outside, and escaping-symlink inputs.
    # ------------------=
    def test_baseline_artifact_boundary(self):
        with tempfile.TemporaryDirectory(dir=runner.ROOT / "build", prefix="latency-test-") as directory:
            path = Path(directory)
            artifact = path / "native.o"
            artifact.write_bytes(bytes(1))
            escaped = path / "outside.o"
            escaped.symlink_to(Path(__file__).resolve())
            self.assertEqual(runner.baseline_object(artifact), artifact.resolve())
            for candidate in (path, path / "absent.o", Path(__file__), escaped):
                with self.subTest(path=candidate), self.assertRaises(ValueError):
                    runner.baseline_object(candidate)

    # ------------------------=
    # FUNC: test_complete_mixed_engine_sequence
    # DESC: Accepts the exact two-recognition twelve-synthesis lifetime and fatal quarantine transition chain.
    # ------------------=
    def test_complete_mixed_engine_sequence(self):
        rows = runner.decode_lifetime(lifetime_fixture())
        self.assertEqual(len(rows), 29)
        self.assertEqual(sum(row["kind"] == 11 for row in rows), 2)
        self.assertEqual(sum(row["kind"] == 1 for row in rows), 12)

    # ------------------------=
    # FUNC: test_rejects_incomplete_and_reordered_results
    # DESC: Rejects a truncated chain, extra fault bytes, and a changed operation identity.
    # ------------------=
    def test_rejects_incomplete_and_reordered_results(self):
        reordered = lifetime_fixture()
        struct.pack_into("<Q", reordered, 216 + 16, 10)
        for data in (lifetime_fixture()[:-1], lifetime_fixture() + bytes(8), reordered):
            with self.subTest(size=len(data)), self.assertRaises(ValueError):
                runner.decode_lifetime(data)

    # ------------------------=
    # FUNC: test_rejects_pcm_heap_and_privacy_failures
    # DESC: Rejects changed repeated samples, silent recognition, malformed heap accounting, and nonzero private PCM after fatal failure.
    # ------------------=
    def test_rejects_pcm_heap_and_privacy_failures(self):
        for row, field, value in ((16, 19, 17), (1, 4, 0), (3, 21, 800_000_001), (27, 19, 1), (28, 20, 999_999_999)):
            data = lifetime_fixture()
            struct.pack_into("<Q", data, row * 216 + field * 8, value)
            with self.subTest(row=row, field=field), self.assertRaises(ValueError):
                runner.decode_lifetime(data)

    # ------------------------=
    # FUNC: test_profile_requires_explicit_diagnostic_mode
    # DESC: Allows diagnostic parsing without admitting instrumented timings into performance comparisons.
    # ------------------=
    def test_profile_requires_explicit_diagnostic_mode(self):
        self.assertEqual(len(latency.decode(fixture(profiled=True), allow_profile=True)[0]), 10)
        with self.assertRaises(ValueError):
            latency.compare(fixture(profiled=True), fixture(profiled=True))

    # ------------------------=
    # FUNC: test_exact_reply_speedup
    # DESC: Accepts identical speech with improved timers and reports the measured factor.
    # ------------------=
    def test_exact_reply_speedup(self):
        result = latency.compare(fixture(), fixture(ticks=50))
        self.assertTrue(result["complete_pcm_identical"])
        self.assertEqual([row["speedup"] for row in result["fixtures"]], [2, 2, 2])
        self.assertEqual([row["realtime_factor"] for row in result["fixtures"]], [0.5, 0.5, 0.5])

    # ------------------------=
    # FUNC: test_rejects_invalid_measurements
    # DESC: Rejects incomplete, instrumented, altered speech, growing memory, and failed numerical evidence.
    # ------------------=
    def test_rejects_invalid_measurements(self):
        damaged = bytearray(fixture())
        struct.pack_into("<Q", damaged, len(damaged) - 24, 0)
        for candidate in (fixture()[:-1], fixture(profiled=True), fixture(changed_case=6),
                          fixture(heap=1001), bytes(damaged), b""):
            with self.subTest(length=len(candidate)), self.assertRaises(ValueError):
                latency.compare(fixture(), candidate)


if __name__ == "__main__":
    unittest.main()
