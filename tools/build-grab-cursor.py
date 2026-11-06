"""Package the approved hand cursor master as a small straight-alpha runtime sprite."""
import os
from pathlib import Path
from PIL import Image
assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
source = Image.open(kit / "grab-cursor-v1.png").convert("RGBA")
sprite = source.resize((56, 56), Image.Resampling.LANCZOS)
(kit / "grab-cursor-v1.rgba").write_bytes(sprite.tobytes())
sprite.save(kit / "grab-cursor-v1-preview.png")
