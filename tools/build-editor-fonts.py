"""Rasterize the shipped OFL mono families at each native editor size."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
ROOT = Path(__file__).resolve().parent.parent
FONTS = ["JetBrainsMono-Variable.ttf", "FiraCode-Variable.ttf", "IBMPlexMono-Regular.ttf", "Inconsolata.ttf"]
SIZES = [8, 10, 12, 14, 16, 18, 20, 22, 24]

# ------------------------=
# FUNC: main
# DESC: Generates fixed records with native size metrics and supersampled glyph pixels.
# ------------------=
def main():
    result = bytearray()
    for name in FONTS:
        for size in SIZES:
            font = ImageFont.truetype(str(ROOT / "assets/fonts" / name), size * 4)
            try:
                if b"Regular" in font.get_variation_names(): font.set_variation_by_name(b"Regular")
            except (OSError, AttributeError):
                pass
            bounds = [font.getbbox(chr(c), anchor="ls") for c in range(32,127)]
            top = min(b[1] for b in bounds)
            bottom = max(b[3] for b in bounds)
            # A monospace advance is not the widest glyph's ink bounding box.
            # Retain overhang pixels separately without adding tracking to every cell.
            cell = max(1, round(font.getlength("M") / 4))
            origin = max(0, -min(b[0] for b in bounds))
            ink_width = (max(b[2] for b in bounds) + origin + 3) // 4
            height = (bottom-top+3)//4 + 2
            if cell > 24 or ink_width > 24 or height > 32: raise ValueError(name)
            canvas = Image.new("L", (24*95*4,32*4))
            draw = ImageDraw.Draw(canvas)
            for i,b in enumerate(bounds):
                draw.text((i*24*4+origin,4-top), chr(i+32), font=font,anchor="ls",fill=255)
            result.extend(bytes([cell,height,size,ink_width]))
            result.extend(canvas.resize((24*95,32),Image.Resampling.LANCZOS).tobytes())
    (ROOT / "assets/fonts/InfinityEditor-library.bin").write_bytes(result)

if __name__ == "__main__": main()
