"""Name the native SpiderMonkey target honestly without impersonating a Unix OS."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Normalizes Rust's float-ABI suffix only for configure and adds the explicit Infinity OS identity.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = helper.stage(root, tomllib.loads(original)["package"], "mozjs_sys", "153.3.0-0")
    path = directory / "makefile.cargo"
    path.write_text(path.read_text().replace('CONFIGURE_FLAGS += --target=$(TARGET)', 'CONFIGURE_FLAGS += --target=$(subst -none-softfloat,-none,$(TARGET))', 1))
    path = directory / "mozjs/build/moz.configure/init.configure"
    text = path.read_text()
    marker = '    elif os.startswith("wasi") and allow_wasi:'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected SpiderMonkey platform selector")
    path.write_text(text.replace(marker, '    elif os == "none":\n        canonical_os = canonical_kernel = "Infinity"\n' + marker, 1))
    lock = servo / "Cargo.lock"
    header = 'name = "mozjs_sys"\nversion = "153.3.0-0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
