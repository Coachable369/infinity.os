"""Derive smooth native busy-cursor frames from one AI-generated transparent master."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Rotates one stable silhouette at equal angular intervals with premultiplied alpha filtering.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    source = Image.open(kit / "wait-orbit-v2.png").convert("RGBA")
    assert source.width == source.height
    assert source.getextrema()[3][0] == 0 and source.getextrema()[3][1] > 200
    # Fit the complete authored silhouette into a fixed circle with padding for rotation.
    box = source.getchannel("A").point(lambda v: 255 if v > 8 else 0).getbbox()
    source = source.crop(box)
    source.thumbnail((384,384), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA",(512,512))
    canvas.alpha_composite(source,((512-source.width)//2,(512-source.height)//2))
    canvas = canvas.convert("RGBa")
    frames = []
    preview = Image.new("RGBA",(480,80),(7,27,42,255))
    for index in range(60):
        frame = canvas.rotate(-index*6, resample=Image.Resampling.BICUBIC).resize((64,64),Image.Resampling.LANCZOS).convert("RGBA")
        assert frame.getextrema()[3][0] == 0
        assert frame.getextrema()[3][1] > 200
        frames.append(frame.tobytes())
        if index%10==0:
            preview.alpha_composite(frame.resize((32,32),Image.Resampling.LANCZOS),(index//10*80+24,24))
    assert len(set(frames)) == 60
    (kit / "wait-orbit-v2.rgba").write_bytes(b"".join(frames))
    preview.save(kit / "wait-orbit-v2-preview.png")

if __name__ == "__main__":
    main()
