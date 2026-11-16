"""Compare a canonical widget reference and native capture at their original dimensions."""
import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops


# ------------------------=
# FUNC: main
# DESC: Requires identical dimensions and RGBA pixels and optionally writes an opaque per-pixel discrepancy map.
# ------------------=
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference", type=Path)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--diff", type=Path)
    args = parser.parse_args()
    with Image.open(args.reference) as reference, Image.open(args.capture) as capture:
        if reference.size != capture.size:
            print(json.dumps({"match": False, "reference_size": reference.size, "capture_size": capture.size}))
            return 1
        bands = ImageChops.difference(reference.convert("RGBA"), capture.convert("RGBA")).split()
        mask = bands[0]
        for band in bands[1:]:
            mask = ImageChops.lighter(mask, band)
        mask = mask.point(lambda value: 255 if value else 0)
        different = mask.histogram()[255]
        if args.diff:
            mask.save(args.diff)
        print(json.dumps({"match": different == 0, "pixels": mask.width * mask.height,
                          "different_pixels": different, "bounds": mask.getbbox()}))
        return int(different != 0)


if __name__ == "__main__":
    raise SystemExit(main())
