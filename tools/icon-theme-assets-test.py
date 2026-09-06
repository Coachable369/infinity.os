#!/usr/bin/env python3
"""Validate installed icon-family dimensions, semantic coverage, and transparency."""

import csv
import struct
from pathlib import Path

from PIL import Image


FAMILIES = ("crystal-blue-glass", "luminous-obsidian", "frosted-quartz")
SIZES = (24, 32, 48, 64, 96, 128, 256)
GROUP_COUNTS = {"base": 45, "actions": 15}


# ------------------------=
# FUNC: manifest_names
# DESC: Loads the ordered semantic filenames expected for one icon group.
# ------------------=
def manifest_names(manifest: Path, group: str) -> list[str]:
    with manifest.open(newline="", encoding="utf-8") as source:
        return [row["id"] for row in csv.DictReader(source) if row["group"] == group]


# ------------------------=
# FUNC: validate_icon
# DESC: Verifies one packaged PNG is square RGBA artwork with clear and translucent pixels.
# ------------------=
def validate_icon(path: Path, size: int) -> None:
    with Image.open(path) as image:
        assert image.size == (size, size), f"wrong dimensions: {path}: {image.size}"
        assert image.mode == "RGBA", f"missing RGBA channels: {path}: {image.mode}"
        alpha = image.getchannel("A")
        minimum, maximum = alpha.getextrema()
        assert minimum == 0, f"background is not transparent: {path}"
        assert maximum > 0, f"icon has no visible pixels: {path}"
        assert any(alpha.histogram()[1:255]), (
            f"icon has no translucent edge or glass pixels: {path}"
        )


# ------------------------=
# FUNC: validate_launcher_atlas
# DESC: Verifies the installed launcher uses native 256px alpha cells rather than the compact UI atlas.
# ------------------=
def validate_launcher_atlas(path: Path) -> None:
    payload = path.read_bytes()
    assert payload[:2] == b"BM", f"launcher atlas is not a bitmap: {path}"
    offset = struct.unpack_from("<I", payload, 10)[0]
    width, height = struct.unpack_from("<ii", payload, 18)
    bits_per_pixel = struct.unpack_from("<H", payload, 28)[0]
    assert (width, abs(height), bits_per_pixel) == (1024, 1024, 32), (
        f"wrong launcher atlas format: {path}"
    )
    alpha = payload[offset + 3 :: 4]
    assert min(alpha) == 0 and max(alpha) == 255, f"launcher atlas lost alpha: {path}"
    for index in range(14):
        cell_x = (index % 4) * 256
        cell_y = (index // 4) * 256
        assert any(
            alpha[(abs(height) - 1 - (cell_y + y)) * width + cell_x + x]
            for y in range(256)
            for x in range(256)
        ), f"launcher atlas cell {index} is empty: {path}"


# ------------------------=
# FUNC: validate_generated_master
# DESC: Verifies a generated source master retains native 256px-or-better cells and real RGBA transparency.
# ------------------=
def validate_generated_master(path: Path, rows: int) -> None:
    with Image.open(path) as image:
        assert image.mode == "RGBA", f"generated master is not RGBA: {path}"
        assert image.width // 5 >= 256 and image.height // rows >= 256, (
            f"generated master lacks 256px source fidelity: {path}: {image.size}"
        )
        alpha = image.getchannel("A")
        minimum, maximum = alpha.getextrema()
        assert minimum == 0 and maximum == 255, f"generated master lost alpha range: {path}"
        assert any(alpha.histogram()[1:255]), f"generated master lost partial alpha: {path}"


# ------------------------=
# FUNC: main
# DESC: Validates every semantic icon in every installed family and supported pixel tier.
# ------------------=
def main() -> None:
    root = Path("assets/icons")
    manifest = root / "manifest.csv"
    checked = 0
    for family in FAMILIES:
        for size in SIZES:
            for group, expected_count in GROUP_COUNTS.items():
                names = manifest_names(manifest, group)
                assert len(names) == expected_count
                files = sorted((root / family / str(size) / group).glob("*.png"))
                assert len(files) == expected_count, (
                    f"incomplete icon tier: {family}/{size}/{group}: {len(files)}"
                )
                expected = [f"{index:02d}-{name}.png" for index, name in enumerate(names)]
                assert [path.name for path in files] == expected, (
                    f"semantic ordering mismatch: {family}/{size}/{group}"
                )
                for path in files:
                    validate_icon(path, size)
                    checked += 1
        validate_launcher_atlas(root / "runtime" / f"{family}-launcher-256.bmp")
    validate_generated_master(root / "luminous-obsidian" / "master-base-v2.png", 9)
    validate_generated_master(root / "luminous-obsidian" / "master-actions-v2.png", 3)
    assert checked == len(FAMILIES) * len(SIZES) * sum(GROUP_COUNTS.values())
    print(f"PASS icon assets: {checked} semantic PNGs have exact dimensions and alpha")


if __name__ == "__main__":
    main()
