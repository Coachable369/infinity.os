#!/usr/bin/env python3
"""Manage disposable InfinityOS build state strictly inside the repository."""
import argparse
from pathlib import Path
import shutil

PROJECT_ROOT = Path(__file__).resolve().parents[1]
MANAGED_DIRECTORIES = (
    "build",
    "target",
    "kernel/runtime/http/target",
    "tools/behavior-harness/target",
    "tools/installer-designer/.build",
    "tools/milestone9-harness/target",
    "tools/nic-probe/target",
    "tools/wire-trust-probe/target",
)


# ------------------------=
# FUNC: validate_root
# DESC: Rejects cleanup outside an InfinityOS repository or through a symlinked root.
# ------------------=
def validate_root(root):
    candidate = Path(root)
    if candidate.is_symlink():
        raise ValueError(f"Refusing symlinked workspace: {candidate}")
    root = candidate.resolve()
    if not (root / "Cargo.toml").is_file() or not (root / "Makefile").is_file():
        raise ValueError(f"Refusing unmanaged workspace: {root}")
    return root


# ------------------------=
# FUNC: managed_paths
# DESC: Resolves the exact disposable build directories without accepting caller paths.
# ------------------=
def managed_paths(root):
    root = validate_root(root)
    paths = tuple(root / relative for relative in MANAGED_DIRECTORIES)
    if any(path == root or root not in path.parents for path in paths):
        raise ValueError("Managed build path escaped the project root")
    return paths


# ------------------------=
# FUNC: clean
# DESC: Removes prior build outputs and caches, then creates the repository-local scratch root.
# ------------------=
def clean(root=PROJECT_ROOT):
    root = validate_root(root)
    removed = []
    for path in managed_paths(root):
        if path.is_symlink():
            raise ValueError(f"Refusing symlinked build path: {path}")
        if path.exists():
            removed.append({"path": path.relative_to(root).as_posix(), "bytes": directory_size(path)})
            shutil.rmtree(path)
    for source_root in (root / "boot", root / "kernel", root / "sdk", root / "tools"):
        for cache in tuple(source_root.rglob("__pycache__")) if source_root.exists() else ():
            if cache.is_symlink():
                raise ValueError(f"Refusing symlinked cache path: {cache}")
            removed.append({"path": cache.relative_to(root).as_posix(), "bytes": directory_size(cache)})
            shutil.rmtree(cache)
    (root / "build/tmp").mkdir(parents=True)
    return removed


# ------------------------=
# FUNC: directory_size
# DESC: Measures regular files in one managed tree without following symbolic links.
# ------------------=
def directory_size(path):
    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file() and not item.is_symlink())


# ------------------------=
# FUNC: main
# DESC: Exposes the bounded clean operation used before every complete build.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("clean",))
    args = parser.parse_args()
    if args.action == "clean":
        removed = clean()
        reclaimed = sum(entry["bytes"] for entry in removed)
        print(f"Clean build workspace ready: removed {len(removed)} trees, {reclaimed} bytes")


if __name__ == "__main__":
    main()
