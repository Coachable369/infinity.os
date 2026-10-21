"""Extract the approved board's native corner, lighting and interaction pixels."""
import os
from pathlib import Path
from PIL import Image

SIZE = 256

# ------------------------=
# FUNC: nine_slice
# DESC: Packs authored corners at native resolution and resizes only the stretchable edge/interior spans.
# ------------------=
def nine_slice(art, cap):
    out = Image.new("RGBA", (SIZE, SIZE))
    xs = (0, cap, art.width - cap, art.width)
    ys = (0, cap, art.height - cap, art.height)
    target = (0, cap, SIZE - cap, SIZE)
    for row in range(3):
        for column in range(3):
            patch = art.crop((xs[column], ys[row], xs[column + 1], ys[row + 1]))
            shape = (target[column + 1] - target[column], target[row + 1] - target[row])
            if patch.size != shape:
                patch = patch.resize(shape, Image.Resampling.LANCZOS)
            out.paste(patch, (target[column], target[row]))
    return out

# ------------------------=
# FUNC: reference_layers
# DESC: Removes reference labels between blank inner-edge samples and separates exact edge/glow pixels from the translucent background.
# ------------------=
def reference_layers(reference, box, cap, blank_x, frame=False):
    art = reference.crop(box).convert("RGB")
    fill = Image.new("RGBA", art.size)
    edge = Image.new("RGBA", art.size)
    outside = reference.getpixel((box[0] - 2, box[1] - 2))[:3]
    for y in range(art.height):
        # Both inner-edge columns are outside every example label. Interpolating
        # their authored colors preserves the horizontal light balance and joins
        # the untouched rim without a visible rectangular interior seam.
        row = max(cap, min(art.height - cap - 1, y))
        left = art.getpixel((cap, row))
        right = art.getpixel((art.width - cap - 1, row))
        for x in range(art.width):
            distance = min(x, y, art.width - 1 - x, art.height - 1 - y)
            amount = max(0, min(art.width - 2 * cap - 1, x - cap))
            span = max(1, art.width - 2 * cap - 1)
            base = (art.getpixel((blank_x, y)) if frame else
                    tuple((a * (span - amount) + b * amount) // span for a, b in zip(left, right)))
            if not frame:
                fill.putpixel((x, y), (*base, 0 if distance < 2 else 255))
            # L arms stay 18px long and 4px thick, but their interior gutter is
            # clear. Six inward lighting pixels contain no title icons or labels.
            if distance >= (6 if frame else cap):
                continue
            color = art.getpixel((x, y))
            if distance < 2:
                # Matte only the tiny optical corner/outer-edge coverage against
                # its actual board background; bright authored strokes stay solid.
                coverage = min(255, max(abs(color[c] - outside[c]) for c in range(3)) * 255 // 40)
                edge.putpixel((x, y), (*color, coverage))
                continue
            if frame:
                corner = (x < 18 or x >= art.width - 18) and (y < 18 or y >= art.height - 18)
                if corner and distance < 4:
                    edge.putpixel((x, y), (*color, 255))
                    continue
            # Minimum source-over alpha reproducing the board pixel over its
            # local unlit surface. This keeps the authored glow's falloff instead
            # of turning a 12px rim into an opaque rectangular band.
            alpha = 0
            for value, background in zip(color, base):
                delta = value - background
                divisor = (255 - background) if delta >= 0 else background
                alpha = max(alpha, (abs(delta) * 255 + max(1, divisor) - 1) // max(1, divisor))
            if alpha:
                light = tuple(max(0, min(255, background + round((value - background) * 255 / alpha)))
                              for value, background in zip(color, base))
                edge.putpixel((x, y), (*light, min(alpha, 255)))
    return nine_slice(fill, cap), nine_slice(edge, cap)

# ------------------------=
# FUNC: main
# DESC: Packs reference-derived normal/focused/pressed surfaces plus the existing clean dot artwork into independent fill and edge atlases.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    kit = Path(__file__).resolve().parents[1] / "assets/ui-design-kit/default"
    source = Image.open(kit / "eclipse-precision-v1-source.png").convert("RGBA")
    reference = Image.open(kit / "eclipse-precision-v1-reference.png").convert("RGB")
    assert source.size == reference.size == (1536, 1024)
    # Cell order is the private, versioned native contract in panel_recipe.rs.
    recipes = [
        ((767, 266, 1368, 317), 18, 300, True),
        ((32, 172, 734, 584), 18, 351, True),
        ((1149, 381, 1507, 476), 12, 330, False),
        ((768, 380, 1127, 476), 12, 331, False),
        ((769, 569, 881, 620), 8, 14, False),
        ((894, 569, 1006, 620), 8, 14, False),
        ((1018, 569, 1126, 620), 8, 14, False),
        ((33, 653, 455, 695), 8, 394, False),
        ((33, 707, 455, 750), 8, 394, False),
    ]
    layers = [reference_layers(reference, *recipe) for recipe in recipes]
    for box in [(748, 687, 919, 859), (1190, 688, 1358, 857)]:
        art = source.crop(box)
        for y in range(art.height):
            for x in range(art.width):
                dx = (x + .5) / art.width * 2 - 1
                dy = (y + .5) / art.height * 2 - 1
                if dx * dx + dy * dy > 1:
                    art.putpixel((x, y), (0, 0, 0, 0))
        layers.append((art.resize((SIZE, SIZE), Image.Resampling.LANCZOS), Image.new("RGBA", (SIZE, SIZE))))
    packed = b"".join(cell[layer].tobytes() for layer in range(2) for cell in layers)
    (kit / "eclipse-precision-v1.rgba").write_bytes(packed)

if __name__ == "__main__":
    main()
