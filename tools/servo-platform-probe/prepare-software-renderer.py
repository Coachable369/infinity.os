"""Select matching upstream SWGL for the native software viewport only."""
import os
import importlib.util
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
    (servo / relative).with_name("infinity_browser_startup.rs").write_bytes(
        (root / "sdk/infinity-browser-core/startup.rs").read_bytes())
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
    # The fallback clock starts at process initialization, so the first frame
    # can be less than one second old on native hardware (unlike slow emulation).
    spec = importlib.util.spec_from_file_location("async_staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = helper.stage(root, tomllib.loads(original)["package"], "webrender", "0.70.0")
    path = directory / "src/profiler.rs"
    source = path.read_text()
    marker = "let one_second_ago = now - ONE_SECOND_NS;"
    if source.count(marker) != 1:
        raise SystemExit("Pinned profiler history window changed")
    source = source.replace(marker, "let one_second_ago = profiler_window::recent_start(now, ONE_SECOND_NS);")
    source += '\n#[path = "profiler_window.rs"]\nmod profiler_window;\n'
    path.write_text(source)
    path.with_name("profiler_window.rs").write_bytes((root / "sdk/servo-std/profiler_window.rs").read_bytes())
    header = 'name = "' + package["name"] + '"\nversion = "' + package["version"] + '"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
