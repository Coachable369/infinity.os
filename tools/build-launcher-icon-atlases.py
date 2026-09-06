#!/usr/bin/env python3
"""Build compact 256px alpha atlases for the native installed app launcher."""

from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parent.parent
FAMILIES = (
    "crystal-blue-glass",
    "luminous-obsidian",
    "frosted-quartz",
    "aurora-harmony",
)
SOURCES = (
    "base/00-home.png",
    "base/01-user.png",
    "base/04-documents.png",
    "base/08-videos.png",
    "base/09-projects.png",
    "base/10-trash-empty.png",
    "base/12-drive-internal.png",
    "base/19-computer.png",
    "base/23-microphone.png",
    "base/25-terminal.png",
    "base/26-settings.png",
    "base/28-information.png",
    "base/32-shield.png",
    "actions/04-new-file.png",
)


# ------------------------=
# FUNC: normalize_icon
# DESC: Fits visible alpha artwork into a padded 256px launcher cell without distorting proportions.
# ------------------=
def normalize_icon(icon: Image.Image) -> Image.Image:
    bounds = icon.getchannel("A").getbbox()
    if bounds is None:
        raise ValueError("launcher icon has no visible alpha pixels")
    visible = icon.crop(bounds)
    visible.thumbnail((224, 224), Image.Resampling.LANCZOS)
    normalized = Image.new("RGBA", (256, 256), (0, 0, 0, 0))
    normalized.alpha_composite(
        visible,
        ((256 - visible.width) // 2, (256 - visible.height) // 2),
    )
    return normalized


# ------------------------=
# FUNC: build_family
# DESC: Packs the launcher role subset into a deterministic 4x4 transparent BGRA bitmap.
# ------------------=
def build_family(family: str) -> None:
    atlas = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    source_root = ROOT / "assets" / "icons" / family / "256"
    for index, relative in enumerate(SOURCES):
        icon = Image.open(source_root / relative).convert("RGBA")
        if icon.size != (256, 256):
            raise ValueError(f"{family}/{relative} is not 256x256")
        atlas.alpha_composite(
            normalize_icon(icon),
            ((index % 4) * 256, (index // 4) * 256),
        )
    output = ROOT / "assets" / "icons" / "runtime" / f"{family}-launcher-256.bmp"
    atlas.save(output, format="BMP")


# ------------------------=
# FUNC: main
# DESC: Rebuilds the high-resolution installed launcher resources for all selectable themes.
# ------------------=
def main() -> None:
    for family in FAMILIES:
        build_family(family)


if __name__ == "__main__":
    main()
