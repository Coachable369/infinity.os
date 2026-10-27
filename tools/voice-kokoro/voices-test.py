#!/usr/bin/env python3
"""Verify each installed voice produces distinct non-silent PCM using the pinned model."""
import json
import subprocess
from pathlib import Path
from reference import inspect_pcm


# ------------------------=
# FUNC: main
# DESC: Exercises all five model voices and records bounded real PCM evidence, not a native-VM acceptance claim.
# ------------------=
def main():
    root = Path(__file__).resolve().parents[2]
    output = root / "build/voice-kokoro/voices"
    output.mkdir(parents=True, exist_ok=True)
    evidence = {}
    for voice in ("af_heart", "af_bella", "af_nicole", "am_adam", "am_michael"):
        wav = output / (voice + ".wav")
        with (output / (voice + ".log")).open("w") as log:
            subprocess.run([str(root / "build/voice-kokoro/reference/kokopop_say"),
                "--model", str(root / "model-cache/kokoro-v1_0.gguf"), "--backend", "cpu",
                "--threads", "1", "--voice", voice, "--speed", "1.15",
                "--text", "Hello. This is your Infinity voice.", "--out", str(wav)],
                cwd=root, stdout=log, stderr=log, check=True)
        evidence[voice] = inspect_pcm(wav)
    assert len({v["pcm_sha256"] for v in evidence.values()}) == 5
    (output / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print("Five distinct offline voices synthesized successfully (host reference).")


if __name__ == "__main__":
    main()
