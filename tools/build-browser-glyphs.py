"""Pack the generated browser glyph sheet into sixteen alpha-preserving cells."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Extracts equal atlas cells and fits their authored alpha bounds without stretching.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit=Path(__file__).resolve().parents[1]/"assets/ui-design-kit/default"
    source=Image.open(kit/"browser-glyphs-v1.png").convert("RGBA")
    packed=[]
    preview=Image.new("RGBA",(384,384),(8,20,34,255))
    for i in range(16):
        x,y=i%4,i//4
        cell=source.crop((x*source.width//4,y*source.height//4,(x+1)*source.width//4,(y+1)*source.height//4))
        bounds=cell.getchannel("A").point(lambda a:255 if a>=128 else 0).getbbox()
        assert bounds is not None
        art=cell.crop(bounds)
        art.thumbnail((52,52),Image.Resampling.LANCZOS)
        out=Image.new("RGBA",(64,64))
        out.alpha_composite(art,((64-art.width)//2,(64-art.height)//2))
        packed.append(out.tobytes())
        preview.alpha_composite(out,(x*96+16,y*96+16))
    (kit/"browser-glyphs-v1.rgba").write_bytes(b"".join(packed))
    preview.save(kit/"browser-glyphs-v1-preview.png")

if __name__=="__main__":main()
