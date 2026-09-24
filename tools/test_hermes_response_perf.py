"""Behavioral validation of native binary response timing receipts."""
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("hermes_perf", Path(__file__).with_name("hermes-response-perf.py"))
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)


class TimingReceiptTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_single_token_has_no_decode_rate
    # DESC: Keeps zero-length decode intervals distinct from measured speedups.
    # ------------------=
    def test_single_token_has_no_decode_rate(self):
        self.assertIsNone(PERF.speed_ratio(0, 0))
        self.assertEqual(PERF.speed_ratio(30, 20), 1.5)

    # ------------------------=
    # FUNC: test_binary_metrics_and_output_boundaries
    # DESC: Verifies numeric timing, content bytes and malformed receipt rejection without any console-text oracle.
    # ------------------=
    def test_binary_metrics_and_output_boundaries(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.bin"
            content = bytes([1, 2, 255, 0])
            valid = struct.pack("<6Q", 1, 2, 30, 4, 10, 20) + content
            path.write_bytes(valid)
            metrics, result = PERF.read_result(path)
            self.assertEqual(result, content)
            self.assertEqual((metrics["tokens"], metrics["first_ns"], metrics["decode_ns"], metrics["total_ns"]), (2, 10, 10, 30))
            for invalid in (valid[:-1], valid+b"\x00", struct.pack("<6Q", 1, 2, 30, 4, 20, 10)+content,
                            struct.pack("<6Q", 1, 2, 15, 4, 10, 20)+content):
                path.write_bytes(invalid)
                with self.assertRaises(AssertionError):
                    PERF.read_result(path)
