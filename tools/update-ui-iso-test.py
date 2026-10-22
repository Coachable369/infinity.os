"""Behavioral acceptance for bounded ISO publication and FAT asset replacement."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import threading
import time
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ui_iso", ROOT / "tools/update-ui-iso.py")
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


class UpdateISOTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUpClass
    # DESC: Builds the real payload decoder through this test's active build-kit context.
    # ------------------=
    @classmethod
    def setUpClass(cls):
        updater.run(["make", "build/behavior-tests/ui-payload-validator"])

    # ------------------------=
    # FUNC: setUp
    # DESC: Allocates isolated test artifacts strictly under the repository build directory.
    # ------------------=
    def setUp(self):
        updater.PUBLISHED_RESULT = None
        updater.CANCEL_FILE = None
        (ROOT / "build/tmp").mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="ui-iso-test-", dir=ROOT / "build/tmp")
        self.work = Path(self.temporary.name)

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the private test fixture directory after each assertion.
    # ------------------=
    def tearDown(self):
        updater.CANCEL_FILE = None
        self.temporary.cleanup()

    # ------------------------=
    # FUNC: test_failed_verification_preserves_published_iso
    # DESC: Proves a rejecting parity gate cannot replace the user's current ISO or publish a checksum.
    # ------------------=
    def test_failed_verification_preserves_published_iso(self):
        original, candidate = self.work / "old.iso", self.work / "new.iso"
        original.write_bytes(b"current installer")
        candidate.write_bytes(b"invalid candidate")
        # ------------------------=
        # FUNC: reject
        # DESC: Simulates a binary parity failure before publication.
        # ------------------=
        def reject(path):
            raise ValueError()
        with self.assertRaises(ValueError):
            updater.atomic_publish(candidate, original, updater.digest(original), reject)
        self.assertEqual(original.read_bytes(), b"current installer")
        self.assertTrue(candidate.exists())
        self.assertFalse(original.with_suffix(".iso.sha256").exists())

    # ------------------------=
    # FUNC: test_external_modification_is_not_overwritten
    # DESC: Proves a concurrent user replacement invalidates the publication transaction.
    # ------------------=
    def test_external_modification_is_not_overwritten(self):
        original, candidate = self.work / "old.iso", self.work / "new.iso"
        original.write_bytes(b"current installer")
        expected = updater.digest(original)
        candidate.write_bytes(b"candidate")
        original.write_bytes(b"user replacement")
        with self.assertRaises(RuntimeError):
            updater.atomic_publish(candidate, original, expected, lambda _: None)
        self.assertEqual(original.read_bytes(), b"user replacement")

    # ------------------------=
    # FUNC: test_verified_publication_has_matching_checksum
    # DESC: Proves successful publication replaces the bytes and writes the corresponding release sidecar.
    # ------------------=
    def test_verified_publication_has_matching_checksum(self):
        original, candidate = self.work / "with spaces.iso", self.work / "candidate.iso"
        original.write_bytes(b"original")
        candidate.write_bytes(b"verified candidate")
        actual = updater.atomic_publish(candidate, original, updater.digest(original), lambda _: None)
        self.assertEqual(original.read_bytes(), b"verified candidate")
        self.assertEqual(actual, hashlib.sha256(original.read_bytes()).hexdigest())
        self.assertEqual(original.with_suffix(".iso.sha256").read_text().split()[0], actual)
        self.assertFalse(candidate.exists())

    # ------------------------=
    # FUNC: test_post_hash_source_mutation_cannot_publish_iso_or_metadata
    # DESC: Mutates a saved source during the final candidate hash and proves the precommit guard preserves all published artifacts.
    # ------------------=
    def test_post_hash_source_mutation_cannot_publish_iso_or_metadata(self):
        original, candidate = self.work / "original.iso", self.work / "candidate.iso"
        source = self.work / "screen.infinityui"
        original.write_bytes(b"original")
        candidate.write_bytes(b"candidate")
        source.write_bytes(b"saved screen")
        expected = updater.digest(original)
        inputs = {source: updater.digest(source)}
        sidecar = original.with_suffix(".iso.sha256")
        sidecar.write_bytes(b"published checksum")
        index = self.work / "SHA256SUMS"
        index.write_bytes(b"published index")
        manifest = self.work / "result.json"
        digest = updater.digest
        # ------------------------=
        # FUNC: edit_during_candidate_hash
        # DESC: Simulates a Studio save after earlier parity checks while the large candidate is being hashed.
        # ------------------=
        def edit_during_candidate_hash(path):
            value = digest(path)
            if path == candidate:
                source.write_bytes(b"newer saved screen")
            return value
        with mock.patch.object(updater, "digest", side_effect=edit_during_candidate_hash):
            with self.assertRaises(RuntimeError):
                updater.atomic_publish(candidate, original, expected, lambda _: None,
                    (manifest, {}), lambda: updater.verify_unchanged(inputs))
        self.assertEqual(original.read_bytes(), b"original")
        self.assertEqual(sidecar.read_bytes(), b"published checksum")
        self.assertEqual(index.read_bytes(), b"published index")
        self.assertFalse(manifest.exists())
        self.assertIsNone(updater.PUBLISHED_RESULT)

    # ------------------------=
    # FUNC: test_iso_replacement_during_final_hash_is_not_overwritten
    # DESC: Detects destination and candidate replacements after their hashes without publishing mismatched checksum metadata.
    # ------------------=
    def test_iso_replacement_during_final_hash_is_not_overwritten(self):
        for changed in ("original", "candidate"):
            with self.subTest(changed=changed):
                original, candidate = self.work / "original.iso", self.work / "candidate.iso"
                original.write_bytes(b"original")
                candidate.write_bytes(b"candidate")
                expected = updater.digest(original)
                digest = updater.digest
                # ------------------------=
                # FUNC: replace_during_candidate_hash
                # DESC: Replaces one media inode after checksum calculation to model an external writer.
                # ------------------=
                def replace_during_candidate_hash(path):
                    value = digest(path)
                    if path == candidate:
                        replacement = self.work / "replacement"
                        replacement.write_bytes(b"external media")
                        replacement.replace(original if changed == "original" else candidate)
                    return value
                with mock.patch.object(updater, "digest", side_effect=replace_during_candidate_hash):
                    with self.assertRaises(RuntimeError):
                        updater.atomic_publish(candidate, original, expected, lambda _: None)
                self.assertEqual(original.read_bytes(), b"external media" if changed == "original" else b"original")
                self.assertFalse(original.with_suffix(".iso.sha256").exists())
                self.assertIsNone(updater.PUBLISHED_RESULT)

    # ------------------------=
    # FUNC: test_metadata_failure_rolls_back_before_iso_commit
    # DESC: Injects a metadata rename failure and proves ISO, checksum and checksum index remain unchanged.
    # ------------------=
    def test_metadata_failure_rolls_back_before_iso_commit(self):
        original, candidate = self.work / "original.iso", self.work / "candidate.iso"
        original.write_bytes(b"original")
        candidate.write_bytes(b"candidate")
        sidecar = original.with_suffix(".iso.sha256")
        sidecar.write_bytes(b"old checksum")
        index = self.work / "SHA256SUMS"
        index.write_bytes(b"old index")
        manifest = self.work / "result.json"
        replace = updater.os.replace
        # ------------------------=
        # FUNC: fail_manifest
        # DESC: Simulates a publication filesystem error after checksum staging but before the ISO commit.
        # ------------------=
        def fail_manifest(source, target):
            if Path(target) == manifest:
                raise OSError()
            return replace(source, target)
        with mock.patch.object(updater.os, "replace", side_effect=fail_manifest):
            with self.assertRaises(OSError):
                updater.atomic_publish(candidate, original, updater.digest(original), lambda _: None, (manifest, {}))
        self.assertEqual(original.read_bytes(), b"original")
        self.assertEqual(sidecar.read_bytes(), b"old checksum")
        self.assertEqual(index.read_bytes(), b"old index")
        self.assertFalse(manifest.exists())

    # ------------------------=
    # FUNC: test_iso_commit_failure_restores_existing_receipt_and_release_metadata
    # DESC: Rejects the final ISO rename after all metadata has advanced and restores the previous publication as a unit.
    # ------------------=
    def test_iso_commit_failure_restores_existing_receipt_and_release_metadata(self):
        original, candidate = self.work / "original.iso", self.work / "candidate.iso"
        original.write_bytes(b"previous installer")
        candidate.write_bytes(b"new installer")
        sidecar, index, manifest = original.with_suffix(".iso.sha256"), self.work / "SHA256SUMS", self.work / "result.json"
        sidecar.write_bytes(b"previous checksum")
        index.write_bytes(b"previous index")
        manifest.write_bytes(b"previous receipt")
        replace = updater.os.replace
        # ------------------------=
        # FUNC: fail_iso_commit
        # DESC: Injects a filesystem error only at the final media replacement, leaving rollback operations available.
        # ------------------=
        def fail_iso_commit(source, target):
            if Path(target) == original:
                raise OSError()
            return replace(source, target)
        with mock.patch.object(updater.os, "replace", side_effect=fail_iso_commit):
            with self.assertRaises(OSError):
                updater.atomic_publish(candidate, original, updater.digest(original), lambda _: None,
                                       (manifest, {"compiled_ui_payloads": {"settings": {"format_version": 9}}}))
        self.assertEqual(original.read_bytes(), b"previous installer")
        self.assertEqual(candidate.read_bytes(), b"new installer")
        self.assertEqual(sidecar.read_bytes(), b"previous checksum")
        self.assertEqual(index.read_bytes(), b"previous index")
        self.assertEqual(manifest.read_bytes(), b"previous receipt")
        self.assertIsNone(updater.PUBLISHED_RESULT)

    # ------------------------=
    # FUNC: test_shard_discontinuities_and_architecture_are_rejected
    # DESC: Exercises structural streamed-payload and ELF machine validation without prose matching.
    # ------------------=
    def test_shard_discontinuities_and_architecture_are_rejected(self):
        members = [("P0-000.BIN", "a"), ("P0-001.BIN", "b"), ("P1-000.BIN", "c")]
        self.assertEqual(updater.validate_shard_names(members, 0), ["a", "b"])
        with self.assertRaises(ValueError):
            updater.validate_shard_names([members[1]], 0)
        header = bytearray(120)
        header[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<H", header, 18, 183)
        struct.pack_into("<Q", header, 32, 64)
        struct.pack_into("<HH", header, 54, 56, 1)
        struct.pack_into("<IIQQQQQQ", header, 64, 1, 5, 0, 0x10000000, 0x10000000, 120, 4096, 4096)
        updater.require_architecture(header, "aarch64")
        with self.assertRaises(ValueError):
            updater.require_architecture(header, "x86_64")

    # ------------------------=
    # FUNC: test_iso_paths_are_contained
    # DESC: Rejects repository escapes and invalid media while accepting paths with spaces.
    # ------------------=
    def test_iso_paths_are_contained(self):
        good = self.work / "selected image.iso"
        good.write_bytes(b"media")
        self.assertEqual(updater.checked_iso(good, self.work), good)
        with self.assertRaises(ValueError):
            updater.checked_iso(good, self.work / "child")
        empty = self.work / "empty.iso"
        empty.touch()
        with self.assertRaises(ValueError):
            updater.checked_iso(empty, self.work)

    # ------------------------=
    # FUNC: test_source_snapshot_detects_saved_edits_and_new_assets
    # DESC: Proves a Studio save or uploaded asset invalidates an in-flight publication's input identity.
    # ------------------=
    def test_source_snapshot_detects_saved_edits_and_new_assets(self):
        for name in ("Cargo.toml", "Cargo.lock", "Makefile", "build-kit", "build-kit.toml", "build.sh", "tools/browser-build.mk", "assets/screen.infinityui"):
            path = self.work / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"original")
        with mock.patch.object(updater, "ROOT", self.work):
            before = updater.source_snapshot()
            (self.work / "assets/screen.infinityui").write_bytes(b"user edit")
            self.assertNotEqual(before, updater.source_snapshot())
            (self.work / "assets/screen.infinityui").write_bytes(b"original")
            self.assertEqual(before, updater.source_snapshot())
            (self.work / "assets/new.png").write_bytes(b"new upload")
            self.assertNotEqual(before, updater.source_snapshot())
            (self.work / "assets/new.png").unlink()
            (self.work / "tools/browser-build.mk").write_bytes(b"changed packaging contract")
            self.assertNotEqual(before, updater.source_snapshot())

    # ------------------------=
    # FUNC: test_relinked_efi_loaders_remain_valid_but_wrong_or_truncated_images_fail
    # DESC: Models cancellation and retry with differing valid bootloader bytes while rejecting wrong machines and out-of-file sections.
    # ------------------=
    def test_relinked_efi_loaders_remain_valid_but_wrong_or_truncated_images_fail(self):
        for target, machine in (("aarch64", 0xaa64), ("x86_64", 0x8664)):
            loader = bytearray(1024)
            loader[:2] = b"MZ"
            struct.pack_into("<I", loader, 60, 128)
            loader[128:132] = b"PE\0\0"
            struct.pack_into("<HH", loader, 132, machine, 1)
            struct.pack_into("<HH", loader, 148, 112, 2)
            struct.pack_into("<H", loader, 152, 0x20b)
            struct.pack_into("<I", loader, 168, 0x1000)
            struct.pack_into("<II", loader, 208, 0x2000, 512)
            struct.pack_into("<H", loader, 220, 10)
            struct.pack_into("<IIII", loader, 272, 512, 0x1000, 512, 512)
            struct.pack_into("<I", loader, 300, 0x20000000)
            updater.require_efi_application(loader, target)
            pe = struct.unpack_from("<I", loader, 60)[0]
            loader[pe + 8] ^= 1  # COFF timestamp can differ after a relink.
            updater.require_efi_application(loader, target)
            with self.assertRaises(ValueError):
                updater.require_efi_application(loader, "aarch64" if target == "x86_64" else "x86_64")
            with self.assertRaises(ValueError):
                updater.require_efi_application(loader[:128], target)
            optional_size = struct.unpack_from("<H", loader, pe + 20)[0]
            section = pe + 24 + optional_size
            struct.pack_into("<I", loader, section + 20, len(loader) + 1)
            with self.assertRaises(ValueError):
                updater.require_efi_application(loader, target)

    # ------------------------=
    # FUNC: test_initial_compile_rejects_changed_documents_but_allows_runtime_outputs
    # DESC: Distinguishes authored edits and uploads from legitimate generated binary changes during the first compilation phase.
    # ------------------=
    def test_initial_compile_rejects_changed_documents_but_allows_runtime_outputs(self):
        document = self.work / "assets/boot/configuration-screens.infinityui"
        document.parent.mkdir(parents=True)
        document.write_bytes(b"authored")
        generated = document.with_suffix(".iuit")
        generated.write_bytes(b"old compiled")
        with mock.patch.object(updater, "ROOT", self.work):
            before = updater.authoring_snapshot()
            generated.write_bytes(b"updated compiled")
            self.assertEqual(before, updater.authoring_snapshot())
            document.write_bytes(b"concurrent save")
            self.assertNotEqual(before, updater.authoring_snapshot())

    # ------------------------=
    # FUNC: test_early_media_preflight_freezes_authored_and_compiler_inputs
    # DESC: Exercises updater entry to reject edits during the initial ISO hash before extraction or any compiler starts.
    # ------------------=
    def test_early_media_preflight_freezes_authored_and_compiler_inputs(self):
        for name in ("Cargo.toml", "Cargo.lock", "Makefile", "build-kit", "build-kit.toml", "build.sh",
                     "tools/compile-uikit.py", "assets/screen.infinityui", "build/aarch64/BOOTAA64.EFI"):
            path = self.work / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"original")
        iso = self.work / "original.iso"
        iso.write_bytes(b"installer")
        for name in ("assets/screen.infinityui", "tools/compile-uikit.py"):
            with self.subTest(changed=name):
                edited = self.work / name
                digest = updater.digest
                # ------------------------=
                # FUNC: edit_during_preflight_hash
                # DESC: Mutates one real input while the starting ISO identity is being computed.
                # ------------------=
                def edit_during_preflight_hash(path):
                    value = digest(path)
                    if path == iso:
                        edited.write_bytes(b"changed during preflight")
                    return value
                with mock.patch.object(updater, "ROOT", self.work), \
                     mock.patch.object(updater, "cached_components", return_value=[]), \
                     mock.patch.object(updater.shutil, "which", return_value="available"), \
                     mock.patch.object(updater.shutil, "disk_usage", return_value=mock.Mock(free=2**50)), \
                     mock.patch.object(updater, "digest", side_effect=edit_during_preflight_hash), \
                     mock.patch.object(updater, "run") as run:
                    with self.assertRaises(RuntimeError):
                        updater.update_iso(iso, "aarch64")
                    run.assert_not_called()
                self.assertEqual(iso.read_bytes(), b"installer")
                self.assertIsNone(updater.PUBLISHED_RESULT)
                edited.write_bytes(b"original")

    # ------------------------=
    # FUNC: test_compiled_screen_parity_covers_style_payload_bytes
    # DESC: Detects a stale style field inside an otherwise identical embedded versioned template.
    # ------------------=
    def test_compiled_screen_parity_covers_style_payload_bytes(self):
        template = self.work / "configuration-screens.iuit"
        payload = (ROOT / "assets/boot/configuration-screens.iuit").read_bytes()
        template.write_bytes(payload)
        kernel = self.work / "kernel.elf"
        kernel.write_bytes(bytes(128) + payload + bytes(128))
        updater.verify_embedded_ui(kernel, [template])
        changed = bytearray(payload)
        changed[-4] ^= 1
        template.write_bytes(changed)
        with self.assertRaises(RuntimeError):
            updater.verify_embedded_ui(kernel, [template])
        styles = self.work / "uikit-styles.bin"
        styles.write_bytes(b"IUISTY01" + struct.pack("<HH", 1, 6) + bytes(72))
        kernel.write_bytes(kernel.read_bytes() + styles.read_bytes())
        updater.verify_embedded_ui(kernel, [styles])
        styles.write_bytes(b"IUISTY01" + struct.pack("<HH", 1, 6) + bytes(71) + b"\x01")
        with self.assertRaises(RuntimeError):
            updater.verify_embedded_ui(kernel, [styles])

    # ------------------------=
    # FUNC: test_settings_receipt_records_native_validated_live_and_installed_payloads
    # DESC: Publishes exact current Settings and UIKit identities after proving both consuming kernels contain their native-valid bytes.
    # ------------------=
    def test_settings_receipt_records_native_validated_live_and_installed_payloads(self):
        paths = [ROOT / "assets/boot/settings-screens.iuit",
                 ROOT / "assets/ui-design-kit/default/uikit-tokens.bin",
                 ROOT / "assets/ui-design-kit/default/uikit-styles.bin"]
        live, installed = self.work / "live.elf", self.work / "installed.elf"
        payloads = [path.read_bytes() for path in paths]
        live.write_bytes(bytes(128) + b"".join(payloads) + bytes(128))
        installed.write_bytes(bytes(256) + b"".join(payloads) + bytes(256))
        updater.verify_embedded_ui(live, paths)
        updater.verify_embedded_ui(installed, paths)
        records = updater.compiled_ui_inventory(paths)
        for path, payload in zip(paths, payloads):
            record = records[path.relative_to(ROOT).as_posix()]
            self.assertEqual(record["bytes"], len(payload))
            self.assertEqual(record["sha256"], hashlib.sha256(payload).hexdigest())
            version, count = struct.unpack_from("<HH", payload, 4 if path.suffix == ".iuit" else 8)
            self.assertEqual(record["format_version"], version)
            self.assertEqual(record["screens" if path.suffix == ".iuit" else "entries"], count)
        original, candidate, manifest = self.work / "original.iso", self.work / "candidate.iso", self.work / "result.json"
        original.write_bytes(b"original")
        candidate.write_bytes(b"updated media")
        result = {"compiled_ui_payloads": records,
                  "kernels": {"live": updater.artifact_receipt(live), "installed": updater.artifact_receipt(installed)}}
        checksum = updater.atomic_publish(candidate, original, updater.digest(original), lambda _: None, (manifest, result))
        published = json.loads(manifest.read_bytes())
        self.assertEqual(published, result)
        self.assertEqual(published["sha256"], checksum)
        self.assertEqual(published["kernels"]["installed"]["sha256"], updater.digest(installed))
        self.assertEqual(updater.PUBLISHED_RESULT, published)
        stale = self.work / "stale.elf"
        stale.write_bytes(bytes(128) + b"".join(payloads[1:]))
        with self.assertRaises(RuntimeError):
            updater.verify_embedded_ui(stale, paths)

    # ------------------------=
    # FUNC: test_receipt_rejects_artifact_mutation_during_hashing
    # DESC: Prevents a receipt from combining a previous binary header with bytes saved concurrently by Studio.
    # ------------------=
    def test_receipt_rejects_artifact_mutation_during_hashing(self):
        path = self.work / "settings-screens.iuit"
        path.write_bytes(b"IUIT" + struct.pack("<HH", 9, 11) + bytes(64))
        digest = updater.digest
        # ------------------------=
        # FUNC: mutate_during_receipt
        # DESC: Changes the file after its hash was computed to exercise stat-identity protection.
        # ------------------=
        def mutate_during_receipt(source):
            result = digest(source)
            source.write_bytes(source.read_bytes() + b"changed")
            return result
        with mock.patch.object(updater, "digest", side_effect=mutate_during_receipt):
            with self.assertRaises(RuntimeError):
                updater.compiled_ui_inventory([path])

    # ------------------------=
    # FUNC: test_cancel_stops_owned_child_group_and_preserves_iso
    # DESC: Requests cancellation while a real child is running and proves both child cleanup and non-publication.
    # ------------------=
    def test_cancel_stops_owned_child_group_and_preserves_iso(self):
        marker = self.work / "cancel"
        ready = self.work / "ready"
        updater.CANCEL_FILE = marker
        # ------------------------=
        # FUNC: request_cancel
        # DESC: Cancels only after the owned fixture process has recorded its identity.
        # ------------------=
        def request_cancel():
            deadline = time.monotonic() + 10
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            marker.write_bytes(b"cancel")
        thread = threading.Thread(target=request_cancel)
        thread.start()
        code = "import os,pathlib,time; pathlib.Path(__import__('sys').argv[1]).write_text(str(os.getpid())); time.sleep(30)"
        try:
            with self.assertRaises(KeyboardInterrupt):
                updater.run([updater.sys.executable, "-c", code, ready])
        finally:
            thread.join(timeout=11)
        self.assertTrue(ready.exists())
        with self.assertRaises(ProcessLookupError):
            updater.os.kill(int(ready.read_text()), 0)
        original, candidate = self.work / "original.iso", self.work / "candidate.iso"
        original.write_bytes(b"original")
        candidate.write_bytes(b"candidate")
        with self.assertRaises(KeyboardInterrupt):
            updater.atomic_publish(candidate, original, "unused", lambda _: None)
        self.assertEqual(original.read_bytes(), b"original")
        self.assertFalse(original.with_suffix(".iso.sha256").exists())

    # ------------------------=
    # FUNC: test_fat_overlay_preserves_models_and_verifies_ui
    # DESC: Resets obsolete nested managed artwork and replaces real FAT UI bytes while retaining a native model object.
    # ------------------=
    def test_fat_overlay_preserves_models_and_verifies_ui(self):
        for tool in ("mformat", "mmd", "mcopy", "mtype", "mdir", "mdeltree"):
            self.assertIsNotNone(shutil.which(tool))
        image = self.work / "efi.img"
        with image.open("wb") as output:
            output.truncate(64 * 1024 * 1024)
        subprocess.run(["mformat", "-i", str(image), "::"], check=True)
        for directory in ("EFI", "EFI/INFINITY", "EFI/INFINITY/PAYLOAD", "EFI/InfinityOS", "EFI/InfinityOS/InfinityUI"):
            subprocess.run(["mmd", "-i", str(image), "::/" + directory], check=True)
        model = self.work / "model.bin"
        model.write_bytes(bytes(range(256)) * 17)
        subprocess.run(["mcopy", "-i", str(image), str(model), "::/EFI/INFINITY/PAYLOAD/P4-000.BIN"], check=True)
        before = updater.payload_inventory(image)
        self.assertEqual(before, {"::/EFI/INFINITY/PAYLOAD/P4-000.BIN":
                                 {"bytes": model.stat().st_size, "sha256": updater.digest(model)}})
        staging = updater.load_tool("iso-staging")
        tree = self.work / "build/installed-fat-aarch64"
        ui = tree / "EFI/InfinityOS/InfinityUI"
        obsolete = ["Installer/installer-assets/nested/obsolete.png", "DesignKit/Default/obsolete.rgba"]
        for relative in obsolete:
            source = ui / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_bytes(b"obsolete artwork")
        subprocess.run(["mcopy", "-o", "-s", "-i", str(image), str(tree / "EFI"), "::/"], check=True)
        with mock.patch.object(staging, "ROOT", self.work):
            staging.reset_ui_stage(tree)
        source = ui / "DesignKit/Default/uikit-tokens.bin"
        source.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "assets/ui-design-kit/default/uikit-tokens.bin", source)
        template = ui / "Installer/settings-screens.iuit"
        template.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "assets/boot/settings-screens.iuit", template)
        subprocess.run(["mdeltree", "-i", str(image), "::/EFI/InfinityOS/InfinityUI"], check=True)
        subprocess.run(["mcopy", "-o", "-s", "-i", str(image), str(tree / "EFI"), "::/"], check=True)
        files = [(path, "::/" + path.relative_to(tree).as_posix()) for path in tree.rglob("*") if path.is_file()]
        updater.verify_file_map(image, files)
        for relative in obsolete:
            self.assertFalse((ui / relative).exists())
            result = subprocess.run(["mtype", "-i", str(image), "::/EFI/InfinityOS/InfinityUI/" + relative],
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(updater.payload_inventory(image), before)
        self.assertEqual(len(updater.payload_members(image)), 1)
        model.write_bytes(b"changed model")
        subprocess.run(["mcopy", "-o", "-i", str(image), str(model), "::/EFI/INFINITY/PAYLOAD/P4-000.BIN"], check=True)
        self.assertNotEqual(updater.payload_inventory(image), before)
        source.write_bytes(b"changed source")
        with self.assertRaises(RuntimeError):
            updater.verify_file_map(image, files)

    # ------------------------=
    # FUNC: test_real_el_torito_catalog_targets_updated_fat
    # DESC: Creates a real UEFI ISO and proves binary validation rejects a catalog redirected away from its FAT image.
    # ------------------=
    def test_real_el_torito_catalog_targets_updated_fat(self):
        tree = self.work / "tree"
        tree.mkdir()
        image = tree / "efi.img"
        with image.open("wb") as output:
            output.truncate(16 * 1024 * 1024)
        subprocess.run(["mformat", "-i", str(image), "::"], check=True)
        iso = self.work / "candidate.iso"
        subprocess.run(["xorriso", "-as", "mkisofs", "-R", "-e", "efi.img", "-no-emul-boot", "-o", str(iso), str(tree)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        updater.verify_boot_catalog(iso, image)
        with iso.open("r+b") as output:
            output.seek(17 * 2048 + 71)
            catalog = struct.unpack("<I", output.read(4))[0]
            output.seek(catalog * 2048 + 38)
            output.write(struct.pack("<H", 0))
        updater.verify_boot_catalog(iso, image)
        with iso.open("r+b") as output:
            output.seek(catalog * 2048 + 40)
            output.write(struct.pack("<I", 1))
        with self.assertRaises(ValueError):
            updater.verify_boot_catalog(iso, image)


if __name__ == "__main__":
    unittest.main()
