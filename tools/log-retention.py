#!/usr/bin/env python3
"""Prune only provably valueless InfinityOS logs within declared boundaries."""
from pathlib import Path
import os
import stat

PROJECT_ROOT = Path(__file__).resolve().parents[1]


# ------------------------=
# FUNC: active_stream_files
# DESC: Protects redirected build streams even before the first byte has been written.
# ------------------=
def active_stream_files():
    identities = set()
    for descriptor in (0, 1, 2):
        try:
            metadata = os.fstat(descriptor)
            if stat.S_ISREG(metadata.st_mode):
                identities.add((metadata.st_dev, metadata.st_ino))
        except OSError:
            pass
    return identities


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
    active = active_stream_files()
    releases = root / "builds"
    if policy.get("remove_empty_release_logs", False) and releases.exists():
        if releases.is_symlink():
            raise ValueError(f"Refusing symlinked releases directory: {releases}")
        for path in releases.rglob("*.log"):
            if releases / ".studio-work" in path.parents:
                continue
            if not path.is_file() or path.is_symlink():
                continue
            metadata = path.stat()
            if metadata.st_size == 0 and (metadata.st_dev, metadata.st_ino) not in active:
                found.append((path, "empty-release-log"))
    for pattern in policy.get("root_crash_globs", ()):
        if "/" in pattern or "\\" in pattern or pattern in ("*", ".*"):
            raise ValueError(f"Unsafe root crash pattern: {pattern}")
        for path in root.glob(pattern):
            if path.parent != root or not path.is_file() or path.is_symlink():
                continue
            metadata = path.stat()
            if (metadata.st_dev, metadata.st_ino) not in active:
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
        if reason == "empty-release-log" and size != 0:
            continue
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
