#!/usr/bin/env python3
"""Build InfinityOS' fixed-size early UI atlas from an OFL TrueType face."""

from pathlib import Path
import sys

from PIL import Image, ImageDraw, ImageFont


CELL_WIDTH = 20
CELL_HEIGHT = 24
FIRST_CHARACTER = 32
LAST_CHARACTER = 126
SUPERSAMPLE = 4


# ------------------------=
# FUNC: render_atlas
# DESC: Rasterizes printable ASCII into a compact antialiased grayscale atlas.
# ------------------=
def render_atlas(font_path: Path, output_prefix: Path) -> None:
    width = CELL_WIDTH * (LAST_CHARACTER - FIRST_CHARACTER + 1)
    canvas = Image.new("L", (width * SUPERSAMPLE, CELL_HEIGHT * SUPERSAMPLE), 0)
    draw = ImageDraw.Draw(canvas)
    font = ImageFont.truetype(str(font_path), 16 * SUPERSAMPLE)
    baseline = 18 * SUPERSAMPLE
    for index, codepoint in enumerate(range(FIRST_CHARACTER, LAST_CHARACTER + 1)):
        character = chr(codepoint)
        cell_left = index * CELL_WIDTH * SUPERSAMPLE
        bounds = draw.textbbox((0, baseline), character, font=font, anchor="ls")
        x = cell_left + SUPERSAMPLE - bounds[0]
        draw.text((x, baseline), character, font=font, fill=255, anchor="ls")
    atlas = canvas.resize((width, CELL_HEIGHT), Image.Resampling.LANCZOS)
    atlas.save(output_prefix.with_name(output_prefix.name + "-atlas.png"))
    output_prefix.with_suffix(".atlas").write_bytes(atlas.tobytes())


# ------------------------=
# FUNC: main
# DESC: Validates command arguments and writes the PNG proof plus native atlas bytes.
# ------------------=
def main() -> int:
    if len(sys.argv) != 3:
        print("usage: generate-ui-font-atlas.py FONT.ttf OUTPUT_PREFIX", file=sys.stderr)
        return 2
    font_path = Path(sys.argv[1])
    output_prefix = Path(sys.argv[2])
    if not font_path.is_file():
        print(f"missing font: {font_path}", file=sys.stderr)
        return 2
    render_atlas(font_path, output_prefix)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
