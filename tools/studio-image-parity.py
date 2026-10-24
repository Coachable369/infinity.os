"""Produces cross-language upload fixtures through the actual UIKit packaging encoder."""
import importlib.util
import os
from pathlib import Path
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Emits bounded RGBA PNG/BMP fixtures under the repository build tree without changing any authored artwork.
# ------------------=
def main():
    if not os.environ.get("INFINITY_BUILD_KIT_ACTIVE"):
        raise SystemExit("Use build-kit for Studio fixture generation")
    root = Path(__file__).resolve().parents[1]
    spec = importlib.util.spec_from_file_location("uikit_compiler", root / "tools/compile-uikit.py")
    compiler = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(compiler)
    destination = root / "builds/.studio-work/behavior-tests"
    destination.mkdir(parents=True, exist_ok=True)
    alphas = [0, 1, 2, 17, 64, 127, 128, 192, 254, 255]
    pixels = [(x * 37 % 256, (x * 61 + 7) % 256, (x * 97 + 23) % 256, alphas[x % 10]) for x in range(100)]
    image = Image.new("RGBA", (10, 10))
    image.putdata(pixels)
    png = destination / "studio-native-upload.png"
    image.save(png)
    (destination / "studio-native-upload.bmp").write_bytes(compiler.normalized_artwork(png, 10, 10, "bmp"))
    (destination / "studio-native-upload.rgba").write_bytes(image.tobytes())

if __name__ == "__main__":
    main()
