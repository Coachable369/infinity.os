"""Pack the generated transparent master for allocation-free native painting."""
import os
from pathlib import Path
from PIL import Image

assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
source = Image.open(kit / "face-mask-v1.png").convert("RGBA")
# Runtime derivative only: the original generated master remains unchanged.
source.resize((480, 480), Image.Resampling.LANCZOS).save(kit / "face-mask-v1.bmp")
assert source.getextrema()[3][0] == 0, "Master must have true alpha transparency"
