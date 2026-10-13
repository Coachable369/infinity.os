"""Behavioral regression for the ARM kernel/PCI aperture collision."""
import struct
import unittest
from kernel_layout import validate


# ------------------------=
# FUNC: image
# DESC: Constructs an ELF program table with supplied physical load ranges.
# ------------------=
def image(ranges):
    data = bytearray(64 + 56 * len(ranges))
    data[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", data, 18, 183)
    struct.pack_into("<Q", data, 32, 64)
    struct.pack_into("<HH", data, 54, 56, len(ranges))
    for index, (start, size) in enumerate(ranges):
        struct.pack_into("<IIQQQQQQ", data, 64 + index * 56,
                         1, 6, 0, start, start, 0, size, 4096)
    return data


class LayoutTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_old_arm_layout_rejected
    # DESC: Reproduces the installed kernel extent crossing the PCI aperture despite ample total RAM.
    # ------------------=
    def test_old_arm_layout_rejected(self):
        with self.assertRaises(ValueError):
            validate(image([(0x403E3000, 0xA1E48940)]))

    # ------------------------=
    # FUNC: test_split_arena_preserves_hole
    # DESC: Accepts independent low kernel and high arena ranges without reserving the physical gap.
    # ------------------=
    def test_split_arena_preserves_hole(self):
        self.assertEqual(validate(image([(0x10000000, 0x72000000),
                                         (0x100000000, 0x60000000)])),
                         [(0x10000000, 0x82000000), (0x100000000, 0x160000000)])

    # ------------------------=
    # FUNC: test_aperture_boundaries
    # DESC: Checks the exact end-exclusive device boundary and a one-byte crossing.
    # ------------------=
    def test_aperture_boundaries(self):
        validate(image([(0xDFFFF000, 4096)]))
        for start, size in [(0xDFFFF000, 4097), (0xE0000000, 4096), (0xFFFFFFFF, 2)]:
            with self.assertRaises(ValueError):
                validate(image([(start, size)]))


if __name__ == "__main__":
    unittest.main()
