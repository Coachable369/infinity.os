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
    relative = "components/shared/fonts/font_identifier.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '''
#[cfg(all(target_os = "none", infinity_native))]
mod platform {
    use malloc_size_of_derive::MallocSizeOf;
    use serde::{Deserialize, Serialize};
    use crate::{FontData, FontDataAndIndex};
    #[derive(Clone, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
    pub enum LocalFontIdentifier { Sans, Serif, Mono }
    impl LocalFontIdentifier {
        // ------------------------=
        // FUNC: index
        // DESC: Packaged single-face fonts use face zero, not a host font path.
        // ------------------=
        pub fn index(&self) -> u32 { 0 }
        // ------------------------=
        // FUNC: font_data_and_index
        // DESC: Loads immutable packaged bytes without filesystem or ambient font authority.
        // ------------------=
        pub fn font_data_and_index(&self) -> Option<FontDataAndIndex> {
            let bytes: &[u8] = match self {
                Self::Sans => include_bytes!("native-fonts/FiraSans-Regular.ttf"),
                Self::Serif => include_bytes!("native-fonts/EBGaramond.ttf"),
                Self::Mono => include_bytes!("native-fonts/IBMPlexMono-Regular.ttf"),
            };
            Some(FontDataAndIndex { data: FontData::from_bytes(bytes), index: 0 })
        }
    }
}
'''
    (servo / relative).write_text(text)
    font_dir = (servo / relative).parent / "native-fonts"
    font_dir.mkdir(exist_ok=True)
    for name in ("FiraSans-Regular.ttf", "EBGaramond.ttf", "IBMPlexMono-Regular.ttf"):
        (font_dir / name).write_bytes((root / "assets/fonts" / name).read_bytes())
    native = 'all(target_os = "none", infinity_native)'
    for relative in ("components/fonts/platform/mod.rs", "components/fonts/Cargo.toml"):
        text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
        text = text.replace('any(target_os = "linux", target_os = "android", target_os = "freebsd")',
                            'any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "none")')
        (servo / relative).write_text(text)
    relative = "components/fonts/platform/freetype/mod.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text += '\n#[cfg(' + native + ')]\n#[path = "native_font_list.rs"]\npub mod font_list;\n'
    (servo / relative).write_text(text)
    (servo / relative).with_name("native_font_list.rs").write_bytes(
        (root / "sdk/servo-std/native_font_list.rs").read_bytes())
    spec = importlib.util.spec_from_file_location("bodies", Path(__file__).with_name("prepare-mio.py"))
    bodies = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(bodies)
    relative = "components/fonts/platform/freetype/font.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    text = bodies.native_body(text, 'fn new_from_local_font_identifier(', '''
        let data = font_identifier.font_data_and_index().ok_or("Packaged font unavailable")?;
        Self::new_from_data(FontIdentifier::Local(font_identifier), &data.data, requested_size, synthetic_bold)
''')
    (servo / relative).write_text(text)
    relative = "components/fonts/system_font_service.rs"
    text = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:" + relative], text=True)
    marker = '                paint_api.add_system_font(font_key, local_font_identifier.native_font_handle());'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected font registration path")
    text = text.replace(marker, '''                #[cfg(all(target_os = "none", infinity_native))]
                {
                    let font = local_font_identifier.font_data_and_index().expect("Packaged font");
                    paint_api.add_font(font_key, font.data.as_ipc_shared_memory(), font.index);
                }
                #[cfg(not(all(target_os = "none", infinity_native)))]
''' + marker, 1)
    (servo / relative).write_text(text)
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
    zlib, _ = helper.stage(root, packages, "libz-sys", "1.1.29")
    path = freetype / "build.rs"
    text = path.read_text().replace('if !cfg!(feature = "bundled") {', 'if !cfg!(feature = "bundled") && env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("none") {', 1)
    text = text.replace('.include("libz-sys/src/zlib")', '.include("' + str(zlib / "src/zlib") + '")')
    path.write_text(text)
    header = 'name = "' + package["name"] + '"\nversion = "' + package["version"] + '"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
