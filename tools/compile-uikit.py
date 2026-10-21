"""Compile Studio's bounded UIKit document and apply reversible artwork overrides."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
from PIL import Image

KIT = Path("assets/ui-design-kit")
DOCUMENT = KIT / "uikit.infinityui"
CATALOG = KIT / "uikit-catalog.json"
BASELINES = KIT / "uikit-baselines.json"
BUILDERS = {
    "tools/build-precision-artwork.py": ["eclipse-precision-v1.rgba"],
    "tools/build-access-artwork.py": ["eclipse-access-glyphs-v1.rgba", "eclipse-access-wallpaper-v1.bmp"],
    "tools/build-browser-glyphs.py": ["browser-glyphs-v1.rgba"],
    "tools/build-window-controls.py": ["window-controls-v1.rgba"],
    "tools/build-assistant-artwork.py": ["ai-window-tab-v1.rgba"],
    "tools/build-wait-artwork.py": ["wait-orbit-v2.rgba"],
    "tools/build-browser-welcome-artwork.py": ["browser-welcome-hero-v1.bmp"],
}

# ------------------------=
# FUNC: digest
# DESC: Computes a stable content identity for conflict detection and behavioral parity.
# ------------------=
def digest(data):
    return hashlib.sha256(data).hexdigest()

# ------------------------=
# FUNC: safe_path
# DESC: Resolves an existing or prospective path without allowing absolute, parent, or symlink escapes.
# ------------------=
def safe_path(root, value, prefix):
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError("UIKit paths must remain inside their declared asset directory")
    resolved = (root / path).resolve()
    if not resolved.is_relative_to((root / prefix).resolve()):
        raise ValueError("UIKit path escapes its asset directory")
    return resolved

# ------------------------=
# FUNC: write_changed
# DESC: Publishes a complete artifact atomically while preserving its mtime when bytes are unchanged.
# ------------------=
def write_changed(path, data):
    if path.exists() and path.read_bytes() == data:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_name(path.name + ".uikit-partial")
    pending.write_bytes(data)
    pending.replace(path)

# ------------------------=
# FUNC: token_bytes
# DESC: Encodes all semantic ARGB values into the fixed 64-byte no-std runtime contract.
# ------------------=
def token_bytes(document):
    colors = document.get("colors")
    if document.get("version") != 1 or not isinstance(colors, list) or len(colors) != 13:
        raise ValueError("UIKit version 1 requires exactly thirteen semantic colors")
    if any(type(color) is not int or not 0 <= color <= 0xffffffff for color in colors):
        raise ValueError("UIKit colors must be unsigned 32-bit ARGB integers")
    return b"IUIKIT01" + struct.pack("<HH13I", 1, 13, *colors)

# ------------------------=
# FUNC: style_bytes
# DESC: Encodes validated optional surface recipes without altering approved artwork or legacy documents.
# ------------------=
def style_bytes(document):
    styles = document.get("styles", [])
    if styles is None:
        styles = []
    if not isinstance(styles, list) or len(styles) not in (0, 6):
        raise ValueError("UIKit styles require exactly six surface recipes")
    records = bytearray()
    for index, style in enumerate(styles or [{} for _ in range(6)]):
        if not isinstance(style, dict):
            raise ValueError("UIKit surface recipe must be an object")
        material, corners, radius = (style.get(key, 0) for key in ("material", "corners", "radius"))
        if any(type(value) is not int for value in (material, corners, radius)) or not 0 <= material <= 2 or not 0 <= corners <= 2 or not 0 <= radius <= 40:
            raise ValueError("UIKit material, corners or radius is outside its native range")
        if index < 2 and corners == 2:
            raise ValueError("OS-owned window frames and substrates use authored or square geometry")
        flags = 0
        colors = []
        for bit, key in ((1, "fill"), (2, "border")):
            color = style.get(key)
            if color is not None:
                if type(color) is not int or not 0 <= color <= 0xffffffff:
                    raise ValueError("UIKit style colors must be unsigned ARGB integers")
                flags |= bit
            colors.append(color or 0)
        records.extend(struct.pack("<4B2I", material, corners, radius, flags, *colors))
    return b"IUISTY01" + struct.pack("<HH", 1, 6) + records

# ------------------------=
# FUNC: native_bitmap
# DESC: Encodes new uploaded artwork as top-down V4 BGRA with explicit alpha masks matching Studio and the native painter.
# ------------------=
def native_bitmap(art):
    art = art.convert("RGBA")
    width, height = art.size
    pixels = art.tobytes("raw", "BGRA")
    header = bytearray(122)
    header[:2] = b"BM"
    struct.pack_into("<I", header, 2, len(header) + len(pixels))
    struct.pack_into("<I", header, 10, len(header))
    struct.pack_into("<IiiHHII", header, 14, 108, width, -height, 1, 32, 3, len(pixels))
    struct.pack_into("<5I", header, 54, 0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000, 0x73524742)
    return bytes(header) + pixels

# ------------------------=
# FUNC: normalized_artwork
# DESC: Preserves straight alpha and antialiasing while fitting an uploaded canvas to its declared native cell.
# ------------------=
def normalized_artwork(path, width, height, format):
    if not 0 < width <= 8192 or not 0 < height <= 8192 or width * height > 32_000_000:
        raise ValueError("UIKit artwork dimensions exceed the bounded native canvas")
    if path.stat().st_size > 128 * 1024 * 1024:
        raise ValueError("UIKit artwork exceeds the import byte limit")
    with Image.open(path) as uploaded:
        if uploaded.width * uploaded.height > 32_000_000:
            raise ValueError("UIKit source image exceeds the pixel limit")
        art = uploaded.convert("RGBA")
        if art.size != (width, height):
            art = art.convert("RGBa").resize((width, height), Image.Resampling.LANCZOS).convert("RGBA")
        if format == "rgba":
            return art.tobytes()
        if format == "bmp":
            return native_bitmap(art)
        output = io.BytesIO()
        art.save(output, format=format.upper())
        return output.getvalue()

# ------------------------=
# FUNC: prepare_overrides
# DESC: Validates every imported edit before touching artifacts, accepting only catalog cells or bounded native image files.
# ------------------=
def prepare_overrides(root, document, catalog):
    result = []
    used = set()
    spans = {}
    allowed = {entry["id"]: entry for entry in catalog["elements"]}
    for override in document.get("overrides", []):
        identity = override["id"]
        if identity in used:
            raise ValueError("Duplicate UIKit override identifier")
        used.add(identity)
        format = override.get("format", "rgba")
        target = safe_path(root, override["target"], "assets")
        source = safe_path(root, override["source"], KIT / "overrides")
        if target.is_relative_to((root / KIT / "overrides").resolve()):
            raise ValueError("Override imports cannot be runtime destinations")
        if format == "rgba":
            expected = allowed.get(identity)
            if expected is None or any(override[key] != expected[key] for key in ("target", "offset", "width", "height")):
                raise ValueError("UIKit cell does not match the native catalog contract")
        elif format in ("png", "bmp"):
            if target.suffix.lower() != "." + format or override["offset"] != 0 or not target.is_file():
                raise ValueError("Whole-image edits require an existing native PNG or BMP destination")
        else:
            raise ValueError("Unsupported UIKit artwork format")
        span = (override["offset"], override["offset"] + override["width"] * override["height"] * 4)
        if any(span[0] < previous[1] and previous[0] < span[1] for previous in spans.get(target, [])):
            raise ValueError("UIKit overrides overlap the same native artwork")
        spans.setdefault(target, []).append(span)
        result.append((override, target, normalized_artwork(source, override["width"], override["height"], format)))
    return result

# ------------------------=
# FUNC: baseline_restorations
# DESC: Checks original and last-applied content hashes before restoring or replacing user-owned artwork.
# ------------------=
def baseline_restorations(root, registry, prepared):
    if registry.get("version") != 1:
        raise ValueError("Unsupported UIKit baseline registry version")
    originals = {}
    for entry in registry["files"]:
        target = safe_path(root, entry["target"], "assets")
        raw_targets = {str(KIT / "default" / name) for names in BUILDERS.values() for name in names if name.endswith(".rgba")}
        if target.suffix.lower() not in (".png", ".bmp") and entry["target"] not in raw_targets:
            raise ValueError("UIKit baselines are restricted to native image assets")
        source = safe_path(root, entry["source"], KIT / "overrides/baselines")
        data = source.read_bytes()
        if digest(data) != entry["sha256"] or entry["target"] in originals:
            raise ValueError("UIKit baseline hash or identity is invalid")
        current = digest(target.read_bytes())
        if current not in (entry["sha256"], entry.get("appliedSHA256")):
            raise ValueError("Artwork changed outside Studio; reload or reconcile " + entry["target"])
        originals[entry["target"]] = (entry, target, data)
    for override, _, _ in prepared:
        if override.get("format", "rgba") != "rgba" and override["target"] not in originals:
            raise ValueError("Whole-image edits require an immutable original baseline")
    return originals

# ------------------------=
# FUNC: apply_cells
# DESC: Replaces exact approved RGBA spans without affecting neighboring states or atlas geometry.
# ------------------=
def apply_cells(root, prepared):
    atlases = {}
    for override, target, data in prepared:
        if override.get("format", "rgba") != "rgba":
            continue
        atlas = atlases.setdefault(target, bytearray(target.read_bytes()))
        start = override["offset"]
        if start < 0 or start + len(data) > len(atlas):
            raise ValueError("UIKit override exceeds its native atlas")
        atlas[start:start + len(data)] = data
    for target, data in atlases.items():
        write_changed(target, bytes(data))

# ------------------------=
# FUNC: rebuild_icons
# DESC: Rebuilds affected canonical icon families, preserves per-icon uploads, and packs their native desktop and launcher atlases.
# ------------------=
def rebuild_icons(root, originals, active_files):
    families = ("crystal-blue-glass", "luminous-obsidian", "frosted-quartz", "aurora-harmony")
    for family in families:
        directory = root / "assets/icons" / family
        runtime = root / "assets/icons/runtime"
        outputs = [runtime / (family + "-" + group + ".bmp") for group in ("base", "actions")]
        masters = list(directory.glob("master-*.png"))
        records = [(name, target, original) for name, (_, target, original) in originals.items()
                   if target.is_relative_to(directory)]
        stale = any(not output.exists() for output in outputs)
        if not stale and masters:
            stale = max(source.stat().st_mtime_ns for source in masters) > min(output.stat().st_mtime_ns for output in outputs)
        if stale or any(Path(name).name.startswith("master-") for name, _, _ in records):
            subprocess.run(["sh", str(root / "tools/build-icon-themes.sh"), family], cwd=root, check=True)
        if not records:
            continue
        for name, target, original in records:
            if target.suffix.lower() == ".png":
                write_changed(target, active_files.get(name, original))
        for group, rows in (("base", 9), ("actions", 3)):
            atlas = Image.new("RGBA", (640, rows * 128))
            sources = sorted((directory / "128" / group).glob("*.png"))
            if len(sources) != rows * 5:
                raise ValueError("Native icon family has incomplete runtime cells")
            for index, source in enumerate(sources):
                with Image.open(source) as image:
                    if image.size != (128, 128):
                        raise ValueError("Native desktop icon cell dimensions changed")
                    atlas.alpha_composite(image.convert("RGBA"), (index % 5 * 128, index // 5 * 128))
            for format in ("PNG", "BMP"):
                output = io.BytesIO()
                atlas.save(output, format=format)
                write_changed(runtime / (family + "-" + group + "." + format.lower()), output.getvalue())
        subprocess.run([sys.executable, str(root / "tools/build-launcher-icon-atlases.py"), family], cwd=root, check=True)

# ------------------------=
# FUNC: compile_document
# DESC: Restores canonical masters, applies uploads, rebuilds native derivatives, then overlays individual state edits.
# ------------------=
def compile_document(root, stamp=None):
    document = json.loads((root / DOCUMENT).read_text())
    catalog = json.loads((root / CATALOG).read_text())
    registry = json.loads((root / BASELINES).read_text())
    tokens = token_bytes(document)
    styles = style_bytes(document)
    prepared = prepare_overrides(root, document, catalog)
    originals = baseline_restorations(root, registry, prepared)
    active_files = {override["target"]: data for override, _, data in prepared if override.get("format", "rgba") != "rgba"}
    # Master PNGs precede derivative generation. Whole runtime BMP edits follow
    # it, so generated output cannot silently overwrite a saved Studio upload.
    for name, (_, target, original) in originals.items():
        write_changed(target, active_files.get(name, original))
    for builder in BUILDERS:
        subprocess.run([sys.executable, str(root / builder)], cwd=root, check=True)
    rebuild_icons(root, originals, active_files)
    if any(name.startswith("assets/cursors/generated/") for name in originals):
        subprocess.run(["swift", str(root / "tools/build-cursor-assets.swift"), str(root)], cwd=root, check=True)
    for name, (_, target, original) in originals.items():
        if target.suffix.lower() == ".png":
            write_changed(target, active_files.get(name, original))
    # Most application/desktop artwork has a matching editable PNG master and
    # a directly embedded BMP. Regenerate only masters represented in this edit.
    for name, (_, target, _) in originals.items():
        produced = {root / KIT / "default" / output for outputs in BUILDERS.values() for output in outputs}
        if target.suffix.lower() == ".png" and target.with_suffix(".bmp").exists() and target.with_suffix(".bmp") not in produced:
            with Image.open(target) as image:
                write_changed(target.with_suffix(".bmp"), native_bitmap(image))
    for name, (_, target, original) in originals.items():
        if target.suffix.lower() == ".bmp":
            write_changed(target, active_files.get(name, original))
    apply_cells(root, prepared)
    for name, (entry, target, _) in originals.items():
        entry["appliedSHA256"] = digest(target.read_bytes())
    write_changed(root / BASELINES, (json.dumps(registry, indent=2, sort_keys=True) + "\n").encode())
    write_changed(root / KIT / "default/uikit-tokens.bin", tokens)
    write_changed(root / KIT / "default/uikit-styles.bin", styles)
    if stamp:
        write_changed(stamp, (digest(tokens + (root / DOCUMENT).read_bytes()) + "\n").encode())
        stamp.touch()

# ------------------------=
# FUNC: main
# DESC: Runs only under Build Kit with every output contained in the InfinityOS checkout.
# ------------------=
def main():
    if not os.environ.get("INFINITY_BUILD_KIT_ACTIVE"):
        raise SystemExit("Use ./build-kit run python3 tools/compile-uikit.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stamp")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    stamp = safe_path(root, args.stamp, "build") if args.stamp else None
    compile_document(root, stamp)

if __name__ == "__main__":
    main()
