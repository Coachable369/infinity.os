"""Verify both live and fresh-install kernels embed the canonical sound cues."""

from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


# ------------------------=
# FUNC: main
# DESC: Asserts exact cue payload presence in every supported live and installed kernel artifact.
# ------------------=
def main() -> None:
    cues = {
        name: (ROOT / f"assets/sounds/{name}.pcm").read_bytes()
        for name in ("boot", "login")
    }
    artifacts = (
        ROOT / "build/x86_64/kernel.elf",
        ROOT / "build/x86_64/installed-kernel.elf",
        ROOT / "build/aarch64/kernel.elf",
        ROOT / "build/aarch64/installed-kernel.elf",
    )
    for artifact in artifacts:
        payload = artifact.read_bytes()
        assert payload[:4] == b"\x7fELF"
        for cue in cues.values():
            assert payload.find(cue) >= 0
    print(
        {
            "artifacts": len(artifacts),
            "boot_pcm_bytes": len(cues["boot"]),
            "login_pcm_bytes": len(cues["login"]),
            "fresh_install_parity": True,
        }
    )


if __name__ == "__main__":
    main()
