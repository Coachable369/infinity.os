#!/usr/bin/env python3
"""Assemble the generated Aurora Harmony icon sheets into transparent production masters."""

from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage


ROOT = Path("assets/icons/aurora-harmony")
SOURCE = ROOT / "source-v1"


# ------------------------=
# FUNC: recover_transparency
# DESC: Removes the generated neutral backdrop while retaining enclosed white details, color, shadow, and antialiased edges.
# ------------------=
def recover_transparency(source: Path) -> Image.Image:
    image = Image.open(source).convert("RGB")
    rgb = np.asarray(image, dtype=np.float32)
    maximum = rgb.max(axis=2)
    minimum = rgb.min(axis=2)
    chroma = maximum - minimum
    luminance = rgb[:, :, 0] * 0.2126 + rgb[:, :, 1] * 0.7152 + rgb[:, :, 2] * 0.0722

    background_candidate = (luminance >= 226.0) & (chroma <= 18.0)
    edge_seed = np.zeros(background_candidate.shape, dtype=bool)
    edge_seed[0, :] = background_candidate[0, :]
    edge_seed[-1, :] = background_candidate[-1, :]
    edge_seed[:, 0] = background_candidate[:, 0]
    edge_seed[:, -1] = background_candidate[:, -1]
    background = ndimage.binary_propagation(edge_seed, mask=background_candidate)

    foreground = ~background
    labels, _ = ndimage.label(foreground)
    component_sizes = np.bincount(labels.ravel())
    retained = component_sizes >= 180
    retained[0] = False
    foreground = retained[labels]

    distance = ndimage.distance_transform_edt(foreground)
    alpha = np.clip(distance * 0.72, 0.0, 1.0)
    alpha[~foreground] = 0.0
    rgba = np.dstack((rgb, alpha[:, :, None] * 255.0)).astype(np.uint8)
    rgba[alpha == 0.0, :3] = 0
    return Image.fromarray(rgba, "RGBA")


# ------------------------=
# FUNC: stack_vertical
# DESC: Combines three ordered 5x3 sheets into the standard 5x9 semantic base atlas.
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
# DESC: Writes the full-resolution base and action masters consumed by the common icon build pipeline.
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
    base.save(ROOT / "master-base-v1.png", optimize=True)
    actions.save(ROOT / "master-actions-v1.png", optimize=True)


if __name__ == "__main__":
    main()
