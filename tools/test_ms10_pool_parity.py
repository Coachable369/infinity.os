"""Behavioral binary-page parsing and Console/Settings parity assertions."""
import struct
import unittest
from ms10_installed_pool_parity import decode_manifest_projection, assert_row


class Parity(unittest.TestCase):
    # ------------------------=
    # FUNC: test_exact_identity_generation_and_protection
    # DESC: Decodes independent verified/offline placements and rejects any differing Settings field.
    # ------------------=
    def test_exact_identity_generation_and_protection(self):
        header = bytearray(128)
        header[:8] = b"INFPMF02"
        header[8:24] = bytes([17]) * 16
        struct.pack_into("<Q", header, 24, 2)
        struct.pack_into("<Q", header, 80, 9)
        header[40:72] = bytes([3]) * 32
        header[72] = 3
        placements = bytearray(1024)
        for index, state in enumerate((2, 2, 3)):
            offset = index * 128
            placements[offset] = state
            placements[offset+8:offset+40] = bytes([index+1]) * 32
            struct.pack_into("<Q", placements, offset+80, 2)
            placements[offset+88:offset+120] = header[40:72]
        result = decode_manifest_projection(header, placements)
        row = [*struct.unpack("<2Q", header[8:24]), 2, 9, 32768, 3, 2, 1]
        assert_row(result, row)
        for field in (0, 2, 3, 5, 6, 7):
            changed = row[:]
            changed[field] += 1
            with self.assertRaises(AssertionError):
                assert_row(result, changed)
        placements[128+88] ^= 1
        self.assertEqual(decode_manifest_projection(header, placements)["verified"], 1)
        with self.assertRaises(AssertionError):
            decode_manifest_projection(header, placements[:-1])


if __name__ == "__main__":
    unittest.main()
