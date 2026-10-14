#!/usr/bin/env python3
"""Produce bounded native UI faces from the shipped OFL font library."""
from pathlib import Path
import re
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONTS = ROOT / "assets/fonts"
WIDTH, HEIGHT, COUNT, SUPER = 24, 28, 95, 4

# ------------------------=
# FUNC: face
# DESC: Rasterizes one real font weight, fitting all ASCII ink inside each shared native cell.
# ------------------=
def face(path, bold):
    for size in range(24, 9, -1):
        font = ImageFont.truetype(str(path), size * SUPER)
        try:
            names = font.get_variation_names()
            preferred = [b"SemiBold", b"Semi Bold", b"Medium", b"Bold"] if bold else [b"Regular", b"Normal"]
            for name in preferred:
                if name in names:
                    font.set_variation_by_name(name)
                    break
        except (OSError, AttributeError):
            pass
        bounds = [font.getbbox(chr(c), anchor="ls") for c in range(32, 127)]
        top, bottom = min(b[1] for b in bounds), max(b[3] for b in bounds)
        if max(b[2]-b[0] for b in bounds) <= (WIDTH-2)*SUPER and bottom-top <= (HEIGHT-2)*SUPER:
            break
    else:
        raise ValueError(f"Font cannot fit native cells: {path}")
    canvas = Image.new("L", (WIDTH*COUNT*SUPER, HEIGHT*SUPER))
    draw = ImageDraw.Draw(canvas)
    baseline = SUPER-top
    for i, b in enumerate(bounds):
        draw.text((i*WIDTH*SUPER+SUPER-b[0], baseline), chr(i+32), font=font, anchor="ls", fill=255)
    atlas = canvas.resize((WIDTH*COUNT, HEIGHT), Image.Resampling.LANCZOS)
    advances = [font.getlength(chr(c)) for c in range(32,127)]
    metrics = bytes(max(1,min(255,round(a/SUPER))) for a in advances)
    kern = bytes(max(0,min(255,128+round((font.getlength(chr(a+32)+chr(b+32))-advances[a]-advances[b])/SUPER)))
                 for a in range(COUNT) for b in range(COUNT))
    return atlas.tobytes()+metrics+kern

# ------------------------=
# FUNC: main
# DESC: Builds deterministic face records in stable catalog order without fetching any external resources.
# ------------------=
def main():
    source=(ROOT/"kernel/runtime/font.rs").read_text()
    resources=re.findall(r'b"System/Fonts/([^"\n]+\.ttf)"',source)
    if len(resources)!=46:
        raise ValueError("Update the font pack schema for the new family count")
    result=bytearray()
    for resource in resources:
        for bold in (False,True):
            if resource=="Roboto-Variable.ttf":
                prefix=FONTS/("InfinityUI-Semibold-24" if bold else "InfinityUI-Regular-24")
                data=b"".join(prefix.with_suffix(ext).read_bytes() for ext in (".atlas",".metrics",".kern"))
            else:
                path=FONTS/resource
                if bold and resource=="Arimo-Regular.ttf": path=FONTS/"Arimo-Bold.ttf"
                data=face(path,bold)
            if len(data)!=WIDTH*HEIGHT*COUNT+COUNT+COUNT*COUNT: raise ValueError(resource)
            result.extend(data)
    (FONTS/"InfinityUI-library.bin").write_bytes(result)

if __name__=="__main__":
    main()
