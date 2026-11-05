"""Render a Settings choice in the packaged Studio and verify bounded keyboard-focus damage."""
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
def render(name, focus):
    path = OUT / f"{name}.rgba"
    subprocess.run([APP, "--simulate-services", "--template", "assets/boot/settings-screens.iuit",
                    "--atlas", "assets/ui-design-kit/default/eclipse-precision-v1.rgba",
                    "--kind", "settings", "--screen", "6", "--width", "1280", "--height", "800",
                    "--control-focus", str(focus), "--output", path], cwd=ROOT, check=True)
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
    changed = ImageChops.difference(normal, focused).getbbox()
    assert changed, "Expected visible keyboard focus on the display selector"
    left, top, right, bottom = changed
    assert left > normal.width // 4 and right > normal.width // 2
    assert 8 < bottom - top < normal.height // 6


if __name__ == "__main__":
    main()
