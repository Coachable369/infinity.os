#!/usr/bin/env python3
"""Prove a candidate ISO contains the kernels produced by its current build."""

from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import sys
from iso_fat import fat_image


# ------------------------=
# FUNC: compare_member
# DESC: Streams one FAT member and rejects any byte or length difference from the current build artifact.
# ------------------=
def compare_member(image: Path, member: str, expected: Path) -> None:
    process = subprocess.Popen(
        ["mtype", "-i", str(image), member],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    assert process.stdout is not None
    expected_digest = hashlib.sha256()
    actual_digest = hashlib.sha256()
    expected_size = 0
    actual_size = 0
    with expected.open("rb") as source:
        while True:
            block = source.read(1024 * 1024)
            if not block:
                break
            expected_digest.update(block)
            expected_size += len(block)
    while True:
        block = process.stdout.read(1024 * 1024)
        if not block:
            break
        actual_digest.update(block)
        actual_size += len(block)
    process.stdout.close()
    if process.wait() != 0:
        raise RuntimeError(f"missing ISO kernel member: {member}")
    if actual_size != expected_size or actual_digest.digest() != expected_digest.digest():
        raise RuntimeError(f"stale ISO kernel member: {member}")


# ------------------------=
# FUNC: compare_shards
# DESC: Streams the installed-kernel shards as one object and rejects divergence from the current installed ELF.
# ------------------=
def compare_shards(image: Path, expected: Path) -> None:
    actual_digest = hashlib.sha256()
    actual_size = 0
    part = 0
    while True:
        process = subprocess.Popen(
            ["mtype", "-i", str(image), f"::/EFI/INFINITY/PAYLOAD/P1-{part:03}.BIN"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        assert process.stdout is not None
        shard_size = 0
        while True:
            block = process.stdout.read(1024 * 1024)
            if not block:
                break
            actual_digest.update(block)
            actual_size += len(block)
            shard_size += len(block)
        process.stdout.close()
        present = process.wait() == 0
        if not present:
            break
        if shard_size == 0:
            raise RuntimeError(f"empty installed-kernel shard: {part}")
        part += 1
    if part == 0:
        raise RuntimeError("ISO has no installed-kernel shards")
    expected_digest = hashlib.sha256()
    expected_size = 0
    with expected.open("rb") as source:
        while True:
            block = source.read(1024 * 1024)
            if not block:
                break
            expected_digest.update(block)
            expected_size += len(block)
    if actual_size != expected_size or actual_digest.digest() != expected_digest.digest():
        raise RuntimeError("stale installed kernel in ISO payload")


# ------------------------=
# FUNC: verify_iso_kernels
# DESC: Reads the release EFI extent in place and proves live and fresh-install kernel bytes match this invocation.
# ------------------=
def verify_iso_kernels(iso: Path, live_kernel: Path, installed_kernel: Path) -> None:
    for artifact in (iso, live_kernel, installed_kernel):
        if not artifact.is_file() or artifact.stat().st_size == 0:
            raise RuntimeError(f"missing build artifact: {artifact}")
    efi = fat_image(iso)
    compare_member(efi, "::/EFI/INFINITY/KERNEL.ELF", live_kernel)
    compare_shards(efi, installed_kernel)


# ------------------------=
# FUNC: main
# DESC: Applies the mandatory live and installed kernel freshness gate to one candidate release.
# ------------------=
def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: iso-kernel-freshness.py <image.iso> <live-kernel.elf> <installed-kernel.elf>")
    verify_iso_kernels(*(Path(value).resolve() for value in sys.argv[1:]))
    print(f"PASS: ISO kernels are current for {sys.argv[1]}")


if __name__ == "__main__":
    main()
