"""Behavioral acceptance for source edits, native pixel replacement and safe reset."""
import importlib.util
import json
import os
from pathlib import Path
import struct
import tempfile
import unittest
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("uikit_compiler", ROOT / "tools/compile-uikit.py")
COMPILER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMPILER)

class UIKitDocumentTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_uploaded_bitmap_preserves_native_alpha_and_orientation
    # DESC: Verifies the packaged V4 bitmap carries exact straight-alpha bytes and can be decoded without losing translucent edges.
    # ------------------=
    def test_uploaded_bitmap_preserves_native_alpha_and_orientation(self):
        image = Image.new("RGBA", (2, 2))
        image.putdata([(77, 163, 219, 127), (3, 7, 11, 1), (0, 0, 255, 255), (100, 200, 50, 0)])
        bmp = COMPILER.native_bitmap(image)
        self.assertEqual(struct.unpack_from("<IiiHHI", bmp, 14), (108, 2, -2, 1, 32, 3))
        self.assertEqual(bmp[122:], image.tobytes("raw", "BGRA"))

    # ------------------------=
    # FUNC: setUp
    # DESC: Creates an isolated repository-local asset tree for observable artifact mutation checks.
    # ------------------=
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "build")
        self.root = Path(self.temporary.name)
        self.directory = self.root / "assets/ui-design-kit/overrides/baselines"
        self.directory.mkdir(parents=True)
        self.target = self.root / "assets/ui-design-kit/default/test.rgba"
        self.target.parent.mkdir(parents=True)
        self.target.write_bytes(bytes([10, 20, 30, 255]) * 8)
        self.image = self.directory.parent / "replacement.png"
        Image.new("RGBA", (2, 2), (200, 100, 50, 128)).save(self.image)
        self.override = dict(id="test", target=str(self.target.relative_to(self.root)),
            source=str(self.image.relative_to(self.root)), offset=8, width=2, height=2)
        self.catalog = dict(version=1, elements=[self.override.copy()])

    # ------------------------=
    # FUNC: tearDown
    # DESC: Removes only this isolated fixture directory after the test completes.
    # ------------------=
    def tearDown(self):
        self.temporary.cleanup()

    # ------------------------=
    # FUNC: test_color_round_trip
    # DESC: Confirms all edited ARGB values reach their exact binary runtime slots.
    # ------------------=
    def test_color_round_trip(self):
        colors = [0xff100000 + index * 123 for index in range(13)]
        data = COMPILER.token_bytes(dict(version=1, colors=colors))
        self.assertEqual(len(data), 64)
        self.assertEqual(struct.unpack("<HH13I", data[8:]), (1, 13, *colors))
        with self.assertRaises(ValueError):
            COMPILER.token_bytes(dict(version=1, colors=colors[:12]))
        with self.assertRaises(ValueError):
            COMPILER.token_bytes(dict(version=2, colors=colors))

    # ------------------------=
    # FUNC: test_surface_style_binary
    # DESC: Confirms materials, corners and optional ARGB overrides survive encoding and rejects invalid OS geometry.
    # ------------------=
    def test_surface_style_binary(self):
        defaults = COMPILER.style_bytes({})
        self.assertEqual(defaults, b"IUISTY01" + struct.pack("<HH", 1, 6) + bytes(72))
        styles = [dict(material=0, corners=0, radius=0) for _ in range(6)]
        styles[2] = dict(material=2, corners=2, radius=24, fill=0x80123456, border=0xffabcdef)
        data = COMPILER.style_bytes(dict(styles=styles))
        self.assertEqual(struct.unpack("<4B2I", data[36:48]), (2, 2, 24, 3, 0x80123456, 0xffabcdef))
        flat = styles.copy()
        flat[2] = dict(styles[2], material=3)
        data = COMPILER.style_bytes(dict(styles=flat))
        self.assertEqual(struct.unpack("<4B2I", data[36:48]), (3, 2, 24, 3, 0x80123456, 0xffabcdef))
        for patch in [dict(material=4), dict(corners=3), dict(radius=41), dict(fill=-1), dict(border=True)]:
            invalid = styles.copy()
            invalid[2] = dict(styles[2], **patch)
            with self.assertRaises(ValueError):
                COMPILER.style_bytes(dict(styles=invalid))
        for index in (0, 1):
            invalid = styles.copy()
            invalid[index] = dict(corners=2, radius=12)
            with self.assertRaises(ValueError):
                COMPILER.style_bytes(dict(styles=invalid))

    # ------------------------=
    # FUNC: test_exact_cell_pixels
    # DESC: Proves an uploaded cell changes only its declared atlas span and preserves native alpha.
    # ------------------=
    def test_exact_cell_pixels(self):
        before = self.target.read_bytes()
        prepared = COMPILER.prepare_overrides(self.root, dict(overrides=[self.override]), self.catalog)
        COMPILER.apply_cells(self.root, prepared)
        after = self.target.read_bytes()
        self.assertEqual(after[:8], before[:8])
        self.assertEqual(after[24:], before[24:])
        self.assertEqual(after[8:24], bytes([200, 100, 50, 128]) * 4)

    # ------------------------=
    # FUNC: test_rejects_untrusted_paths_and_cells
    # DESC: Prevents an imported document from writing outside assets or corrupting adjacent runtime state cells.
    # ------------------=
    def test_rejects_untrusted_paths_and_cells(self):
        for values in [dict(source="../secret.png"), dict(target="/etc/test.rgba"), dict(offset=12), dict(width=3)]:
            edited = dict(self.override, **values)
            with self.assertRaises(ValueError):
                COMPILER.prepare_overrides(self.root, dict(overrides=[edited]), self.catalog)
        with self.assertRaises(ValueError):
            COMPILER.prepare_overrides(self.root, dict(overrides=[self.override, self.override]), self.catalog)

    # ------------------------=
    # FUNC: test_baseline_restore_and_external_conflict
    # DESC: Checks saved uploads can restore their original pixels but cannot overwrite newer external user edits.
    # ------------------=
    def test_baseline_restore_and_external_conflict(self):
        target = self.target.with_suffix(".png")
        Image.new("RGBA", (3, 3), (12, 34, 56, 255)).save(target)
        original = target.read_bytes()
        baseline = self.directory / "original.image"
        baseline.write_bytes(original)
        modified = self.image.read_bytes()
        target.write_bytes(modified)
        record = dict(target=str(target.relative_to(self.root)), source=str(baseline.relative_to(self.root)),
            sha256=COMPILER.digest(original), appliedSHA256=COMPILER.digest(modified))
        registry = dict(version=1, files=[record])
        restored = COMPILER.baseline_restorations(self.root, registry, [])
        self.assertEqual(restored[record["target"]][2], original)
        COMPILER.write_changed(target, restored[record["target"]][2])
        self.assertEqual(target.read_bytes(), original)
        target.write_bytes(b"external edit")
        with self.assertRaises(ValueError):
            COMPILER.baseline_restorations(self.root, registry, [])
        self.assertEqual(target.read_bytes(), b"external edit")

if __name__ == "__main__":
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE"), "Use ./build-kit run"
    unittest.main()
