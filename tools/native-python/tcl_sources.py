"""Pinned upstream Tcl/Tk sources for the native platform port."""
from pathlib import Path
import hashlib
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
VERSION = "8.6.18"
SOURCES = {
    "tcl": (
        "https://github.com/tcltk/tcl/releases/download/core-8-6-18/tcl8.6.18-src.tar.gz",
        "14f9af32b1767ff718477a8f974ad03c34341097e6b43f4ce54644ee974e268e",
        "tcl8.6.18"),
    "tk": (
        "https://codeload.github.com/tcltk/tk/tar.gz/6dbe68819aac4a22f6f7a143133ce26c5e8d2b05",
        "f968d5c3b3e16ede93d4cd8fbb0ba552f2ba8cdc4a2942d68528f8036ec127c6",
        "tk-6dbe68819aac4a22f6f7a143133ce26c5e8d2b05"),
}


# ------------------------=
# FUNC: sources
# DESC: Retrieves exact official Tcl/Tk archives into repository-owned storage and rejects mismatched bytes before extraction.
# ------------------=
def sources():
    # Dependency retrieval only: this function neither compiles nor writes to
    # build/builds, so it may run while another build owns those directories.
    parent = ROOT / "third_party/python/tcl-tk"
    parent.mkdir(parents=True, exist_ok=True)
    result = {}
    for component, (url, digest, directory) in SOURCES.items():
        archive = parent / (component + "-" + VERSION + ".tar.gz")
        if not archive.exists():
            partial = archive.with_suffix(".partial")
            with urllib.request.urlopen(url, timeout=60) as response:
                data = response.read(32 * 1024 * 1024 + 1)
            if len(data) > 32 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != digest:
                raise ValueError("Invalid " + component + " archive")
            partial.write_bytes(data)
            partial.replace(archive)
        if hashlib.sha256(archive.read_bytes()).hexdigest() != digest:
            raise ValueError("Modified " + component + " archive")
        destination = parent / directory
        if not destination.exists():
            with tarfile.open(archive) as bundle:
                bundle.extractall(parent, filter="data")
        result[component] = destination
    return result


if __name__ == "__main__":
    for name, path in sources().items():
        print(name + ": " + str(path))
