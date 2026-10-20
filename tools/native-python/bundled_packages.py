"""Pinned, architecture-neutral public packages for the native Python image."""
import hashlib
import json
import os
import tarfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOCK = ROOT / "sdk/python/packages.lock.json"


# ------------------------=
# FUNC: packages
# DESC: Reads the explicit release package lock, without resolving mutable latest versions.
# ------------------=
def packages():
    return json.loads(LOCK.read_text())["packages"]


# ------------------------=
# FUNC: archive
# DESC: Retrieves a pinned upstream artifact and rejects corrupt cached or downloaded content.
# ------------------=
def archive(package):
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    destination = ROOT / "third_party/python/packages" / package["filename"]
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.exists():
        with urllib.request.urlopen(package["url"], timeout=60) as response:
            data = response.read(16 * 1024 * 1024 + 1)
        if len(data) > 16 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != package["sha256"]:
            raise RuntimeError("Package artifact failed verification: " + package["name"])
        temporary = destination.with_suffix(destination.suffix + ".partial")
        temporary.write_bytes(data)
        temporary.replace(destination)
    if hashlib.sha256(destination.read_bytes()).hexdigest() != package["sha256"]:
        raise RuntimeError("Cached package failed verification: " + package["name"])
    return destination


# ------------------------=
# FUNC: package_files
# DESC: Reads portable wheel files or source-package library files and upstream metadata.
# ------------------=
def package_files(package):
    path = archive(package)
    if path.suffix == ".whl":
        if not path.name.endswith("none-any.whl"):
            raise RuntimeError("Host binary wheels cannot enter the native image")
        with zipfile.ZipFile(path) as bundle:
            return {entry.filename: bundle.read(entry) for entry in bundle.infolist() if not entry.is_dir()}
    files = {}
    with tarfile.open(path) as bundle:
        prefix = package["source_prefix"]
        for entry in bundle.getmembers():
            if not entry.isfile():
                continue
            relative = entry.name.split("/", 1)[-1]
            body = bundle.extractfile(entry).read()
            if relative.startswith(prefix):
                files[relative[len(prefix):]] = body
            elif relative == "PKG-INFO":
                files[package["name"] + ".dist-info/METADATA"] = body
            elif relative.lower().startswith(("license", "copying")):
                files[package["name"] + ".dist-info/licenses/" + relative] = body
    return files


# ------------------------=
# FUNC: freeze_packages
# DESC: Exposes real public modules, resources and distribution metadata rather than pip-private vendored copies.
# ------------------=
def freeze_packages(names, resources, distributions):
    for package in packages():
        metadata = {}
        for filename, body in package_files(package).items():
            if ".dist-info/" in filename:
                metadata[filename.split(".dist-info/", 1)[1]] = body
            elif filename.endswith(".py") and ".egg-info/" not in filename:
                name = filename[:-3].replace("/__init__", "").replace("/", ".")
                if name in names:
                    raise RuntimeError("Bundled module collision: " + name)
                names[name] = (filename, body)
            elif not filename.endswith((".c", ".h", ".pyc")) and ".egg-info/" not in filename:
                resources[filename] = body
        if "METADATA" not in metadata:
            raise RuntimeError("Missing upstream distribution metadata: " + package["name"])
        distributions[package["name"]] = metadata


# ------------------------=
# FUNC: native_sources
# DESC: Compiles MarkupSafe's genuine C accelerator for the selected native ABI, never a host wheel.
# ------------------=
def native_sources():
    package = next(p for p in packages() if p["name"] == "markupsafe")
    files = package_files(package)
    directory = ROOT / "build/native-python/package-sources/markupsafe"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "_speedups.c"
    path.write_bytes(files["markupsafe/_speedups.c"])
    return [path]


# ------------------------=
# FUNC: license_notices
# DESC: Retains upstream license texts for every redistributable bundled package.
# ------------------=
def license_notices():
    result = []
    for package in packages():
        licenses = [(name, body) for name, body in package_files(package).items()
                    if "/licenses/" in name or name.rsplit("/", 1)[-1].lower().startswith(("license", "copying"))]
        if not licenses:
            raise RuntimeError("Missing package license: " + package["name"])
        for name, body in licenses:
            result += [b"\n\n" + package["name"].encode() + b" / " + name.encode() + b":\n", body]
    return result
