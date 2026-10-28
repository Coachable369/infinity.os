"""Behavioral regression for exact live and installed UI data gates."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import subprocess
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ui_payload_parity", ROOT / "tools/ui-payload-parity.py")
parity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parity)


class UIPayloadParityTests(unittest.TestCase):
    # ------------------------=
    # FUNC: setUpClass
    # DESC: Builds the production-decoder harness under the active build-kit process before parity tests run.
    # ------------------=
    @classmethod
    def setUpClass(cls):
        subprocess.run(["make", "build/behavior-tests/ui-payload-validator"], cwd=ROOT, check=True)

    # ------------------------=
    # FUNC: setUp
    # DESC: Creates distinct binary payloads and kernel fixtures in repository-local scratch space.
    # ------------------=
    def setUp(self):
        scratch = ROOT / "build/tmp"
        scratch.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="ui-payload-parity-", dir=scratch)
        self.root = Path(self.temporary.name)
        self.payloads = []
        for name in ("installer", "configuration", "settings", "boot", "login", "desktop"):
            path = self.root / f"assets/boot/{name}-screens.iuit"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((ROOT / f"assets/boot/{name}-screens.iuit").read_bytes())
            self.payloads.append(path)
        for name in ("tokens", "styles"):
            path = self.root / f"assets/ui-design-kit/default/uikit-{name}.bin"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((ROOT / f"assets/ui-design-kit/default/uikit-{name}.bin").read_bytes())
            self.payloads.append(path)
        self.live, self.installed = self.root / "live.elf", self.root / "installed.elf"
        self.live.write_bytes(bytes(128) + b"".join(p.read_bytes() for p in self.payloads))
        self.installed.write_bytes(bytes(128) + b"".join(p.read_bytes() for p in self.payloads[1:]))

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only the test-owned scratch directory.
    # ------------------=
    def tearDown(self):
        self.temporary.cleanup()

    # ------------------------=
    # FUNC: test_live_and_installed_accept_exact_current_payloads
    # DESC: Accepts an installed kernel without installer-only screens when its required payloads match exactly.
    # ------------------=
    def test_live_and_installed_accept_exact_current_payloads(self):
        parity.verify_kernel_pair(self.live, self.installed, self.root)

    # ------------------------=
    # FUNC: test_each_changed_source_payload_rejects_stale_kernel
    # DESC: Mutates each screen, palette and style independently and rejects stale embedded bytes.
    # ------------------=
    def test_each_changed_source_payload_rejects_stale_kernel(self):
        for path in self.payloads:
            with self.subTest(payload=path.name):
                original = path.read_bytes()
                path.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
                with self.assertRaises(RuntimeError):
                    parity.verify_kernel_pair(self.live, self.installed, self.root)
                path.write_bytes(original)

    # ------------------------=
    # FUNC: test_each_installed_payload_is_independently_required
    # DESC: Rejects an installed kernel missing any required screen or theme even when live boot is current.
    # ------------------=
    def test_each_installed_payload_is_independently_required(self):
        for missing in self.payloads[1:]:
            with self.subTest(payload=missing.name):
                self.installed.write_bytes(bytes(128) + b"".join(p.read_bytes() for p in self.payloads[1:] if p != missing))
                with self.assertRaises(RuntimeError):
                    parity.verify_kernel_pair(self.live, self.installed, self.root)

    # ------------------------=
    # FUNC: test_unknown_or_truncated_formats_are_rejected
    # DESC: Rejects unrecognized and truncated payloads even if the identical bytes occur in the kernel.
    # ------------------=
    def test_unknown_or_truncated_formats_are_rejected(self):
        for data in (b"IUIT", b"IUIT\x07\x00\x01\x00", b"IUIKIT01" + bytes(55),
                     b"IUIKIT01" + bytes(56), b"IUISTY01" + bytes(75), b"IUISTY01" + bytes(76),
                     b"IUISTY01\x01\x00\x06\x00" + b"\x04" + bytes(71),
                     b"UNKNOWN0" + bytes(76)):
            with self.subTest(data=data[:8]):
                name = "configuration-screens.iuit" if data[:4] == b"IUIT" else (
                    "uikit-tokens.bin" if data[:8] == b"IUIKIT01" else "uikit-styles.bin")
                path = self.root / name
                path.write_bytes(data)
                self.live.write_bytes(bytes(128) + data)
                with self.assertRaises(RuntimeError):
                    parity.verify_embedded_ui(self.live, [path])

    # ------------------------=
    # FUNC: test_source_replacement_during_decode_is_rejected
    # DESC: Runs the actual decoder on captured bytes while replacing the source, then rejects the concurrent edit.
    # ------------------=
    def test_source_replacement_during_decode_is_rejected(self):
        path = self.payloads[-2]
        original = path.read_bytes()
        run = subprocess.run
        decoded = []

        # ------------------------=
        # FUNC: replace_source
        # DESC: Replaces mutable source after capture and records the real native decoder result for the captured snapshot.
        # ------------------=
        def replace_source(arguments, **kwargs):
            path.write_bytes(b"invalid concurrent replacement")
            self.assertEqual(kwargs["input"], original)
            result = run(arguments, **kwargs)
            decoded.append(result.returncode)
            return result

        with mock.patch.object(parity.subprocess, "run", side_effect=replace_source):
            with self.assertRaises(RuntimeError):
                parity.verify_embedded_ui(self.live, [path])
        self.assertEqual(decoded, [0])

    # ------------------------=
    # FUNC: test_valid_payload_cannot_impersonate_another_artifact_role
    # DESC: Rejects valid palette, style and screen bytes placed under a different artifact role even when already embedded.
    # ------------------=
    def test_valid_payload_cannot_impersonate_another_artifact_role(self):
        for target, substitute in ((0, 3), (1, 3), (2, 4), (3, 1), (4, 3)):
            path = self.payloads[target]
            original = path.read_bytes()
            with self.subTest(payload=path.name):
                path.write_bytes(self.payloads[substitute].read_bytes())
                with self.assertRaises(RuntimeError):
                    parity.verify_kernel_pair(self.live, self.installed, self.root)
                path.write_bytes(original)

    # ------------------------=
    # FUNC: test_earlier_source_change_during_installed_validation_is_rejected
    # DESC: Changes an installer-only input during the final installed decoder and rejects the otherwise valid pair.
    # ------------------=
    def test_earlier_source_change_during_installed_validation_is_rejected(self):
        path = self.payloads[0]
        original = path.read_bytes()
        run = subprocess.run
        decoded = []

        # ------------------------=
        # FUNC: replace_earlier_source
        # DESC: Lets real decoders validate all nine records, then replaces the live-only source during the last invocation.
        # ------------------=
        def replace_earlier_source(arguments, **kwargs):
            result = run(arguments, **kwargs)
            decoded.append(result.returncode)
            if len(decoded) == 2 * len(self.payloads) - 1:
                path.write_bytes(original + b"concurrent edit")
            return result

        with mock.patch.object(parity.subprocess, "run", side_effect=replace_earlier_source):
            with self.assertRaises(RuntimeError):
                parity.verify_kernel_pair(self.live, self.installed, self.root)
        self.assertEqual(decoded, [0] * (2 * len(self.payloads) - 1))


if __name__ == "__main__":
    unittest.main()
