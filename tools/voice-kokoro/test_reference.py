"""Behavioral checks for the reference WAV validator, not native voice acceptance."""
import array
from pathlib import Path
import tempfile
import sys
import unittest
import wave

from reference import inspect_pcm


class ReferencePcmTests(unittest.TestCase):
    # ------------------------=
    # FUNC: fixture
    # DESC: Writes a controlled PCM container to exercise format and signal validation.
    # ------------------=
    def fixture(self, path, rate=24000, channels=1, frames=4800, value=1000):
        with wave.open(str(path), "wb") as stream:
            stream.setparams((channels, 2, rate, frames, "NONE", "not compressed"))
            samples = array.array("h", [value, -value] * (frames * channels // 2))
            if sys.byteorder != "little":
                samples.byteswap()
            stream.writeframes(samples.tobytes())

    # ------------------------=
    # FUNC: test_pcm_properties
    # DESC: Asserts structured values derived from the complete waveform.
    # ------------------=
    def test_pcm_properties(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audio.wav"
            self.fixture(path)
            result = inspect_pcm(path)
            self.assertEqual(result["frames"], 4800)
            self.assertEqual(result["seconds"], 0.2)
            self.assertEqual(result["peak"], 1000)
            self.assertEqual(result["clipped_samples"], 0)

    # ------------------------=
    # FUNC: test_rejects_wrong_rate_channels_silence_and_bounds
    # DESC: Rejects Flite-rate PCM, stereo, silence, and invalid duration instead of calling them Kokoro proof.
    # ------------------=
    def test_rejects_wrong_rate_channels_silence_and_bounds(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "audio.wav"
            for configuration in ({"rate": 16000}, {"channels": 2}, {"value": 0}, {"frames": 100}):
                self.fixture(path, **configuration)
                with self.assertRaises(ValueError):
                    inspect_pcm(path)


if __name__ == "__main__":
    unittest.main()
