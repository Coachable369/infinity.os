"""Select matching upstream SWGL for the native software viewport only."""
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Stages the CPU context and exact locked rasterizer packages without altering host renderers.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    servo = root / "build/servo-port-audit"
    relative = "components/shared/paint/rendering_context.rs"
    source = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    for marker in ("pub struct SoftwareRenderingContext {", "impl SoftwareRenderingContext {",
                   "impl Drop for SoftwareRenderingContext {", "impl RenderingContext for SoftwareRenderingContext {"):
        if source.count(marker) != 1:
            raise SystemExit("Pinned software context changed")
        source = source.replace(marker, '#[cfg(not(all(target_os = "none", infinity_native)))]\n' + marker)
    source += '''
#[cfg(all(target_os = "none", infinity_native))]
#[path = "native_software_context.rs"]
mod native_software_context;
#[cfg(all(target_os = "none", infinity_native))]
pub use native_software_context::SoftwareRenderingContext;
'''
    (servo / relative).write_text(source)
    (servo / relative).with_name("native_software_context.rs").write_bytes(
        (root / "sdk/servo-std/software_rendering_context.rs").read_bytes())
    relative = "components/paint/painter.rs"
    source = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    marker = "webrender::WebRenderOptions {"
    if source.count(marker) != 1:
        raise SystemExit("Pinned renderer options changed")
    # SWGL supports scissored clears directly, not GL_ALWAYS depth quad clears.
    source = source.replace(marker, marker + '''
                #[cfg(all(target_os = "none", infinity_native))]
                clear_caches_with_quads: false,
''')
    (servo / relative).write_text(source)
    relative = "components/shared/paint/Cargo.toml"
    source = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    source += '\n[target.\'cfg(target_os = "none")\'.dependencies.swgl]\nversion = "=0.70.0"\n'
    (servo / relative).write_text(source)
    lock = servo / "Cargo.lock"
    source = lock.read_text()
    blocks = source.split("[[package]]")
    for index, block in enumerate(blocks[1:], 1):
        package = tomllib.loads(block)
        if package["name"] == "servo-paint-api":
            blocks[index] = block.replace(' "surfman",', ' "surfman",\n "swgl",')
    source = "[[package]]".join(blocks)
    # Cargo verifies the registry checksums. These three additions reuse every
    # other existing engine pin, including nom 7 rather than the newer nom 8.
    raster_lock = Path(__file__).with_name("swgl") / "Cargo.lock"
    for block in raster_lock.read_text().split("[[package]]")[1:]:
        package = tomllib.loads(block)
        if package["name"] in ("swgl", "glsl", "glsl-to-cxx"):
            source += "[[package]]" + block.replace(' "nom",', ' "nom 7.1.3",')
    lock.write_text(source)

if __name__ == "__main__":
    main()
