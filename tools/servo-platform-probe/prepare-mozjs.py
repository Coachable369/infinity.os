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
    text = path.read_text().replace('CONFIGURE_FLAGS += --target=$(TARGET)', 'CONFIGURE_FLAGS += --target=$(subst -none-softfloat,-none,$(TARGET))', 1)
    # Configure probes the compiler before applying CFLAGS; carry the native
    # identity in the target compiler command, never in HOST_CC/HOST_CXX.
    text = ('NATIVE_CC_FLAGS := $(if $(findstring -none,$(TARGET)),$(CFLAGS),)\n'
            'NATIVE_CXX_FLAGS := $(if $(findstring -none,$(TARGET)),$(CXXFLAGS),)\n') + text
    for compiler in ("CC", "CXX"):
        text = text.replace(compiler + '="$(' + compiler + ')"',
                            compiler + '="$(' + compiler + ') $(NATIVE_' + compiler + '_FLAGS)"')
    text = text.replace('CONFIGURE_INPUTS := "', 'CONFIGURE_INPUTS := "$(NATIVE_CC_FLAGS)$(NATIVE_CXX_FLAGS)', 1)
    path.write_text(text)
    path = directory / "mozjs/build/moz.configure/init.configure"
    text = path.read_text()
    marker = '    elif os.startswith("wasi") and allow_wasi:'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected SpiderMonkey platform selector")
    path.write_text(text.replace(marker, '    elif os == "none":\n        canonical_os = canonical_kernel = "Infinity"\n' + marker, 1))
    path = directory / "mozjs/python/mozbuild/mozbuild/configure/constants.py"
    text = path.read_text()
    for kind in ("OS", "Kernel"):
        marker = 'class ' + kind + '(EnumString):\n    POSSIBLE_VALUES = ('
        if text.count(marker) != 1:
            raise SystemExit("Unexpected platform enum")
        text = text.replace(marker, marker + '\n        "Infinity",', 1)
    marker = 'kernel_preprocessor_checks = {'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected kernel compiler checks")
    text = text.replace(marker, marker + '\n    "Infinity": "__INFINITYOS__",', 1)
    path.write_text(text)
    lock = servo / "Cargo.lock"
    header = 'name = "mozjs_sys"\nversion = "153.3.0-0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
