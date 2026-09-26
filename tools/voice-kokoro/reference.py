#!/usr/bin/env python3
"""Build a pinned CPU Kokoro reference. Never used by the installed OS."""
import argparse
import array
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import urllib.request
import wave

ROOT = Path(__file__).resolve().parents[2]
REVISION = "2c6989ac800f624ea984210215e1b76be42eea81"
MODEL_REVISION = "b7958cdedd0efa56e8756895e739fbe2852f635d"
MODEL_SHA256 = "e8242a1321e580599c26981692adf088ebfdc4c7b7555959b13f4d6667d0d206"


# ------------------------=
# FUNC: run
# DESC: Executes a reference-only build or synthesis command and rejects unsuccessful exits.
# ------------------=
def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


# ------------------------=
# FUNC: checksum
# DESC: Hashes the complete model without keeping a second model-sized byte array.
# ------------------=
def checksum(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


# ------------------------=
# FUNC: inspect_pcm
# DESC: Checks actual bounded 24-kHz mono PCM; does not claim intelligibility from nonzero samples.
# ------------------=
def inspect_pcm(path):
    with wave.open(str(path), "rb") as stream:
        if (stream.getnchannels(), stream.getsampwidth(), stream.getframerate()) != (1, 2, 24000):
            raise ValueError("Kokoro reference must be signed 16-bit mono PCM at 24 kHz")
        frames = stream.getnframes()
        if not 2400 <= frames <= 24000 * 30:
            raise ValueError("Reference duration is outside the bounded speech contract")
        pcm = array.array("h", stream.readframes(frames))
    if sys.byteorder != "little":
        pcm.byteswap()
    if len(pcm) != frames or not any(pcm):
        raise ValueError("Reference contains truncated or silent PCM")
    return {"sample_rate": 24000, "frames": frames, "seconds": frames / 24000,
            "peak": max(abs(value) for value in pcm),
            "clipped_samples": sum(abs(value) >= 32767 for value in pcm),
            "pcm_sha256": hashlib.sha256(pcm.tobytes()).hexdigest()}


# ------------------------=
# FUNC: main
# DESC: Produces reproducible real Kokoro reference audio while explicitly excluding native/install acceptance claims.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--text", default="Hi. I am the voice of Infinity OS. How can I help you today?")
    args = parser.parse_args()
    if not args.text.strip() or len(args.text) > 160:
        parser.error("Reference text must contain 1 to 160 characters")
    source = ROOT / "build/kokopop-port-audit"
    output = ROOT / "build/voice-kokoro/reference"
    output.mkdir(parents=True, exist_ok=True)
    if not source.exists():
        run("git", "clone", "https://github.com/tterrasson/kokopop.git", source)
        run("git", "-C", source, "checkout", "--detach", REVISION)
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    if revision != REVISION or subprocess.check_output(
            ["git", "-C", str(source), "status", "--porcelain", "--untracked-files=no"]):
        raise RuntimeError("Unreviewed Kokoro source; refusing reference build")
    model = ROOT / "model-cache/kokoro-v1_0.gguf"
    model.parent.mkdir(exist_ok=True)
    if not model.exists():
        temporary = model.with_suffix(".gguf.partial")
        url = f"https://huggingface.co/tterrasson/Kokoro-GGUF/resolve/{MODEL_REVISION}/kokoro-v1_0.gguf"
        with urllib.request.urlopen(url, timeout=90) as response, temporary.open("wb") as stream:
            while chunk := response.read(1024 * 1024):
                stream.write(chunk)
        if checksum(temporary) != MODEL_SHA256:
            raise RuntimeError("Downloaded Kokoro model checksum mismatch")
        temporary.replace(model)
    if checksum(model) != MODEL_SHA256:
        raise RuntimeError("Kokoro model checksum mismatch")
    run("cmake", "-S", source, "-B", output, "-DCMAKE_BUILD_TYPE=Release",
        "-DKOKOPOP_ENABLE_METAL=OFF", "-DKOKOPOP_BUILD_TESTS=OFF",
        "-DKOKOPOP_ENABLE_OPUS=OFF", "-DKOKOPOP_BUILD_TOOLS=ON")
    run("cmake", "--build", output, "--target", "kokopop_say", "-j4")
    audio = output / "hello.wav"
    audio.unlink(missing_ok=True)
    run(output / "kokopop_say", "--model", model, "--backend", "cpu", "--threads", "4",
        "--voice", "af_heart", "--text", args.text, "--out", audio)
    evidence = {"environment": "host CPU reference only", "source_revision": REVISION,
                "model_sha256": MODEL_SHA256, "voice": "af_heart", "pcm": inspect_pcm(audio),
                "native_verified": False, "installed_verified": False,
                "perceptual_quality_verified": False, "recognition_verified": False}
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
