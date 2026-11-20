"""Pack only the two shared window-frame states from the generated Sapphire kit."""
import os
from pathlib import Path
from statistics import median
from PIL import Image

# ------------------------=
# FUNC: main
# DESC: Extracts uniform sapphire bevel profiles and replaces only frame atlas cells, preserving every control and panel byte.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE'), 'Use ./build-kit run'
    kit = Path(__file__).resolve().parents[1] / 'assets/ui-design-kit/default'
    reference = Image.open(kit / 'sapphire-window-frame-v2-reference.png').convert('RGB')
    target = kit / 'eclipse-precision-v1.rgba'
    data = bytearray(target.read_bytes())
    cell_bytes = 256 * 256 * 4
    for cell, left, right in [(0, 788, 1501), (1, 34, 747)]:
        # Median straight rails remove raster grain before stretching. Samples
        # come from empty spans, never from the title or window controls.
        profiles = []
        for depth in [0, 1, 3, 4, 5, 6]:
            top = [reference.getpixel((x, 182 + depth)) for x in range(left + 100, right - 100)]
            side = [reference.getpixel((left + depth, y)) for y in range(280, 470)]
            bottom = [reference.getpixel((x, 553 - depth)) for x in range(left + 100, right - 100)]
            profiles.append(tuple(tuple(int(median(p[c] for p in strip)) for c in range(3)) for strip in [top, side, bottom]))
        edge = Image.new('RGBA', (256, 256))
        for y in range(256):
            for x in range(256):
                depth = min(x, y, 255-x, 255-y)
                if depth >= 6:
                    continue
                rail = 0 if y <= min(x, 255-x, 255-y) else 2 if 255-y <= min(x, 255-x) else 1
                edge.putpixel((x,y), (*profiles[depth][rail], [255,255,255,255,128,40][depth]))
        start = (14 + cell) * cell_bytes
        data[start:start + cell_bytes] = edge.tobytes()
        data[cell * cell_bytes:(cell + 1) * cell_bytes] = bytes(cell_bytes)
    target.write_bytes(data)

if __name__ == '__main__':
    main()
