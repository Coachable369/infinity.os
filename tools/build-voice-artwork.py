"""Pack generated transparent stop-audio artwork for the native UIKit renderer."""
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Preserves generated alpha and downsamples once for sharp native-size rendering.
# ------------------=
def main():
    root = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    image = Image.open(root / "stop-audio-v1.png").convert("RGBA")
    image = image.crop(image.getbbox()).resize((128, 128), Image.Resampling.LANCZOS)
    image.save(root / "stop-audio-v1.bmp")

if __name__ == "__main__":
    main()
