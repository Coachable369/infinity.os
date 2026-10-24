#!/usr/bin/env python3
"""Bounded Studio UI update: reuse an ISO and native caches, never rebuild models."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import struct
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CHUNK = 1024 * 1024
SHARD = re.compile(r"P([01])-(\d{3})\.BIN", re.IGNORECASE)
PUBLISHED_RESULT = None
CANCEL_FILE = None


# ------------------------=
# FUNC: cancellation_point
# DESC: Converts a private Studio cancellation request into the same guarded unwind as terminal interruption.
# ------------------=
def cancellation_point():
    if CANCEL_FILE is not None and CANCEL_FILE.exists():
        raise KeyboardInterrupt()


# ------------------------=
# FUNC: event
# DESC: Emits structured, line-delimited progress consumed by Studio without scraping human logs.
# ------------------=
def event(kind, **fields):
    print(json.dumps({"event": kind, **fields}), flush=True)


# ------------------------=
# FUNC: digest
# DESC: Hashes a file in bounded memory for publication and cache identity checks.
# ------------------=
def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        while block := source.read(CHUNK):
            cancellation_point()
            result.update(block)
    return result.hexdigest()


# ------------------------=
# FUNC: run
# DESC: Runs a bounded child under the already-active build authority and contained environment.
# ------------------=
def run(arguments, **kwargs):
    environment = kwargs.pop("env", os.environ.copy())
    environment["CARGO_NET_OFFLINE"] = "true"
    cancellation_point()
    process = subprocess.Popen([str(value) for value in arguments], cwd=ROOT,
                               env=environment, start_new_session=True, stdin=subprocess.DEVNULL, **kwargs)
    try:
        while True:
            cancellation_point()
            try:
                status = process.wait(timeout=0.2)
                break
            except subprocess.TimeoutExpired:
                pass
        if status:
            raise subprocess.CalledProcessError(status, arguments)
    except BaseException:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        raise


# ------------------------=
# FUNC: load_tool
# DESC: Reuses existing binary parity implementations instead of duplicating their model and ELF contracts.
# ------------------=
def load_tool(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "tools" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ------------------------=
# FUNC: checked_iso
# DESC: Restricts publication to an existing regular ISO inside the project, resolving symlinks before use.
# ------------------=
def checked_iso(value, root=ROOT):
    path = Path(value).resolve(strict=True)
    if not path.is_relative_to(root.resolve()) or not path.is_file() or path.suffix.lower() != ".iso":
        raise ValueError("Select an existing .iso inside the InfinityOS repository")
    if path.stat().st_size == 0:
        raise ValueError("The selected ISO is empty")
    return path


# ------------------------=
# FUNC: member_bytes
# DESC: Reads only small boot metadata; kernels and model files use streaming helpers instead.
# ------------------=
def member_bytes(image, member):
    return subprocess.check_output(["mtype", "-i", str(image), member], stderr=subprocess.DEVNULL)


# ------------------------=
# FUNC: stream_member
# DESC: Streams a FAT object to a sink and returns its full byte identity, rejecting unreadable objects.
# ------------------=
def stream_member(image, member, sink=None):
    process = subprocess.Popen(["mtype", "-i", str(image), member], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    result, length = hashlib.sha256(), 0
    try:
        while block := process.stdout.read(CHUNK):
            cancellation_point()
            result.update(block)
            length += len(block)
            if sink is not None:
                sink.write(block)
    except BaseException:
        if process.poll() is None:
            process.kill()
        process.wait()
        raise
    finally:
        process.stdout.close()
    if process.wait() != 0 or not length:
        raise RuntimeError(f"Unreadable or empty FAT object: {member}")
    return length, result.hexdigest()


# ------------------------=
# FUNC: payload_members
# DESC: Enumerates exact existing payload objects without shell wildcards or arbitrary deletion targets.
# ------------------=
def payload_members(image):
    output = subprocess.check_output(["mdir", "-b", "-i", str(image), "::/EFI/INFINITY/PAYLOAD/"], text=True)
    result = []
    for value in output.splitlines():
        name = value.rsplit("/", 1)[-1]
        if not name or name in (".", "..") or "/" in name or "\\" in name:
            continue
        result.append((name, "::/EFI/INFINITY/PAYLOAD/" + name))
    return sorted(result)


# ------------------------=
# FUNC: payload_inventory
# DESC: Captures exact installed payload identities so model preservation is both checked and recorded in the publication receipt.
# ------------------=
def payload_inventory(image):
    result = {}
    for _, member in payload_members(image):
        size, sha = stream_member(image, member)
        result[member] = {"bytes": size, "sha256": sha}
    return result


# ------------------------=
# FUNC: validate_shard_names
# DESC: Rejects missing or noncontiguous streamed install payloads before reconstructing an image.
# ------------------=
def validate_shard_names(members, slot):
    parts = sorted((int(match[2]), member) for name, member in members
                   if (match := SHARD.fullmatch(name)) and int(match[1]) == slot)
    if not parts or [index for index, _ in parts] != list(range(len(parts))):
        raise ValueError(f"Missing or discontinuous P{slot} install payload")
    return [member for _, member in parts]


# ------------------------=
# FUNC: reassemble
# DESC: Reconstructs one validated payload while rejecting short interior shards and oversized extents.
# ------------------=
def reassemble(image, members, target):
    with target.open("wb") as output:
        for index, member in enumerate(members):
            length, _ = stream_member(image, member, output)
            if length > 512 * CHUNK or (index + 1 < len(members) and length != 512 * CHUNK):
                raise ValueError("Invalid install payload shard extent")


# ------------------------=
# FUNC: require_architecture
# DESC: Checks the binary ELF machine and native load layout rather than trusting an ISO filename.
# ------------------=
def require_architecture(header, target):
    from kernel_layout import validate
    validate(header, "virtualbox")
    if struct.unpack_from("<H", header, 18)[0] != {"aarch64": 183, "x86_64": 62}[target]:
        raise ValueError("Selected ISO architecture does not match the requested target")


# ------------------------=
# FUNC: require_efi_application
# DESC: Validates an EFI PE32+ image structurally so a safely rebuilt cached loader does not prevent retrying an older ISO.
# ------------------=
def require_efi_application(binary, target):
    if len(binary) < 64 or binary[:2] != b"MZ":
        raise ValueError("Invalid EFI DOS header")
    pe = struct.unpack_from("<I", binary, 60)[0]
    if pe < 64 or pe + 24 > len(binary) or binary[pe:pe + 4] != b"PE\0\0":
        raise ValueError("Invalid EFI PE header")
    machine, count = struct.unpack_from("<HH", binary, pe + 4)
    optional_size, characteristics = struct.unpack_from("<HH", binary, pe + 20)
    optional, sections = pe + 24, pe + 24 + optional_size
    if machine != {"aarch64": 0xaa64, "x86_64": 0x8664}[target]:
        raise ValueError("EFI bootloader architecture does not match the selected target")
    if not 1 <= count <= 96 or optional_size < 112 or sections + count * 40 > len(binary) or not characteristics & 2:
        raise ValueError("Invalid EFI executable section table")
    if struct.unpack_from("<H", binary, optional)[0] != 0x20b or struct.unpack_from("<H", binary, optional + 68)[0] != 10:
        raise ValueError("Bootloader is not a PE32+ EFI application")
    entry = struct.unpack_from("<I", binary, optional + 16)[0]
    image_size, headers_size = struct.unpack_from("<II", binary, optional + 56)
    if not sections + count * 40 <= headers_size <= len(binary) or image_size < headers_size:
        raise ValueError("Invalid EFI image extents")
    executable_entry, ranges = False, []
    for index in range(count):
        section = sections + index * 40
        virtual_size, virtual, raw_size, raw = struct.unpack_from("<IIII", binary, section + 8)
        flags = struct.unpack_from("<I", binary, section + 36)[0]
        extent = max(virtual_size, raw_size)
        if virtual + extent > image_size or (raw_size and (raw < headers_size or raw + raw_size > len(binary))):
            raise ValueError("EFI section is outside its image")
        if extent:
            ranges.append((virtual, virtual + extent))
        executable_entry |= bool(flags & 0x20000000 and virtual <= entry < virtual + virtual_size)
    ranges.sort()
    if not executable_entry or any(left[1] > right[0] for left, right in zip(ranges, ranges[1:])):
        raise ValueError("Invalid EFI executable entry or overlapping sections")


# ------------------------=
# FUNC: verify_embedded_ui
# DESC: Proves compiled screen styles and saved element data are embedded byte-for-byte in the kernel that will consume them.
# ------------------=
def verify_embedded_ui(kernel, templates):
    load_tool("ui-payload-parity").verify_embedded_ui(kernel, templates, cancellation_point)


# ------------------------=
# FUNC: verify_file_map
# DESC: Requires every saved UI/template asset in the fresh-install FAT image to equal its current source bytes.
# ------------------=
def verify_file_map(image, files):
    for source, member in files:
        size, actual = stream_member(image, member)
        if size != source.stat().st_size or actual != digest(source):
            raise RuntimeError(f"Fresh-install asset parity failed: {source}")


# ------------------------=
# FUNC: cached_components
# DESC: Requires the already-built native bundle and prevents this UI operation from downloading or rebuilding dependencies.
# ------------------=
def cached_components(target):
    names = [f"build/voice-kokoro/{target}/private-native.o",
             f"build/servo-platform-probe/browser-private-{target}.o", "build/browser-mode"]
    names += [f"builds/toolchains/{target}/{name}" for name in
              ("python.elf", "infinity_native_test.so", "python-license.txt", "clang.elf", "lld.elf", "application-crt.o")]
    paths = [ROOT / name for name in names]
    missing = [str(path.relative_to(ROOT)) for path in paths if not path.is_file() or not path.stat().st_size]
    if missing:
        raise RuntimeError("A full package build is required first; missing cached components: " + ", ".join(missing))
    if (ROOT / "build/browser-mode").read_text().strip() != "1":
        raise RuntimeError("UI update requires an existing complete native-browser package build")
    return paths


# ------------------------=
# FUNC: verify_unchanged
# DESC: Rejects mutation of any protected native dependency or source ISO during the staged update.
# ------------------=
def verify_unchanged(expected):
    for path, sha in expected.items():
        if not path.is_file() or digest(path) != sha:
            raise RuntimeError(f"Protected artifact changed during UI update: {path}")


# ------------------------=
# FUNC: source_snapshot
# DESC: Freezes saved UI inputs and shared kernel source identity while allowing Studio to remain usable during packaging.
# ------------------=
def source_snapshot(include_assets=True):
    files = set()
    folders = ("kernel", "boot", "linker", "sdk", ".cargo", "tools")
    for folder in (("assets",) + folders if include_assets else folders):
        for directory, children, names in os.walk(ROOT / folder):
            children[:] = [name for name in children if name not in ("target", ".build", "build", ".git", "__pycache__")]
            for name in names:
                path = Path(directory) / name
                if path.is_file():
                    files.add(path)
    for name in ("Cargo.toml", "Cargo.lock", "Makefile", "build-kit", "build-kit.toml", "build.sh"):
        files.add(ROOT / name)
    return {path: digest(path) for path in sorted(files)}


# ------------------------=
# FUNC: authoring_snapshot
# DESC: Detects a Studio save during initial document compilation without treating regenerated runtime atlases as authored changes.
# ------------------=
def authoring_snapshot():
    files = set((ROOT / "assets").rglob("*.infinityui"))
    files.update(path for path in (ROOT / "assets/ui-design-kit/overrides").rglob("*") if path.is_file())
    # The baseline registry's applied hashes are compiler output; its immutable
    # original images are already covered by the override directory above.
    for name in ("uikit-catalog.json",):
        path = ROOT / "assets/ui-design-kit" / name
        if path.is_file():
            files.add(path)
    return {path: digest(path) for path in sorted(files)}


# ------------------------=
# FUNC: verify_initial_inputs
# DESC: Rejects authored or compiler edits throughout preflight and initial UI generation while permitting generated asset updates.
# ------------------=
def verify_initial_inputs(authored, sources):
    if authored != authoring_snapshot() or sources != source_snapshot(include_assets=False):
        raise RuntimeError("Saved UI documents or build sources changed during preparation. Your existing ISO is unchanged; save your edits and run Update ISO again.")


# ------------------------=
# FUNC: file_identity
# DESC: Detects replacement or modification across publication hashes without reading a multi-gigabyte ISO again.
# ------------------=
def file_identity(path):
    stat = path.stat()
    return stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns


# ------------------------=
# FUNC: artifact_receipt
# DESC: Records a nonempty artifact's byte identity while rejecting replacement or modification during hashing.
# ------------------=
def artifact_receipt(path):
    before = file_identity(path)
    if not path.is_file() or before[2] == 0:
        raise RuntimeError(f"Cannot record an empty or non-file artifact: {path}")
    sha = digest(path)
    if file_identity(path) != before:
        raise RuntimeError(f"Artifact changed while recording publication evidence: {path}")
    return {"bytes": before[2], "sha256": sha}


# ------------------------=
# FUNC: compiled_ui_inventory
# DESC: Records versioned header metadata for native-validated UI payloads without duplicating the native schema decoder.
# ------------------=
def compiled_ui_inventory(payloads):
    result = {}
    for path in payloads:
        before = file_identity(path)
        with path.open("rb") as source:
            header = source.read(12)
        if path.name in ("boot-screens.iuit", "installer-screens.iuit", "configuration-screens.iuit", "settings-screens.iuit"):
            magic, offset, count_key = b"IUIT", 4, "screens"
        elif path.name in ("uikit-tokens.bin", "uikit-styles.bin"):
            magic = b"IUIKIT01" if path.name == "uikit-tokens.bin" else b"IUISTY01"
            offset, count_key = 8, "entries"
        else:
            raise RuntimeError(f"Unknown compiled UI artifact: {path}")
        if len(header) < offset + 4 or not header.startswith(magic):
            raise RuntimeError(f"Invalid compiled UI artifact header: {path}")
        version, count = struct.unpack_from("<HH", header, offset)
        record = artifact_receipt(path)
        if file_identity(path) != before:
            raise RuntimeError(f"UI payload changed while recording publication evidence: {path}")
        result[path.relative_to(ROOT).as_posix()] = {**record, "format_version": version, count_key: count}
    return result


# ------------------------=
# FUNC: atomic_publish
# DESC: Publishes only a validated candidate, retaining the selected ISO on all pre-publication failures.
# ------------------=
def atomic_publish(candidate, destination, expected_original, validator, metadata=None, precommit=None):
    global PUBLISHED_RESULT
    validator(candidate)
    cancellation_point()
    original_identity = file_identity(destination)
    if digest(destination) != expected_original:
        raise RuntimeError("Selected ISO changed externally; it was not overwritten")
    candidate_identity = file_identity(candidate)
    checksum = digest(candidate)
    sidecar = candidate.with_suffix(".sha256")
    sidecar.write_text(f"{checksum}  {destination.name}\n")
    publications = [(sidecar, destination.with_suffix(".iso.sha256"))]
    index = destination.parent / "SHA256SUMS"
    if index.exists():
        lines = index.read_text().splitlines()
        retained = [line for line in lines if len(line.split(maxsplit=1)) != 2 or
                    Path(line.split(maxsplit=1)[1].removeprefix("*")).name != destination.name]
        pending_index = candidate.parent / "SHA256SUMS.pending"
        pending_index.write_text("\n".join([*retained, f"{checksum}  {destination.name}"]) + "\n")
        publications.append((pending_index, index))
    if metadata:
        manifest, result = metadata
        result["sha256"] = checksum
        pending_manifest = candidate.parent / "ui-update-manifest.pending"
        pending_manifest.write_text(json.dumps(result, indent=2) + "\n")
        publications.append((pending_manifest, manifest))
    rollback = []
    for number, (_, target) in enumerate(publications):
        if target.exists():
            backup = candidate.parent / f"publication-{number}.rollback"
            os.link(target, backup)
        else:
            backup = None
        rollback.append((target, backup))
    with candidate.open("rb") as source:
        os.fsync(source.fileno())
    if precommit is not None:
        precommit()
    cancellation_point()
    if file_identity(destination) != original_identity or file_identity(candidate) != candidate_identity:
        raise RuntimeError("ISO bytes changed during publication preparation; the selected ISO was not overwritten")
    mask = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGINT, signal.SIGTERM})
    try:
        for pending, target in publications:
            os.replace(pending, target)
        # Final commit point: no filesystem work can fail after this rename.
        os.replace(candidate, destination)
        PUBLISHED_RESULT = metadata[1] if metadata else {"status": "passed", "iso": str(destination), "sha256": checksum}
    except BaseException:
        for target, backup in rollback:
            if backup is None:
                target.unlink(missing_ok=True)
            else:
                os.replace(backup, target)
        raise
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, mask)
    return checksum


# ------------------------=
# FUNC: verify_boot_catalog
# DESC: Validates binary El Torito metadata and proves the UEFI entry boots the actual updated efi.img extent.
# ------------------=
def verify_boot_catalog(iso, efi):
    with iso.open("rb") as source:
        root = None
        catalog = None
        for sector in range(16, 80):
            source.seek(sector * 2048)
            descriptor = source.read(2048)
            if len(descriptor) != 2048 or descriptor[1:6] != b"CD001":
                raise ValueError("Invalid ISO volume descriptor")
            if descriptor[0] == 0 and descriptor[7:39].rstrip(b"\0 ") == b"EL TORITO SPECIFICATION":
                catalog = struct.unpack_from("<I", descriptor, 71)[0]
            if descriptor[0] == 1:
                root = struct.unpack_from("<I", descriptor, 158)[0], struct.unpack_from("<I", descriptor, 166)[0]
            if descriptor[0] == 255:
                break
        if root is None or not catalog or root[1] > 1024 * 1024:
            raise ValueError("ISO lacks a bounded root directory or El Torito catalog")
        source.seek(root[0] * 2048)
        directory = source.read(root[1])
        offset, extent = 0, None
        while offset < len(directory):
            length = directory[offset]
            if length == 0:
                offset = ((offset // 2048) + 1) * 2048
                continue
            entry = directory[offset:offset + length]
            if length < 34 or len(entry) != length:
                raise ValueError("Malformed ISO directory record")
            if entry[33:33 + entry[32]].upper().split(b";")[0] == b"EFI.IMG" and extent is None:
                extent = struct.unpack_from("<I", entry, 2)[0]
            offset += length
        source.seek(catalog * 2048)
        entries = source.read(2048)
        if len(entries) != 2048 or entries[0] != 1 or entries[30:32] != b"\x55\xaa" or sum(struct.unpack("<16H", entries[:32])) & 0xffff:
            raise ValueError("Invalid El Torito validation entry")
        platform, offset, matched = entries[1], 32, False
        while offset + 32 <= len(entries):
            entry = entries[offset:offset + 32]
            if entry[0] in (0x90, 0x91):
                platform = entry[1]
            elif entry[0] == 0x88 and platform == 0xef:
                image_extent = struct.unpack_from("<I", entry, 8)[0]
                # Zero is the established whole-image convention for large
                # no-emulation UEFI media; it is valid for our multi-GiB FAT.
                if entry[1] != 0 or image_extent != extent:
                    raise ValueError("UEFI boot catalog does not reference the updated EFI image")
                matched = True
            offset += 32
        if not matched:
            raise ValueError("ISO has no bootable UEFI image")
        source.seek(extent * 2048)
        with efi.open("rb") as expected:
            if source.read(65536) != expected.read(65536):
                raise ValueError("UEFI boot extent differs from the updated FAT image")


# ------------------------=
# FUNC: update_iso
# DESC: Reuses existing media and native dependencies, rebuilding only UI-dependent kernels and install metadata before atomic publication.
# ------------------=
def update_iso(iso, target):
    # Freeze documents and compiler/kernel inputs before multi-gigabyte media
    # checks. Generated UI artifacts intentionally join the snapshot only after
    # their compilation, because producing them is part of this operation.
    authored = authoring_snapshot()
    sources = source_snapshot(include_assets=False)
    boot = "BOOTAA64.EFI" if target == "aarch64" else "BOOTX64.EFI"
    triple = "aarch64-unknown-none-softfloat" if target == "aarch64" else "x86_64-unknown-none"
    cached = cached_components(target)
    boot_file = ROOT / "build" / target / boot
    if not boot_file.is_file():
        raise RuntimeError("Cached bootloader is missing; create a full package first")
    for executable in ("xorriso", "mtype", "mcopy", "mdir", "mdel", "mdeltree", "make", "cargo"):
        if not shutil.which(executable):
            raise RuntimeError(f"Required build tool is unavailable: {executable}")
    required = iso.stat().st_size * 4 + 8 * 1024**3
    if shutil.disk_usage(ROOT).free < required:
        raise RuntimeError(f"UI update requires {required // 1024**3} GiB of free staging space")
    event("progress", phase="Checking ISO and cached native components", progress=0.03)
    original = digest(iso)
    protected = {path: digest(path) for path in cached}
    verify_initial_inputs(authored, sources)
    freshness = load_tool("iso-kernel-freshness")
    bundle = load_tool("full-bundle-iso-test")
    temporary = ROOT / "build/tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="studio-ui-iso-", dir=temporary, ignore_cleanup_errors=True) as directory:
        work = Path(directory)
        live, installed = work / "efi.img", work / "installed-efi.img"
        run(["xorriso", "-osirrox", "on", "-indev", iso, "-extract", "/efi.img", live], stdout=subprocess.DEVNULL)
        require_efi_application(member_bytes(live, f"::/EFI/BOOT/{boot}"), target)
        require_efi_application(boot_file.read_bytes(), target)
        # Native model bundles embed live artwork/templates in their kernel.
        # Older loose-asset media use a different staging contract and must not
        # silently retain obsolete external images after a nominal UI update.
        for folder in ("INFINITYUI", "FONTS", "FONT-LICENSES", "APPLICATIONS"):
            probe = subprocess.run(["mdir", "-i", str(live), f"::/EFI/INFINITY/{folder}/"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if probe.returncode == 0:
                raise RuntimeError("Selected ISO uses a legacy loose-asset layout; create a current native package before using incremental UI updates")
        from kernel_layout import HeaderSink
        header = HeaderSink()
        stream_member(live, "::/EFI/INFINITY/KERNEL.ELF", header)
        require_architecture(header.header, target)
        members = payload_members(live)
        reassemble(live, validate_shard_names(members, 0), installed)
        validate_shard_names(members, 1)
        model_before = payload_inventory(installed)
        for model in bundle.MODELS:
            bundle.verify_model(installed, *model)
        verify_initial_inputs(authored, sources)
        event("progress", phase="Compiling saved UIKit and screen documents", progress=0.17)
        run(["make", "uikit-runtime", "installer-template-runtime", "build/behavior-tests/ui-payload-validator"])
        verify_initial_inputs(authored, sources)
        inputs = source_snapshot()
        event("progress", phase="Incrementally compiling the installed UI kernel", progress=0.25)
        frozen = [argument for path in cached for argument in ("-o", path.relative_to(ROOT))]
        # Cache policy is explicit: native browser, Python, compiler and speech objects
        # cannot trigger a heavy dependency rebuild from this narrow entrypoint.
        installed_esp = ROOT / f"build/{target}/installed-esp.img"
        if installed_esp.is_symlink() or (installed_esp.exists() and not installed_esp.is_file()):
            raise RuntimeError("Disposable installed ESP must be a regular build artifact")
        # Asset deletion need not advance any remaining input timestamp. Rebuild
        # only this disposable image so the shared recipe resets managed trees.
        installed_esp.unlink(missing_ok=True)
        run(["make", *frozen, f"build/{target}/{boot}", f"build/{target}/installed-kernel.elf", f"build/{target}/installed-esp.img"])
        require_efi_application(boot_file.read_bytes(), target)
        installed_kernel = ROOT / f"build/{target}/installed-kernel.elf"
        with installed_kernel.open("rb") as source:
            require_architecture(source.read(65536), target)
        fat = ROOT / ("build/installed-fat-aarch64" if target == "aarch64" else "build/installed-fat")
        files = [(path, "::/" + path.relative_to(fat).as_posix()) for path in sorted((fat / "EFI").rglob("*")) if path.is_file()]
        if any("/PAYLOAD/" in member.upper() for _, member in files):
            raise RuntimeError("Unexpected model payload in UI staging tree")
        for folder in ("InfinityUI", "Fonts", "FontLicenses", "Applications"):
            run(["mdeltree", "-i", installed, "::/EFI/InfinityOS/" + folder], stdout=subprocess.DEVNULL)
        run(["mcopy", "-o", "-s", "-i", installed, fat / "EFI", "::/"])
        verify_file_map(installed, files)
        verify_file_map(installed, [(boot_file, f"::/EFI/BOOT/{boot}"),
                                   (boot_file, "::/EFI/InfinityOS/infinity.efi")])
        saved_assets = {source: digest(source) for source, _ in files}
        model_after = payload_inventory(installed)
        if model_before != model_after:
            raise RuntimeError("Model payload changed during the UI-only update")
        event("progress", phase="Updating verified fresh-install payload", progress=0.43)
        env = {**os.environ, "CARGO_TARGET_DIR": str(ROOT / "build/behavior-harness")}
        run(["cargo", "build", "--offline", "--quiet", "--release", "--manifest-path", "tools/behavior-harness/Cargo.toml", "--bin", "qwen-pack"], env=env)
        payload = work / "payload"
        run([ROOT / "build/behavior-harness/release/qwen-pack", "install", installed, installed_kernel, payload, ROOT / "build/qwen/payload-manifest.rs"])
        # Retain the installed ESP through verification, then free its large staging extent.
        installed.unlink()
        event("progress", phase="Incrementally compiling the live UI kernel", progress=0.55)
        cache = ROOT / f"build/cargo-ui-update-{target}"
        env = {**os.environ, "RUSTC_BOOTSTRAP": "1", "CARGO_TARGET_DIR": str(cache)}
        run(["cargo", "build", "--offline", "--release", "-Z", "build-std=core", "--target", triple, "--features", "streamed-payload,native-browser"], env=env)
        live_kernel = work / "KERNEL.ELF"
        run(["/opt/homebrew/opt/lld/bin/ld.lld", "-nostdlib", "-static", "--strip-debug", "--undefined=infinity_browser_private_infinity_browser_run", "-T", f"linker/{target}.ld", "-o", live_kernel, cache / triple / "release/libinfinity_kernel.a", ROOT / f"build/{target}/qwen-math.o", ROOT / f"build/voice-kokoro/{target}/private-native.o", ROOT / f"build/servo-platform-probe/browser-private-{target}.o"])
        run([sys.executable, "tools/design-kit-parity.py", "--arch", target, "--kernel", live_kernel, "--kernel", installed_kernel])
        with live_kernel.open("rb") as source:
            require_architecture(source.read(65536), target)
        templates = [ROOT / f"assets/boot/{name}-screens.iuit" for name in ("installer", "configuration", "settings", "boot")]
        styles = [ROOT / f"assets/ui-design-kit/default/uikit-{name}.bin" for name in ("tokens", "styles")]
        verify_embedded_ui(live_kernel, [*templates, *styles])
        verify_embedded_ui(installed_kernel, [*templates[1:], *styles])
        compiled_ui = compiled_ui_inventory([*templates, *styles])
        kernel_files = {path: file_identity(path) for path in (live_kernel, installed_kernel)}
        kernels = {"live": artifact_receipt(live_kernel), "installed": artifact_receipt(installed_kernel)}
        for name, member in members:
            if SHARD.fullmatch(name):
                run(["mdel", "-i", live, member])
        run(["mcopy", "-o", "-i", live, live_kernel, "::/EFI/INFINITY/KERNEL.ELF"])
        run(["mcopy", "-o", "-i", live, boot_file, f"::/EFI/BOOT/{boot}"])
        verify_file_map(live, [(boot_file, f"::/EFI/BOOT/{boot}")])
        for path in sorted(payload.iterdir()):
            run(["mcopy", "-o", "-i", live, path, "::/EFI/INFINITY/PAYLOAD/" + path.name])
            path.unlink()
        freshness.compare_member(live, "::/EFI/INFINITY/KERNEL.ELF", live_kernel)
        freshness.compare_shards(live, installed_kernel)
        verify_unchanged(protected)
        event("progress", phase="Repacking existing ISO with its original boot layout", progress=0.74)
        candidate = work / f"InfinityOS-{target}.iso"
        run(["xorriso", "-indev", iso, "-outdev", candidate, "-boot_image", "any", "replay", "-map", live, "/efi.img", "-map", boot_file, f"/EFI/BOOT/{boot}"])
        # Final-media gates inspect the actual candidate, not only staging files.
        event("progress", phase="Verifying media kernels, model preservation and install parity", progress=0.88)
        run([sys.executable, "tools/iso-kernel-freshness.py", candidate, live_kernel, installed_kernel])
        published_boot = work / boot
        run(["xorriso", "-osirrox", "on", "-indev", candidate, "-extract", f"/EFI/BOOT/{boot}", published_boot], stdout=subprocess.DEVNULL)
        if digest(published_boot) != digest(boot_file):
            raise RuntimeError("Published ISO bootloader differs from the current loader")
        verify_boot_catalog(candidate, live)
        run([sys.executable, "tools/full-bundle-iso-test.py", candidate])
        run([sys.executable, "tools/audio-install-parity.py", candidate, target, installed_kernel])
        verify_unchanged(protected)
        verify_unchanged(saved_assets)
        if inputs != source_snapshot():
            raise RuntimeError("Saved UI or kernel inputs changed while the ISO was being updated. Your existing ISO is unchanged; save your edits and run Update ISO again.")
        manifest = ROOT / "builds/manifests" / ("ui-update-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ") + ".json")
        manifest.parent.mkdir(parents=True, exist_ok=True)
        result = {"status": "passed", "iso": str(iso), "target": target,
                  "manifest": str(manifest), "base_sha256": original, "mode": "incremental-ui-repack",
                  "native_dependencies_preserved": {str(path.relative_to(ROOT)): sha for path, sha in protected.items()},
                  "compiled_ui_payloads": compiled_ui,
                  "kernels": kernels,
                  "preserved_install_payloads": model_before,
                  "checks": ["binary-architecture", "efi-loader-architecture", "bootloader-copy-parity", "uefi-boot-catalog", "live-kernel-freshness", "installed-kernel-freshness", "compiled-screen-parity", "saved-ui-asset-parity", "native-model-hashes", "audio-install-parity", "original-iso-conflict-check"]}
        # ------------------------=
        # FUNC: verify_precommit_inputs
        # DESC: Rechecks source and protected artifact identity after the final large ISO hashes and immediately before publication.
        # ------------------=
        def verify_precommit_inputs():
            verify_unchanged(protected)
            verify_unchanged(saved_assets)
            if any(file_identity(path) != identity for path, identity in kernel_files.items()):
                raise RuntimeError("A verified kernel changed before publication; the existing ISO is unchanged")
            if inputs != source_snapshot():
                raise RuntimeError("Saved UI or build inputs changed during final verification; the existing ISO is unchanged")
        atomic_publish(candidate, iso, original, lambda _: None, (manifest, result), verify_precommit_inputs)
    event("progress", phase="Updated ISO published", progress=1.0)
    event("result", **result)
    return result


# ------------------------=
# FUNC: interrupted
# DESC: Converts cancellation into normal stack unwinding so private staging is removed without publishing.
# ------------------=
def interrupted(signum, frame):
    raise KeyboardInterrupt()


# ------------------------=
# FUNC: main
# DESC: Exposes the guarded Studio/build-kit update contract and structured errors without running broad tests.
# ------------------=
def main():
    global CANCEL_FILE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=("aarch64", "x86_64"))
    parser.add_argument("--iso", required=True)
    parser.add_argument("--cancel-file", help="Private Studio cancellation marker inside build/tmp")
    arguments = parser.parse_args()
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run this operation through ./build-kit update-iso")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        if arguments.cancel_file:
            CANCEL_FILE = Path(arguments.cancel_file).resolve()
            if not CANCEL_FILE.is_relative_to((ROOT / "build/tmp").resolve()):
                raise ValueError("Cancellation marker must be inside repository build/tmp")
        cancellation_point()
        update_iso(checked_iso(arguments.iso), arguments.target)
    except (Exception, KeyboardInterrupt) as error:
        if PUBLISHED_RESULT is not None:
            event("result", **PUBLISHED_RESULT)
            return 0
        event("result", status="failed", error=str(error) or "ISO update cancelled; the published ISO was not replaced")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
