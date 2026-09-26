"""Select WebPKI and the pinned native HTTPS roots, never a host trust store."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib


# ------------------------=
# FUNC: main
# DESC: Stages the existing verifier with native trust anchors and unchanged chain/signature validation.
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
    packages = tomllib.loads(original)["package"]
    directory, package = helper.stage(root, packages, "rustls-platform-verifier", "0.7.0")
    path = directory / "src/verification/mod.rs"
    text = path.read_text().replace('any(unix, target_arch = "wasm32")',
                                    'any(unix, target_arch = "wasm32", all(target_os = "none", infinity_native))')
    path.write_text(text)
    path = directory / "src/verification/others.rs"
    text = path.read_text()
    marker = '        // While we ignore invalid certificates from the system, we forward errors from'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected WebPKI root initialization")
    text = text.replace(marker, '''        #[cfg(any(all(target_os = "none", infinity_native), infinity_certificate_test))]
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

''' + marker, 1)
    text = text.replace('            unix,', '            unix,\n            not(infinity_certificate_test),', 1)
    path.write_text(text)
    # Exactly the same version and checksum as the native HTTP service.
    native_packages = tomllib.loads((root / "kernel/runtime/http/Cargo.lock").read_text())["package"]
    roots = next(p for p in packages if p["name"] == "webpki-roots")
    native_roots = next(p for p in native_packages if p["name"] == "webpki-roots")
    if (roots["version"], roots["checksum"]) != (native_roots["version"], native_roots["checksum"]):
        raise SystemExit("Browser and native HTTPS trust roots differ")
    path = directory / "Cargo.toml"
    path.write_text(path.read_text() + '\n[target.\'cfg(target_os = "none")\'.dependencies.webpki-roots]\nversion = "=' + roots["version"] + '"\ndefault-features = false\n')
    lock = servo / "Cargo.lock"
    header = 'name = "rustls-platform-verifier"\nversion = "0.7.0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    text = lock.read_text().replace(entry, header)
    start = text.index(header)
    end = text.index("\n[[package]]", start)
    block = text[start:end].replace(' "webpki-root-certs",', ' "webpki-root-certs",\n "webpki-roots",')
    lock.write_text(text[:start] + block + text[end:])


if __name__ == "__main__":
    main()
