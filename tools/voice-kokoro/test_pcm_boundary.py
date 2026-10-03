"""Actual native PCM edge behavior; not installed audio or hardware acceptance."""
from pathlib import Path
import array
import importlib.util
import json
import os
import subprocess
import unittest
import wave

ROOT = Path(__file__).resolve().parents[2]
COMPARISON_SPEC = importlib.util.spec_from_file_location("pcm_comparison", ROOT / "tools/voice-kokoro/compare-reference.py")
COMPARISON = importlib.util.module_from_spec(COMPARISON_SPEC)
COMPARISON_SPEC.loader.exec_module(COMPARISON)


class NativePcmBoundaryTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUpClass
    # DESC: Builds the production edge helper in a sanitizer-backed executable inside the repository build kit.
    # ------------------=
    @classmethod
    def setUpClass(cls):
        if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
            raise RuntimeError("Run through build-kit")
        cls.output = ROOT / "build/voice-kokoro/pcm-boundary"
        cls.output.mkdir(parents=True, exist_ok=True)
        cls.binary = cls.output / "test"
        subprocess.run(["clang++", "-std=c++17", "-O2", "-fsanitize=address,undefined",
                        str(ROOT / "tools/voice-kokoro/pcm-boundary-test.cpp"),
                        "-o", str(cls.binary)], check=True)

    # ------------------------=
    # FUNC: test_exact_samples_and_bounds
    # DESC: Executes boundary and quiet-consonant fixtures with sanitizers, including unchanged internal pauses.
    # ------------------=
    def test_exact_samples_and_bounds(self):
        subprocess.run([str(self.binary)], check=True)

    # ------------------------=
    # FUNC: test_comparator_accepts_only_zero_edge_padding
    # DESC: Allows different exact-silent guards while comparing every quiet speech sample and the full internal pause.
    # ------------------=
    def test_comparator_accepts_only_zero_edge_padding(self):
        speech = [1, -1, 1000, -1000, 0, 0, 0, 1, -1]
        evidence = COMPARISON.compare_pcm([0] * 5000 + speech + [0] * 4000,
                                          [0] * 480 + speech + [0] * 480)
        self.assertEqual(evidence["active_frames"], len(speech))
        self.assertEqual((evidence["correlation"], evidence["relative_rms_error"]), (1.0, 0.0))

    # ------------------------=
    # FUNC: test_comparator_rejects_truncation_and_pause_changes
    # DESC: Rejects missing one-LSB edge consonants, interior silence deletion, distortion and a completely silent result.
    # ------------------=
    def test_comparator_rejects_truncation_and_pause_changes(self):
        speech = [1, -1, 1000, -1000, 0, 0, 0, 1, -1]
        for broken in (speech[1:], speech[:-1], speech[:4] + speech[7:], [-v for v in speech], [0] * 9):
            with self.assertRaises(AssertionError):
                COMPARISON.compare_pcm(speech, broken)

    # ------------------------=
    # FUNC: test_existing_native_artifacts
    # DESC: Applies the real helper to available guest PCM and proves every nonzero sample and its relative timing survives.
    # ------------------=
    def test_existing_native_artifacts(self):
        inputs = sorted((ROOT / "build/voice-kokoro/aarch64").glob("native-*.wav"))
        if not inputs:
            self.skipTest("No previous native guest PCM artifacts available")
        evidence = []
        for path in inputs:
            with wave.open(str(path), "rb") as wav:
                self.assertEqual((wav.getnchannels(), wav.getsampwidth(), wav.getframerate()), (1, 2, 24000))
                source = array.array("h", wav.readframes(wav.getnframes()))
            raw = self.output / (path.stem + ".pcm")
            trimmed = self.output / (path.stem + "-trimmed.pcm")
            raw.write_bytes(source.tobytes())
            subprocess.run([str(self.binary), str(raw), str(trimmed)], check=True)
            actual = array.array("h", trimmed.read_bytes())
            positions = [i for i, value in enumerate(source) if value]
            if positions:
                start = max(0, positions[0] - 480)
                end = min(len(source), positions[-1] + 481)
                self.assertEqual(actual, source[start:end])
            else:
                self.assertEqual(actual, source)
            if positions:
                comparison = COMPARISON.compare_pcm(source, actual)
                self.assertEqual((comparison["correlation"], comparison["relative_rms_error"]), (1.0, 0.0))
            evidence.append(dict(source=str(path.relative_to(ROOT)), original_frames=len(source),
                                 trimmed_frames=len(actual), removed_seconds=(len(source) - len(actual)) / 24000,
                                 exact_nonzero_and_internal_pause_preservation=True))
        (self.output / "evidence.json").write_text(json.dumps(dict(installed_verified=False, cases=evidence), indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
