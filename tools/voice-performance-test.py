import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("report", Path(__file__).with_name("voice-performance-report.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

# ------------------------=
# FUNC: wire
# DESC: Encodes a numeric protocol fixture rather than matching diagnostic prose.
# ------------------=
def wire(kind, values):
    return f"[VOICE {kind}] " + " ".join(format(value, "x") for value in values)

class PerformanceTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_partial_and_correlated_phases
    # DESC: Checks partial milestones, span correlation, work accounting and malformed record isolation.
    # ------------------=
    def test_partial_and_correlated_phases(self):
        phases = [1, 2, 1 | 2 | 4 | 8 | 512, 100, 200, 300, 400, 0, 0, 0, 0, 0, 450, 0, 0, 0, 2, 30, 20, 1, 10, 10]
        result = report.decode([wire("PHASES", phases), wire("LINKS", [1, 2, 101, 5, 500, 4]),
            wire("SPAN", [1, 2, 101, 0, 110, 190, 16000, 0, 1]),
            wire("SPAN", [1, 7, 500, 3, 800, 900, 0, 0, 1]),
            wire("SPAN", [1, 11, 2, 0, 220, 250, 0, 0, 1]),
            wire("SPAN", [1, 2, 999, 0, 110, 190, 16000, 0, 1]), "[VOICE PHASES] bad"])
        turn = result["turns"][0]
        self.assertEqual(turn["milliseconds"]["text_to_display"], 50/1e6)
        self.assertIsNone(turn["milliseconds"]["endpoint_to_first_audio"])
        self.assertEqual(len(turn["spans"]), 3)
        self.assertEqual(turn["owner_work"]["service"]["calls"], 2)
        self.assertEqual(result["malformed_records"], 1)

    # ------------------------=
    # FUNC: test_invalid_clock_and_failed_span
    # DESC: Preserves failed work and refuses clock reversals as negative or zero latency successes.
    # ------------------=
    def test_invalid_clock_and_failed_span(self):
        result = report.decode([wire("SPAN", [1, 4, 99, 1, 100, 90, 0, 7, 0])])
        self.assertIsNone(result["spans"][0]["milliseconds"])
        self.assertEqual(result["spans"][0]["status"], 7)
        self.assertEqual(result["turns"], [])

if __name__ == "__main__":
    unittest.main()
