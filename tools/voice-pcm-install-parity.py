"""Assert fixed microphone filters are embedded in live and installed kernels."""
from pathlib import Path
import sys


# ------------------------=
# FUNC: main
# DESC: Checks exact canonical binary filter payloads in each requested live and installed ELF pair.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    targets = sys.argv[1:] or ["aarch64", "x86_64"]
    filters = [(root / f"kernel/runtime/ai/voice-pcm-{rate}.bin").read_bytes()
               for rate in (44100, 48000)]
    for target in targets:
        assert target in ("aarch64", "x86_64")
        for name in ("kernel.elf", "installed-kernel.elf"):
            artifact = root / "build" / target / name
            payload = artifact.read_bytes()
            assert payload[:4] == b"\x7fELF", artifact
            for coefficients in filters:
                assert len(coefficients) == 96 * 160 * 4
                assert coefficients in payload, artifact
    print({"targets": targets, "filters_per_kernel": len(filters), "installed_parity": True})


if __name__ == "__main__":
    main()
