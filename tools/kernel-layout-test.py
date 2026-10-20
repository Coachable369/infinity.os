"""Behavioral regression for the ARM kernel/PCI aperture collision."""
import struct
import unittest
from kernel_layout import validate


# ------------------------=
# FUNC: image
# DESC: Constructs an ELF program table with supplied physical load ranges.
# ------------------=
def image(ranges, machine=183):
    data = bytearray(64 + 56 * len(ranges))
    data[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", data, 18, machine)
    struct.pack_into("<Q", data, 32, 64)
    struct.pack_into("<HH", data, 54, 56, len(ranges))
    for index, (start, size) in enumerate(ranges):
        struct.pack_into("<IIQQQQQQ", data, 64 + index * 56,
                         1, 6, 0, start, start, 0, size, 4096)
    return data


class LayoutTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_x86_high_arenas
    # DESC: Preserves complete speech and browser arenas while keeping ordinary globals within relative reach.
    # ------------------=
    def test_x86_high_arenas(self):
        ranges = [(0x4000000, 0x70000000), (0x100000000, 0x60000000),
                  (0x160000000, 0x20000000)]
        self.assertEqual(validate(image(ranges, 62)),
                         [(start, start + size) for start, size in ranges])
        for invalid in ([(0x4000000, 0x90000000)],
                        [(0x160000000, 0x20001000)],
                        [(0x4000000, 8192), (0x4001000, 4096)]):
            with self.assertRaises(ValueError):
                validate(image(invalid, 62))
        for field, value in ((68, 7), (96, 4096)):
            header = image([(0x160000000, 0x20000000)], 62)
            struct.pack_into("<I", header, field, value)
            with self.assertRaises(ValueError):
                validate(header)

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

    # ------------------------=
    # FUNC: test_qemu_map_is_explicit
    # DESC: Allows QEMU RAM through the VirtualBox device hole only for an explicitly selected QEMU kernel.
    # ------------------=
    def test_qemu_map_is_explicit(self):
        header = image([(0x50000000, 0xA0000000), (0x100000000, 0x60000000)])
        self.assertEqual(validate(header, "qemu"),
                         [(0x50000000, 0xF0000000), (0x100000000, 0x160000000)])
        with self.assertRaises(ValueError):
            validate(header)

    # ------------------------=
    # FUNC: test_qemu_rejects_wrong_base_or_ram_extent
    # DESC: Rejects canonical VirtualBox kernels, out-of-RAM segments and unrelated ELF targets in QEMU media.
    # ------------------=
    def test_qemu_rejects_wrong_base_or_ram_extent(self):
        for ranges in ([(0x10000000, 4096)], [(0x50000000, 4096), (0x33FFFF000, 4097)]):
            with self.assertRaises(ValueError):
                validate(image(ranges), "qemu")
        wrong_architecture = image([(0x50000000, 4096)])
        struct.pack_into("<H", wrong_architecture, 18, 62)
        with self.assertRaises(ValueError):
            validate(wrong_architecture, "qemu")


if __name__ == "__main__":
    unittest.main()
