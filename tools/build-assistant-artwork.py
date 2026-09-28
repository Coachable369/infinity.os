"""Bounded runtime derivative for the default design kit; preserves PNG master."""
import os
from pathlib import Path
from PIL import Image

assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
source = Image.open(kit / "ai-window-tab-v1.png").convert("RGBA")
sprite = source.crop((392, 64, 672, 1472)).resize((112, 416), Image.Resampling.LANCZOS)
(kit / "ai-window-tab-v1.rgba").write_bytes(sprite.tobytes())
