"""Behavioral validation of the distributed standard-library audio example."""
import importlib.util
import pathlib
import struct
import tempfile
import wave


# ------------------------=
# FUNC: main
# DESC: Decodes generated WAV bytes and verifies bounded signal and invalid-input behavior.
# ------------------=
def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    spec = importlib.util.spec_from_file_location(
        "sdk_audio", root / "sdk/python/examples/audio_tone.py")
    example = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(example)
    with tempfile.TemporaryDirectory(dir=root / "build/tests") as directory:
        path = pathlib.Path(directory) / "tone.wav"
        assert example.write_tone(path) == 8000
        with wave.open(str(path), "rb") as source:
            assert (source.getnchannels(), source.getsampwidth(),
                    source.getframerate(), source.getnframes()) == (1, 2, 16000, 8000)
            samples = struct.unpack("<8000h", source.readframes(8000))
        assert samples[0] == samples[-1] == 0
        assert 6000 <= max(samples) <= 6553
        assert -6553 <= min(samples) <= -6000
        crossings = sum(a <= 0 < b for a, b in zip(samples[200:-200], samples[201:-199]))
        assert 208 <= crossings <= 210
        invalid = pathlib.Path(directory) / "invalid.wav"
        for options in ({"rate": 1}, {"frequency": 8000}, {"frequency": -1},
                        {"duration": 11}, {"duration": 0}, {"duration": 0.00001}):
            try:
                example.write_tone(invalid, **options)
            except ValueError:
                pass
            else:
                raise AssertionError(options)
            assert not invalid.exists()
    print("SDK audio example: decoded PCM metadata, signal, envelope and rejection checks passed")


if __name__ == "__main__":
    main()
