"""Convert the generated horizontal tab master into a bounded runtime sprite."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Preserves the generated master and converts its authored tab crop to RGBA8.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    source = Image.open(kit / "browser-horizontal-tab-v1.png").convert("RGBA")
    assert source.size == (2172, 724)
    sprite = source.crop((0, 190, 2172, 515)).resize((944, 192), Image.Resampling.LANCZOS)
    assert sprite.getextrema()[3][0] == 0
    assert sprite.getpixel((472, 100))[3] > 230
    (kit / "browser-horizontal-tab-v1.rgba").write_bytes(sprite.tobytes())

if __name__ == "__main__":
    main()
