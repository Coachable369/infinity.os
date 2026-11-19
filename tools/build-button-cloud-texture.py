"""Package the existing UIKit preview clouds as persistent glass-button artwork."""
from pathlib import Path
from PIL import Image, ImageOps

# ------------------------=
# FUNC: main
# DESC: Extracts the approved preview backdrop into identical native and Studio texture resources.
# ------------------=
def main():
    root = Path(__file__).resolve().parent.parent
    kit = root / "assets/ui-design-kit/default"
    image = Image.open(kit / "eclipse-access-wallpaper-v1.bmp").convert("RGB")
    texture = ImageOps.fit(image, (256, 72), method=Image.Resampling.LANCZOS)
    data = texture.tobytes()
    (kit / "glass-button-clouds-v1.rgb").write_bytes(data)
    (root / "tools/installer-designer/Sources/InfinityInstallerStudio/Resources/glass-button-clouds-v1.rgb").write_bytes(data)

if __name__ == "__main__":
    main()
