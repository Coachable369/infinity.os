"""Derive the native busy cursor from the generated, transparent design-kit sheet."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Packs sixteen fixed-size authored frames without introducing runtime decoding.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    source = Image.open(kit / "wait-infinity-v1.png").convert("RGBA")
    assert source.width == source.height
    frames = []
    for index in range(16):
        column, row = index % 4, index // 4
        frame = source.crop((column * source.width // 4, row * source.height // 4,
                             (column + 1) * source.width // 4, (row + 1) * source.height // 4)).resize((64, 64), Image.Resampling.LANCZOS)
        assert frame.getextrema()[3][0] == 0
        assert frame.getextrema()[3][1] > 200
        frames.append(frame.tobytes())
    assert len(set(frames)) == 16
    (kit / "wait-infinity-v1.rgba").write_bytes(b"".join(frames))

if __name__ == "__main__":
    main()
