"""Pack the generated neutral window-control state plates for native rendering."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Packs authored normal, hover and pressed plates with alpha into fixed runtime cells.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    source = Image.open(kit / "window-controls-v1.png").convert("RGBA")
    cells = []
    for index in range(3):
        cell = source.crop((index * source.width // 3, 0,
                            (index + 1) * source.width // 3, source.height))
        bounds = cell.getchannel("A").point(lambda a: 255 if a >= 128 else 0).getbbox()
        assert bounds is not None
        cell = cell.crop(bounds).resize((96, 96), Image.Resampling.LANCZOS)
        assert cell.getextrema()[3][0] == 0
        assert cell.getpixel((48, 48))[3] > 230
        cells.append(cell.tobytes())
    (kit / "window-controls-v1.rgba").write_bytes(b"".join(cells))

if __name__ == "__main__":
    main()
