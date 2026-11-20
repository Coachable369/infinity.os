"""Extract shared Sapphire controls from the generated, approved visual system."""
import importlib.util
import os
from pathlib import Path
from statistics import median
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Packs reference-derived surfaces in the existing native nine-slice ABI without changing explicit glass recipes.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    root = Path(__file__).resolve().parents[1]
    kit = root / "assets/ui-design-kit/default"
    spec = importlib.util.spec_from_file_location("precision", root / "tools/build-precision-artwork.py")
    precision = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(precision)
    board = Image.open(kit / "sapphire-controls-reference.png").convert("RGB")
    # Empty edge columns avoid text and icons. Corners remain native pixels.
    boxes = [
        ((778, 763, 1120, 986), 18, 170, True),
        ((778, 763, 1120, 986), 18, 170, True),
        ((33, 814, 437, 968), 12, 14, False),
        ((867, 201, 1038, 239), 12, 14, False),
        ((639, 145, 808, 182), 8, 12, False),
        ((866, 145, 1035, 182), 8, 12, False),
        ((1093, 145, 1262, 182), 8, 12, False),
        ((636, 258, 825, 296), 8, 12, False),
        ((1089, 258, 1277, 296), 8, 12, False),
    ]
    layers = [precision.reference_layers(board, *recipe) for recipe in boxes]
    # Keep authored corners and the vertical light falloff, but do not magnify
    # small reference-board compression/noise into long ragged window edges.
    for index, (_, edge) in enumerate(layers):
        cap = boxes[index][1]
        for y in (*range(cap), *range(256-cap,256)):
            strip=[edge.getpixel((x,y)) for x in range(cap,256-cap)]
            color=tuple(int(median(pixel[c] for pixel in strip)) for c in range(4))
            for x in range(cap,256-cap): edge.putpixel((x,y),color)
    inactive=layers[0][1]
    inactive.putalpha(inactive.getchannel('A').point(lambda alpha:alpha*3//5))
    # Solid, optical thumb artwork; no outline-only inactive knob.
    for box in [(746, 628, 767, 649), (699, 575, 721, 597)]:
        art = board.crop(box).convert("RGBA")
        for y in range(art.height):
            for x in range(art.width):
                dx = (x + .5) / art.width * 2 - 1
                dy = (y + .5) / art.height * 2 - 1
                if dx * dx + dy * dy > 1:
                    art.putpixel((x, y), (0, 0, 0, 0))
        layers.append((art.resize((256, 256), Image.Resampling.LANCZOS), Image.new("RGBA", (256, 256))))
    for box in [(640, 201, 811, 239), (867, 201, 1038, 239), (1094, 201, 1265, 239)]:
        layers.append(precision.reference_layers(board, box, 8, 12))
    packed = b"".join(cell[layer].tobytes() for layer in range(2) for cell in layers)
    (kit / "eclipse-precision-v1.rgba").write_bytes(packed)
    # UIKit rebuilds this atlas before applying user overrides. Preserve the
    # approved frame family through that production generation path too.
    import runpy
    runpy.run_path(str(root / "tools/build-window-frame-artwork.py"), run_name="__main__")

if __name__ == "__main__":
    main()
