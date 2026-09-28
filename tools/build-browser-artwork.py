"""Losslessly encode approved browser PNG artwork for the native BGRA painter."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Converts existing approved artwork without resampling, recoloring or changing alpha.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[1] / "assets/apps"
    for name in ("infinity-browser-icon-v1", "infinity-browser-navigation-v1", "infinity-world-shift-icon-v1", "infinity-holographic-desktop-icon-v1"):
        with Image.open(root / (name + "-source.png")) as image:
            rgba = image.convert("RGBA")
            output = root / (name + ".bmp")
            rgba.save(output, format="BMP")
            # Pillow's BMP reader ignores BI_RGB alpha; verify actual BGRA rows.
            data = output.read_bytes()
            offset = int.from_bytes(data[10:14], "little")
            width, height = rgba.size
            assert int.from_bytes(data[28:30], "little") == 32
            raw = rgba.tobytes("raw", "BGRA")
            for y in range(height):
                start = offset + (height - 1 - y) * width * 4
                assert data[start:start + width * 4] == raw[y * width * 4:(y + 1) * width * 4]
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
