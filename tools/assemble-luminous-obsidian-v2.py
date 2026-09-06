#!/usr/bin/env python3
"""Assemble generated luminous-obsidian sheets and recover production RGBA transparency."""

from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage


ROOT = Path("assets/icons/luminous-obsidian")
SOURCE = ROOT / "source-v2"


# ------------------------=
# FUNC: recover_transparency
# DESC: Removes the generated neutral checkerboard matte while retaining colored glow and partial-alpha edges.
# ------------------=
def recover_transparency(source: Path) -> Image.Image:
    image = Image.open(source).convert("RGB")
    rgb = np.asarray(image, dtype=np.float32)
    maximum = rgb.max(axis=2)
    minimum = rgb.min(axis=2)
    chroma = maximum - minimum
    luminance = rgb[:, :, 0] * 0.2126 + rgb[:, :, 1] * 0.7152 + rgb[:, :, 2] * 0.0722

    neutral_strength = np.clip((242.0 - luminance) / 18.0, 0.0, 1.0)
    color_strength = np.clip((chroma - 3.0) / 22.0, 0.0, 1.0)
    alpha = np.maximum(neutral_strength, color_strength)
    alpha[alpha < 0.19] = 0.0
    labels, _ = ndimage.label(alpha > 0.0)
    component_sizes = np.bincount(labels.ravel())
    retained = component_sizes >= 900
    retained[0] = False
    alpha[~retained[labels]] = 0.0

    matte = 249.0
    safe_alpha = np.maximum(alpha[:, :, None], 0.08)
    unpremultiplied = (rgb - matte * (1.0 - safe_alpha)) / safe_alpha
    unpremultiplied = np.clip(unpremultiplied, 0.0, 255.0)
    unpremultiplied[alpha == 0.0] = 0.0
    rgba = np.dstack((unpremultiplied, alpha[:, :, None] * 255.0)).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


# ------------------------=
# FUNC: stack_vertical
# DESC: Combines equal-width transparent source sheets into one ordered semantic master atlas.
# ------------------=
def stack_vertical(images: list[Image.Image]) -> Image.Image:
    width = max(image.width for image in images)
    height = sum(image.height for image in images)
    atlas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    top = 0
    for image in images:
        atlas.alpha_composite(image, ((width - image.width) // 2, top))
        top += image.height
    return atlas


# ------------------------=
# FUNC: main
# DESC: Writes the 45-icon base and 15-icon action masters consumed by the standard icon build pipeline.
# ------------------=
def main() -> None:
    base = stack_vertical(
        [
            recover_transparency(SOURCE / "base-00-14.png"),
            recover_transparency(SOURCE / "base-15-29.png"),
            recover_transparency(SOURCE / "base-30-44.png"),
        ]
    )
    actions = recover_transparency(SOURCE / "actions-45-59.png")
    base.save(ROOT / "master-base-v2.png", optimize=True)
    actions.save(ROOT / "master-actions-v2.png", optimize=True)


if __name__ == "__main__":
    main()
