"""Stage the pinned Stylo workspace with native thread identity support."""
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tomllib

# ------------------------=
# FUNC: main
# DESC: Stages an exact Git tree, preserving real thread joins and using opaque std thread IDs.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    revision = "bb8a1c140b5023087e7a8ab51deab7618c184d53"
    sources = list((root / "build/servo-cargo-home/git/checkouts").glob("stylo-*/bb8a1c1"))
    if len(sources) != 1 or subprocess.check_output(["git", "-C", str(sources[0]), "rev-parse", "HEAD"], text=True).strip() != revision:
        raise SystemExit("Unexpected Stylo revision")
    destination = root / "build/servo-native-deps/stylo"
    destination.mkdir(parents=True, exist_ok=True)
    data = subprocess.check_output(["git", "-C", str(sources[0]), "archive", revision])
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        archive.extractall(destination, filter="data")
    path = destination / "style/global_style_data.rs"
    text = path.read_text()
    text = text.replace('/// A noop thread join handle for wasm', '''/// Native identities are opaque; no POSIX handle or cross-process authority.
#[cfg(all(target_os = "none", infinity_native))]
pub type PlatformThreadHandle = std::thread::ThreadId;

/// A noop thread join handle for wasm''', 1)
    text = text.replace('            handles.push(handle);', '''            #[cfg(all(target_os = "none", infinity_native))]
            let handle = join_handle.thread().id();
            handles.push(handle);''', 1)
    path.write_text(text)
    source = 'git+https://github.com/servo/stylo?rev=' + revision + '#' + revision
    lock = root / "build/servo-port-audit/Cargo.lock"
    locked = lock.read_text()
    original = subprocess.check_output(["git", "-C", str(lock.parent), "show", "HEAD:Cargo.lock"], text=True)
    packages = tomllib.loads(original)["package"]
    names = {p["name"] for p in packages if p.get("source") == source}
    patches = {}
    for manifest in destination.rglob("Cargo.toml"):
        package = tomllib.loads(manifest.read_text()).get("package", {})
        if package.get("name") in names:
            patches[package["name"]] = str(manifest.parent)
    if set(patches) != names:
        raise SystemExit("Incomplete Stylo workspace mapping")
    lock.write_text(locked.replace('source = "' + source + '"\n', ''))
    (destination / "native-patches.json").write_text(json.dumps(patches))

if __name__ == "__main__":
    main()
