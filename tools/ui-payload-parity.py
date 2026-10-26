"""Exact compiled UI payload verification shared by full and incremental media builds."""
import argparse
import mmap
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PAYLOAD_KINDS = {
    "login-screens.iuit": "login",
    "boot-screens.iuit": "boot",
    "installer-screens.iuit": "template",
    "configuration-screens.iuit": "template",
    "settings-screens.iuit": "settings",
    "uikit-tokens.bin": "tokens",
    "uikit-styles.bin": "styles",
}


# ------------------------=
# FUNC: verify_embedded_ui
# DESC: Uses the native decoders before requiring exact current IUIT, palette and style bytes in a consuming kernel.
# ------------------=
def verify_embedded_ui(kernel, payloads, checkpoint=None, *, snapshot=None):
    captured = {path: path.read_bytes() if snapshot is None else snapshot[path] for path in payloads}
    with kernel.open("rb") as source, mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as binary:
        for path, payload in captured.items():
            if checkpoint is not None:
                checkpoint()
            kind = PAYLOAD_KINDS.get(path.name)
            if kind is None:
                raise RuntimeError(f"Unrecognized UI payload role: {path}")
            # Validate the same immutable bytes searched below, even if Studio
            # saves the original file while this subprocess is running.
            result = subprocess.run([str(ROOT / "build/behavior-tests/ui-payload-validator"), kind, "-"],
                                    input=payload, stdout=subprocess.DEVNULL)
            if result.returncode != 0 or binary.find(payload) < 0:
                raise RuntimeError(f"Kernel does not contain current UI data: {path}")
    for path, payload in captured.items():
        if path.read_bytes() != payload:
            raise RuntimeError(f"UI data changed during native validation: {path}")


# ------------------------=
# FUNC: verify_kernel_pair
# DESC: Verifies every screen and theme payload consumed by live and installed kernels, excluding installer-only screens from installed boot.
# ------------------=
def verify_kernel_pair(live, installed, root=ROOT):
    screens = [root / f"assets/boot/{name}-screens.iuit" for name in ("installer", "configuration", "settings", "boot", "login")]
    styles = [root / f"assets/ui-design-kit/default/uikit-{name}.bin" for name in ("tokens", "styles")]
    snapshot = {path: path.read_bytes() for path in [*screens, *styles]}
    verify_embedded_ui(live, [*screens, *styles], snapshot=snapshot)
    verify_embedded_ui(installed, [*screens[1:], *styles], snapshot=snapshot)
    for path, payload in snapshot.items():
        if path.read_bytes() != payload:
            raise RuntimeError(f"UI data changed during kernel-pair validation: {path}")


# ------------------------=
# FUNC: main
# DESC: Exposes the shared byte-level gate only through repository build authority.
# ------------------=
def main():
    if not os.environ.get("INFINITY_BUILD_KIT_ACTIVE"):
        raise RuntimeError("Use ./build-kit run")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("live", type=Path)
    parser.add_argument("installed", type=Path)
    args = parser.parse_args()
    subprocess.run(["make", "build/behavior-tests/ui-payload-validator"], cwd=ROOT, check=True)
    verify_kernel_pair(args.live, args.installed)
    print("Live and installed screen, palette and surface-style bytes verified")


if __name__ == "__main__":
    main()
