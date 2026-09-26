"""Stage WebRender's memory-backed FreeType rasterizer for native targets."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Reuses upstream rasterization without enabling filesystem font lookup or dynamic loading.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("async_staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    packages = tomllib.loads(original)["package"]
    directory, package = helper.stage(root, packages, "wr_glyph_rasterizer", "0.70.0")
    native = 'all(target_os = "none", infinity_native)'
    path = directory / "src/lib.rs"
    text = path.read_text()
    text = text.replace('pub mod platform {', 'pub mod platform {\n    #[cfg(' + native + ')]\n    pub use crate::platform::infinity::font;\n    #[cfg(' + native + ')]\n    pub mod infinity { pub mod font; }', 1)
    path.write_text(text)
    source = directory / "src/platform/unix/font.rs"
    text = source.read_text()
    # Static FreeType symbols only. The backend receives font bytes from the adapter.
    text = text.replace('any(not(target_os = "android"), feature = "dynamic_freetype")', 'not(' + native + ')')
    text = text.replace('all(target_os = "android", not(feature = "dynamic_freetype"))', native)
    start = text.index('                FontTemplate::Native(NativeFontHandle { ref path, index }) => {')
    end = text.index('\n                }', start) + len('\n                }')
    text = text[:start] + '                FontTemplate::Native(_) => return Err(FT_Err_Unimplemented_Feature as FT_Error),' + text[end:]
    destination = directory / "src/platform/infinity/font.rs"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(text)
    path = directory / "Cargo.toml"
    path.write_text(path.read_text() + '''
[target.'cfg(target_os = "none")'.dependencies.freetype]
version = "0.8"
default-features = false
[target.'cfg(target_os = "none")'.dependencies.libc]
version = "0.2"
''')
    lock = servo / "Cargo.lock"
    header = 'name = "' + package["name"] + '"\nversion = "' + package["version"] + '"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))
    freetype, package = helper.stage(root, packages, "freetype-sys", "0.23.0")
    path = freetype / "build.rs"
    path.write_text(path.read_text().replace('if !cfg!(feature = "bundled") {', 'if !cfg!(feature = "bundled") && env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("none") {', 1))
    header = 'name = "' + package["name"] + '"\nversion = "' + package["version"] + '"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
