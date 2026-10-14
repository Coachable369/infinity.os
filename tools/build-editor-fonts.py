"""Rasterize the shipped OFL mono families at each native editor size."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
ROOT = Path(__file__).resolve().parent.parent
FONTS = ["JetBrainsMono-Variable.ttf", "FiraCode-Variable.ttf", "IBMPlexMono-Regular.ttf", "Inconsolata.ttf"]
SIZES = [12, 14, 16, 18, 20, 24]

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
            cell = max(round(font.getlength("M") / 4), max((b[2]-b[0]+3)//4 for b in bounds)) + 1
            height = (bottom-top+3)//4 + 2
            if cell > 24 or height > 32: raise ValueError(name)
            canvas = Image.new("L", (24*95*4,32*4))
            draw = ImageDraw.Draw(canvas)
            for i,b in enumerate(bounds):
                draw.text((i*24*4+max(0,-b[0]),4-top), chr(i+32), font=font,anchor="ls",fill=255)
            result.extend(bytes([cell,height,size,0]))
            result.extend(canvas.resize((24*95,32),Image.Resampling.LANCZOS).tobytes())
    (ROOT / "assets/fonts/InfinityEditor-library.bin").write_bytes(result)

if __name__ == "__main__": main()
