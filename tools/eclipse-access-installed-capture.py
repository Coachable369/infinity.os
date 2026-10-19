"""Capture real first-run and login pixels from a private ARM installation.

Run through build-kit with current QEMU-compatible ARM media. This reuses the
installed acceptance harness and its synthetic local account; it never bypasses
authentication, writes guest memory, or changes the user's VirtualBox machine.
Reusing a configured disk captures login only, not first-run configuration.
"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys
import time
import zlib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("eclipse_native_guest", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


# ------------------------=
# FUNC: png_chunk
# DESC: Encodes one checksummed PNG chunk without changing framebuffer colors.
# ------------------=
def png_chunk(kind, payload):
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))


# ------------------------=
# FUNC: framebuffer_png
# DESC: Losslessly converts an actual QMP RGB framebuffer capture to a portable PNG for visual review.
# ------------------=
def framebuffer_png(source, target):
    width, height, pixels = native.desktop.read_pixels(source)
    rows = b"".join(b"\0" + pixels[y * width * 3:(y + 1) * width * 3] for y in range(height))
    encoded = b"\x89PNG\r\n\x1a\n"
    encoded += png_chunk(b"IHDR", struct.pack(">2I5B", width, height, 8, 2, 0, 0, 0))
    encoded += png_chunk(b"IDAT", zlib.compress(rows))
    encoded += png_chunk(b"IEND", b"")
    target.write_bytes(encoded)
    return {"width": width, "height": height, "sha256": hashlib.sha256(encoded).hexdigest()}


class CaptureGuest(native.NativeGuest):
    # ------------------------=
    # FUNC: wait
    # DESC: Observes existing typed UI transitions and captures each configuration step and unfilled login focus state.
    # ------------------=
    def wait(self, predicate, label, timeout=120):
        state = super().wait(predicate, label, timeout)
        if state[3] != 1:
            return state
        if state[4] == 4 and state[7] <= 7:
            screen = f"configuration-{state[7]:02d}"
        elif state[4] in (9, 10):
            screen = "login-password-focus" if state[8] == 1 else "login"
        else:
            return state
        if screen in self.capture_report["screens"]:
            return state
        # Snapshot publication may precede the painter within the same loop.
        # No input is sent here; allow that frame to finish before QMP capture.
        time.sleep(.3)
        current = self.state()
        assert current is not None and current[3:5] == state[3:5] and current[7:9] == state[7:9]
        ppm = self.screenshot("eclipse-" + screen)
        png = self.capture_output / (screen + ".png")
        receipt = framebuffer_png(ppm, png)
        receipt.update({"path": str(png), "boot_route": current[3], "mode": current[4],
                        "configuration_step": current[7], "focus": current[8],
                        "guest_clock_ns": current[10], "snapshot_sequence": current[2]})
        self.capture_report["screens"][screen] = receipt
        if screen == "login-password-focus":
            # The idle animation must not restore wallpaper over the wordmark.
            # Observe actual RGB pixels across a complete ambient pulse period.
            width, height, pixels = native.desktop.read_pixels(ppm)
            scale = min(width * 1000 // 1536, height * 1000 // 1024)
            origin_x = (width - 1536 * scale // 1000) // 2
            origin_y = (height - 1024 * scale // 1000) // 2
            rect = (origin_x + 900 * scale // 1000, origin_y + 92 * scale // 1000,
                    570 * scale // 1000, 92 * scale // 1000)
            baseline = pixel_region(width, pixels, rect)
            samples = []
            for index in range(4):
                time.sleep(.85)
                frame = self.screenshot(f"eclipse-login-idle-{index}")
                next_width, next_height, next_pixels = native.desktop.read_pixels(frame)
                assert (next_width, next_height) == (width, height)
                assert pixel_region(width, next_pixels, rect) == baseline, "Idle animation altered branding pixels"
                path = self.capture_output / f"login-idle-{index}.png"
                samples.append(framebuffer_png(frame, path))
            receipt["idle_branding_stable"] = True
            receipt["idle_samples"] = samples
        (self.capture_output / "captures.json").write_text(json.dumps(self.capture_report, indent=2) + "\n")
        return state


# ------------------------=
# FUNC: pixel_region
# DESC: Extracts a bounded RGB rectangle for temporal repaint invariance checks.
# ------------------=
def pixel_region(width, pixels, rect):
    left, top, region_width, region_height = rect
    return b"".join(pixels[((top+y)*width+left)*3:((top+y)*width+left+region_width)*3]
                    for y in range(region_height))


# ------------------------=
# FUNC: main
# DESC: Runs the existing detached-install/account proof while retaining actual configuration and login framebuffer evidence.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iso", type=Path, required=True, help="Explicit current ARM QEMU test media, not the VirtualBox release ISO")
    parser.add_argument("--reuse-installed", action="store_true", help="Capture login only on this harness's existing private installation")
    parser.add_argument("--update-kernel", type=Path, help="Update only the reused private disk; not fresh-install parity evidence")
    parser.add_argument("--output", type=Path, default=ROOT / "builds/validation/eclipse-access" / str(time.time_ns()))
    parser.add_argument("work", type=Path, help="New harness work directory under build/, or the exact existing harness directory when reusing")
    args = parser.parse_args()
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        parser.error("Run through ./build-kit run")
    if args.update_kernel and not args.reuse_installed:
        parser.error("--update-kernel requires --reuse-installed")
    args.work.resolve().relative_to(ROOT / "build")
    args.work.resolve().parent.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve()
    output.relative_to(ROOT / "builds/validation")
    output.mkdir(parents=True, exist_ok=False)
    CaptureGuest.capture_output = output
    report = {"boundary": "actual detached installed ARM framebuffer", "screens": {},
              "first_run_expected": not args.reuse_installed, "visual_review_complete": False,
              "release_media_verified": False, "microphone_verified": False}
    CaptureGuest.capture_report = report
    native.NativeGuest = CaptureGuest
    sys.argv = [str(ROOT / "tools/native-editor-installed-test.py"), "--arch", "aarch64",
                "--iso", str(args.iso.resolve()), "--boot-only"]
    if args.reuse_installed:
        sys.argv.append("--reuse-installed")
    if args.update_kernel:
        sys.argv.extend(["--update-kernel", str(args.update_kernel.resolve())])
    sys.argv.append(str(args.work.resolve()))
    try:
        native.main()
        installation = json.loads((args.work.resolve() / "evidence.json").read_text())
        report["installed_boot_verified"] = installation["installed_boot_verified"]
        report["kernel_sha256"] = installation.get("updated_kernel_sha256", installation["packaged"]["installed-kernel.elf"]["sha256"])
        report["source_media"] = installation["source_media"]
        expected = {"login-password-focus"}
        if not args.reuse_installed:
            expected |= {f"configuration-{step:02d}" for step in range(8)}
        assert expected.issubset(report["screens"]), {"missing_screens": sorted(expected - report["screens"].keys())}
        report["capture_complete"] = True
    finally:
        (output / "captures.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
