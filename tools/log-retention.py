#!/usr/bin/env python3
"""Prune only provably valueless InfinityOS logs within declared boundaries."""
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]


# ------------------------=
# FUNC: validate_root
# DESC: Confines log cleanup to a recognizable, non-symlinked InfinityOS repository.
# ------------------=
def validate_root(root):
    candidate = Path(root)
    if candidate.is_symlink():
        raise ValueError(f"Refusing symlinked workspace: {candidate}")
    root = candidate.resolve()
    if not (root / "build-kit.toml").is_file() or not (root / "Makefile").is_file():
        raise ValueError(f"Refusing unmanaged workspace: {root}")
    return root


# ------------------------=
# FUNC: candidates
# DESC: Identifies only empty durable logs and configured root compiler crash dumps.
# ------------------=
def candidates(root=PROJECT_ROOT, policy=None):
    root = validate_root(root)
    policy = policy or {}
    found = []
    releases = root / "builds"
    if policy.get("remove_empty_release_logs", False) and releases.exists():
        if releases.is_symlink():
            raise ValueError(f"Refusing symlinked releases directory: {releases}")
        for path in releases.rglob("*.log"):
            if path.is_file() and not path.is_symlink() and path.stat().st_size == 0:
                found.append((path, "empty-release-log"))
    for pattern in policy.get("root_crash_globs", ()):
        if "/" in pattern or "\\" in pattern or pattern in ("*", ".*"):
            raise ValueError(f"Unsafe root crash pattern: {pattern}")
        for path in root.glob(pattern):
            if path.parent == root and path.is_file() and not path.is_symlink():
                found.append((path, "compiler-crash-dump"))
    return sorted(set(found), key=lambda entry: entry[0].as_posix())


# ------------------------=
# FUNC: prune
# DESC: Deletes the bounded candidate set and returns structured removal evidence.
# ------------------=
def prune(root=PROJECT_ROOT, policy=None):
    root = validate_root(root)
    removed = []
    for path, reason in candidates(root, policy):
        size = path.stat().st_size
        path.unlink()
        removed.append({
            "bytes": size,
            "path": path.relative_to(root).as_posix(),
            "reason": reason,
        })
    releases = root / "builds"
    if releases.exists() and not releases.is_symlink():
        directories = sorted(
            (path for path in releases.rglob("*") if path.is_dir() and not path.is_symlink()),
            key=lambda path: len(path.parts), reverse=True)
        for directory in directories:
            try:
                directory.rmdir()
            except OSError:
                pass
    return removed
