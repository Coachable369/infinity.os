"""Derive the welcome hero's native alpha bitmap from its generated master."""
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Produces a bounded BGRA bitmap without changing the generated composition.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    image = Image.open(kit / "browser-welcome-hero-v1.png").convert("RGBA")
    image.thumbnail((960, 540), Image.Resampling.LANCZOS)
    assert image.getextrema()[3][0] == 0
    image.save(kit / "browser-welcome-hero-v1.bmp")

if __name__ == "__main__":
    main()
