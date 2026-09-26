"""Behavioral verification for canonical MP3 cues and their runtime PCM."""

from array import array
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent


# ------------------------=
# FUNC: decoded_samples
# DESC: Decodes a canonical MP3 through the production conversion command and returns signed PCM samples.
# ------------------=
def decoded_samples(source: Path) -> array:
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "cue.pcm"
        subprocess.run(
            [str(ROOT / "tools/build-system-sound.sh"), str(source), str(output)],
            check=True,
        )
        samples = array("h")
        samples.frombytes(output.read_bytes())
        return samples


# ------------------------=
# FUNC: stored_samples
# DESC: Loads the immutable runtime form while enforcing complete 16-bit sample framing.
# ------------------=
def stored_samples(path: Path) -> array:
    payload = path.read_bytes()
    assert payload and len(payload) % 2 == 0
    samples = array("h")
    samples.frombytes(payload)
    return samples


# ------------------------=
# FUNC: main
# DESC: Proves each runtime cue is the deterministic, audible conversion of its named MP3 source.
# ------------------=
def main() -> None:
    observed = {}
    for name in ("boot", "login"):
        source = ROOT / f"assets/sounds/{name}.mp3"
        runtime = ROOT / f"assets/sounds/{name}.pcm"
        decoded = decoded_samples(source)
        stored = stored_samples(runtime)
        assert stored == decoded
        assert 16_000 < len(stored) <= 16_000 * 32
        peak = max(abs(sample) for sample in stored)
        energy = sum(sample * sample for sample in stored) // len(stored)
        assert peak > 1_000 and energy > 10_000
        observed[name] = {
            "frames": len(stored),
            "duration_ms": len(stored) * 1_000 // 16_000,
            "peak": peak,
        }
    assert stored_samples(ROOT / "assets/sounds/boot.pcm") != stored_samples(
        ROOT / "assets/sounds/login.pcm"
    )
    assert observed["login"]["duration_ms"] > 2_200
    print(observed)


if __name__ == "__main__":
    main()
