"""Pack generated point-cloud turntable cells for bounded native composition."""
import os
from pathlib import Path
from PIL import Image

assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
source = Image.open(kit / "face-digital-turn-v1.png").convert("RGBA")
assert source.getextrema()[3][0] == 0
frames = bytearray()
for row in range(4):
    for col in range(4):
        frame = source.crop((col*source.width//4, row*source.height//4,
                             (col+1)*source.width//4, (row+1)*source.height//4))
        frames.extend(frame.resize((256, 256), Image.Resampling.LANCZOS).tobytes())
assert len(frames) == 16*256*256*4
(kit / "face-digital-turn-v1.rgba").write_bytes(frames)
