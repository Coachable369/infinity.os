"""Pack the approved Eclipse Access artwork into native image and glyph formats."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Converts the authored monochrome glyph mask and wallpaper without redrawing their shapes or composition.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    Image.open(kit / "eclipse-access-wallpaper-v1.png").convert("RGB").save(kit / "eclipse-access-wallpaper-v1.bmp")
    source = Image.open(kit / "eclipse-access-glyphs-v1.png").convert("L")
    packed = bytearray()
    preview = Image.new("RGBA", (384, 384), (9, 15, 38, 255))
    for index in range(16):
        x, y = index % 4, index // 4
        mask = source.crop((x*source.width//4, y*source.height//4,
                            (x+1)*source.width//4, (y+1)*source.height//4))
        bounds = mask.point(lambda a: 255 if a >= 128 else 0).getbbox()
        assert bounds is not None
        mask = mask.crop(bounds)
        mask.thumbnail((52, 52), Image.Resampling.LANCZOS)
        art = Image.new("RGBA", mask.size, (241, 246, 255, 255))
        art.putalpha(mask)
        cell = Image.new("RGBA", (64, 64))
        cell.alpha_composite(art, ((64-art.width)//2, (64-art.height)//2))
        packed.extend(cell.tobytes())
        preview.alpha_composite(cell, (x*96+16, y*96+16))
    (kit / "eclipse-access-glyphs-v1.rgba").write_bytes(packed)
    preview.save(kit / "eclipse-access-glyphs-v1-preview.png")

if __name__ == "__main__":
    main()
