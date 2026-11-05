"""Render a Settings choice in the packaged Studio and verify bounded keyboard-focus damage."""
import json
from pathlib import Path
import subprocess
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "builds/validation/enterprise-controls"
APP = ROOT / "builds/InfinityOS Installer Studio.app/Contents/MacOS/InfinityStudioNativeRenderer"


# ------------------------=
# FUNC: render
# DESC: Renders a complete Devices frame using the packaged renderer with a given keyboard focus.
# ------------------=
def render(name, focus, screen=6, extra=()):
    path = OUT / f"{name}.rgba"
    subprocess.run([APP, "--simulate-services", "--template", "assets/boot/settings-screens.iuit",
                    "--atlas", "assets/ui-design-kit/default/eclipse-precision-v1.rgba",
                    "--kind", "settings", "--screen", str(screen), "--width", "1280", "--height", "800",
                    "--control-focus", str(focus), *extra, "--output", path], cwd=ROOT, check=True)
    frame = Image.frombytes("RGBA", (1280, 800), path.read_bytes()).convert("RGB")
    frame.save(OUT / f"{name}.png")
    return frame


# ------------------------=
# FUNC: main
# DESC: Requires a visible focus transition localized to the display selector and its selectable row.
# ------------------=
def main():
    OUT.mkdir(parents=True, exist_ok=True)
    normal = render("devices", 0)
    focused = render("devices-focused", 128)
    render("general-node-field", 0, 1)
    picker = render("accent-picker", 0, 2, ("--expanded-row", "3", "--scroll", "160"))
    layers = json.loads((OUT / "accent-picker.geometry.json").read_text())
    row = next(e for e in layers if e["id"] == "53595354-0002-0034-8000-000000000001")
    # Sample the standalone hue strip in the fourth row's expanded picker.
    hue = picker.crop((row["x"] + row["width"] - 28, row["y"] + row["height"] + 16,
                       row["x"] + row["width"] - 27, row["y"] + row["height"] + 94))
    assert len({rgb for rgb in hue.getdata() if max(rgb) - min(rgb) > 128}) >= 12
    geometry = json.loads((OUT / "devices.geometry.json").read_text())
    value = next(e for e in geometry if e["id"] == "53595354-0006-002a-8000-000000000001")
    x, y, w, h = (value[k] for k in ("x", "y", "width", "height"))
    row = next(e for e in geometry if e["id"] == "53595354-0006-0028-8000-000000000001")
    h = min(36, row["height"])
    y = row["y"] + (row["height"] - h) // 2
    for frame in (normal, focused):
        # Exclude the surface border and indicator well; measure the light text ink.
        ink = frame.crop((x+12, y+2, x+w-40, y+h-2)).convert("L").point(lambda v: 255 if v > 135 else 0).getbbox()
        assert ink is not None
        top_gap, bottom_gap = ink[1], h-4-ink[3]
        assert abs(top_gap-bottom_gap) <= 2, (top_gap, bottom_gap)
    changed = ImageChops.difference(normal, focused).getbbox()
    assert changed, "Expected visible keyboard focus on the display selector"
    left, top, right, bottom = changed
    assert left > normal.width // 4 and right > normal.width // 2
    assert 8 < bottom - top < normal.height // 6


if __name__ == "__main__":
    main()
