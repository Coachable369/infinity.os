#!/usr/bin/env python3
"""Build proportional Roboto InfinityOS interface font atlases."""

from pathlib import Path
import sys

from PIL import Image, ImageDraw, ImageFont


CELL_WIDTH = 24
CELL_HEIGHT = 28
FONT_SIZE = 24
BASELINE = 21
FIRST_CHARACTER = 32
LAST_CHARACTER = 126
SUPERSAMPLE = 4


# ------------------------=
# FUNC: render_atlas
# DESC: Rasterizes Roboto into unclipped cells and writes matching proportional advances.
# ------------------=
def render_atlas(
    font_path: Path, output_prefix: Path, variation: str, font_size: int = FONT_SIZE
) -> None:
    cell_width = CELL_WIDTH if font_size == FONT_SIZE else font_size
    cell_height = CELL_HEIGHT if font_size == FONT_SIZE else font_size + 6
    baseline = BASELINE if font_size == FONT_SIZE else round(font_size * BASELINE / FONT_SIZE)
    glyph_count = LAST_CHARACTER - FIRST_CHARACTER + 1
    canvas = Image.new(
        "L",
        (cell_width * glyph_count * SUPERSAMPLE, cell_height * SUPERSAMPLE),
        0,
    )
    draw = ImageDraw.Draw(canvas)
    font = ImageFont.truetype(str(font_path), font_size * SUPERSAMPLE)
    font.set_variation_by_name(variation)
    for index, codepoint in enumerate(range(FIRST_CHARACTER, LAST_CHARACTER + 1)):
        character = chr(codepoint)
        cell_left = index * cell_width * SUPERSAMPLE
        bounds = draw.textbbox((0, baseline * SUPERSAMPLE), character, font=font, anchor="ls")
        x = cell_left + SUPERSAMPLE - bounds[0]
        draw.text(
            (x, baseline * SUPERSAMPLE),
            character,
            font=font,
            fill=255,
            anchor="ls",
        )
    atlas = canvas.resize(
        (cell_width * glyph_count, cell_height),
        Image.Resampling.LANCZOS,
    )
    atlas.save(output_prefix.with_name(output_prefix.name + "-atlas.png"))
    output_prefix.with_suffix(".atlas").write_bytes(atlas.tobytes())
    metrics = bytearray()
    for codepoint in range(FIRST_CHARACTER, LAST_CHARACTER + 1):
        advance = round(font.getlength(chr(codepoint)) / SUPERSAMPLE)
        metrics.append(max(1, min(255, advance)))
    output_prefix.with_suffix(".metrics").write_bytes(metrics)
    kerning = bytearray()
    for left in range(FIRST_CHARACTER, LAST_CHARACTER + 1):
        left_character = chr(left)
        left_advance = font.getlength(left_character)
        for right in range(FIRST_CHARACTER, LAST_CHARACTER + 1):
            right_character = chr(right)
            pair_advance = font.getlength(left_character + right_character)
            adjustment = round(
                (pair_advance - left_advance - font.getlength(right_character))
                / SUPERSAMPLE
            )
            kerning.append(max(0, min(255, adjustment + 128)))
    output_prefix.with_suffix(".kern").write_bytes(kerning)


# ------------------------=
# FUNC: main
# DESC: Validates arguments and writes one installer atlas plus its PNG proof.
# ------------------=
def main() -> int:
    if len(sys.argv) not in (4, 5):
        print(
            "usage: generate-installer-font-atlas.py FONT.ttf OUTPUT_PREFIX VARIATION [FONT_SIZE]",
            file=sys.stderr,
        )
        return 2
    font_path = Path(sys.argv[1])
    output_prefix = Path(sys.argv[2])
    if not font_path.is_file():
        print(f"missing font: {font_path}", file=sys.stderr)
        return 2
    font_size = int(sys.argv[4]) if len(sys.argv) == 5 else FONT_SIZE
    if font_size < 12 or font_size > 64:
        print("font size must be between 12 and 64", file=sys.stderr)
        return 2
    render_atlas(font_path, output_prefix, sys.argv[3], font_size)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
