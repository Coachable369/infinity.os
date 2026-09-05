#!/usr/bin/env python3
"""Extract irregular AI-generated icon grids into clean semantic PNG resources."""

import argparse
import csv
from pathlib import Path

from PIL import Image


# ------------------------=
# FUNC: normalize_transparency
# DESC: Removes generator fringe noise while preserving useful partial alpha in translucent glass.
# ------------------=
def normalize_transparency(atlas: Image.Image) -> Image.Image:
    red, green, blue, alpha = atlas.split()
    alpha = alpha.point(lambda value: 0 if value <= 48 else value)
    return Image.merge("RGBA", (red, green, blue, alpha))


# ------------------------=
# FUNC: axis_projection
# DESC: Measures visible alpha coverage along one atlas axis for transparent-gap discovery.
# ------------------=
def axis_projection(alpha: Image.Image, horizontal: bool) -> list[int]:
    width, height = alpha.size
    pixels = alpha.load()
    if horizontal:
        return [sum(pixels[x, y] > 16 for y in range(height)) for x in range(width)]
    return [sum(pixels[x, y] > 16 for x in range(width)) for y in range(height)]


# ------------------------=
# FUNC: transparent_boundaries
# DESC: Finds the nearest low-alpha valley around each expected row or column division.
# ------------------=
def transparent_boundaries(alpha: Image.Image, divisions: int, horizontal: bool) -> list[int]:
    extent = alpha.width if horizontal else alpha.height
    projection = axis_projection(alpha, horizontal)
    visible = alpha.getbbox()
    if visible is None:
        raise ValueError("atlas has no visible pixels")
    content_start = visible[0] if horizontal else visible[1]
    content_end = visible[2] if horizontal else visible[3]
    step = (content_end - content_start) / divisions
    radius = max(4, round(step * 0.36))
    boundaries = [0]
    for division in range(1, divisions):
        expected = round(content_start + step * division)
        start = max(boundaries[-1] + 1, expected - radius)
        end = min(extent - 1, expected + radius)
        boundary = min(range(start, end + 1), key=lambda value: (projection[value], abs(value - expected)))
        boundaries.append(boundary)
    boundaries.append(extent)
    return boundaries


# ------------------------=
# FUNC: semantic_names
# DESC: Loads the stable ordered names for one icon atlas group from the shared catalog.
# ------------------=
def semantic_names(manifest: Path, group: str) -> list[str]:
    with manifest.open(newline="", encoding="utf-8") as source:
        return [row["id"] for row in csv.DictReader(source) if row["group"] == group]


# ------------------------=
# FUNC: extract_icon
# DESC: Isolates one transparent cell, trims its visible artwork, and preserves breathing room.
# ------------------=
def extract_icon(atlas: Image.Image, box: tuple[int, int, int, int]) -> Image.Image:
    cell = atlas.crop(box)
    visible = cell.getchannel("A").getbbox()
    if visible is None:
        raise ValueError(f"atlas cell has no visible pixels: {box}")
    icon = cell.crop(visible)
    padding_x = max(2, icon.width // 24)
    padding_y = max(2, icon.height // 24)
    padded = Image.new("RGBA", (icon.width + padding_x * 2, icon.height + padding_y * 2))
    padded.alpha_composite(icon, (padding_x, padding_y))
    return padded


# ------------------------=
# FUNC: render_sizes
# DESC: Writes one extracted semantic icon at every supported square desktop size.
# ------------------=
def render_sizes(icon: Image.Image, root: Path, group: str, index: int, name: str, sizes: list[int]) -> None:
    for size in sizes:
        destination = root / str(size) / group
        destination.mkdir(parents=True, exist_ok=True)
        rendered = icon.copy()
        rendered.thumbnail((size, size), Image.Resampling.LANCZOS)
        square = Image.new("RGBA", (size, size))
        square.alpha_composite(rendered, ((size - rendered.width) // 2, (size - rendered.height) // 2))
        square.save(destination / f"{index:02d}-{name}.png", optimize=True)


# ------------------------=
# FUNC: main
# DESC: Slices one irregular generated master into a complete ordered multi-resolution icon group.
# ------------------=
def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("master", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("group", choices=("base", "actions"))
    parser.add_argument("rows", type=int)
    parser.add_argument("sizes")
    parser.add_argument("--manifest", type=Path, default=Path("assets/icons/manifest.csv"))
    arguments = parser.parse_args()

    atlas = normalize_transparency(Image.open(arguments.master).convert("RGBA"))
    names = semantic_names(arguments.manifest, arguments.group)
    if len(names) != arguments.rows * 5:
        raise ValueError("semantic catalog does not match the requested atlas geometry")
    columns = transparent_boundaries(atlas.getchannel("A"), 5, True)
    rows = transparent_boundaries(atlas.getchannel("A"), arguments.rows, False)
    sizes = [int(value) for value in arguments.sizes.split(",")]
    for index, name in enumerate(names):
        column = index % 5
        row = index // 5
        icon = extract_icon(
            atlas,
            (columns[column], rows[row], columns[column + 1], rows[row + 1]),
        )
        render_sizes(icon, arguments.output, arguments.group, index, name, sizes)


if __name__ == "__main__":
    main()
