"""Reject unsupported native remote-debug server startup without POSIX signal emulation."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Keeps WebDriver protocol types but disables its out-of-scope listening server on native targets.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    modules = {}
    for key, name in (("stage", "prepare-async-net.py"), ("native", "prepare-mio.py")):
        spec = importlib.util.spec_from_file_location(key, Path(__file__).with_name(name))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        modules[key] = module
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = modules["stage"].stage(root, tomllib.loads(original)["package"], "webdriver", "0.54.0")
    path = directory / "src/server.rs"
    path.write_text(modules["native"].native_body(path.read_text(), "pub fn start<T, U>(", '''
        let _ = (address, allow_hosts, allow_origins, handler, extension_routes);
        Err(::std::io::Error::from(::std::io::ErrorKind::Unsupported))
    '''))
    lock = servo / "Cargo.lock"
    header = 'name = "webdriver"\nversion = "0.54.0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))
    directory, package = modules["stage"].stage(root, tomllib.loads(original)["package"], "imsz", "0.4.1")
    path = directory / "src/lib.rs"
    text = path.read_text()
    marker = '        return (&self).imsz();'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected stdin adapter")
    path.write_text(text.replace(marker, '''        #[cfg(all(target_os = "none", infinity_native))]
        { return Err(ImError::IO(std::io::Error::from(std::io::ErrorKind::Unsupported))); }
        #[cfg(not(all(target_os = "none", infinity_native)))]
''' + marker, 1))
    header = 'name = "imsz"\nversion = "0.4.1"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
