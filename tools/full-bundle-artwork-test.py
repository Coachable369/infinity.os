"""Behavioral tests for bounded final-ISO binary artwork matching."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("full_bundle", Path(__file__).with_name("full-bundle-iso-test.py"))
BUNDLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUNDLE)


class ArtworkSinkTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_all_binary_chunk_boundaries
    # DESC: Matches the same non-text atlas at every split, including a split between installed payload shards.
    # ------------------=
    def test_all_binary_chunk_boundaries(self):
        atlas = bytes(range(256)) + b"\0\xff\x80\0"
        image = b"before\0" + atlas + b"\xffafter"
        for boundary in range(len(image) + 1):
            with self.subTest(boundary=boundary):
                sink = BUNDLE.BinaryMatchSink(atlas)
                self.assertEqual(sink.write(image[:boundary]), boundary)
                self.assertEqual(sink.write(image[boundary:]), len(image) - boundary)
                sink.require_match()
                self.assertEqual(sink.header, image)
                self.assertEqual(sink.tail, b"")

    # ------------------------=
    # FUNC: test_bytewise_stream_and_repeated_prefix
    # DESC: Handles one-byte reads and false partial matches before the complete binary artwork.
    # ------------------=
    def test_bytewise_stream_and_repeated_prefix(self):
        atlas = b"\0\0\0\xff\0\x80"
        sink = BUNDLE.BinaryMatchSink(atlas)
        for value in b"\0" * 40 + atlas[:-1] + atlas:
            sink.write(bytes([value]))
            self.assertLess(len(sink.tail), len(atlas))
        sink.require_match()

    # ------------------------=
    # FUNC: test_missing_truncated_and_modified_artwork_fail
    # DESC: Rejects no atlas, a truncated atlas, and a one-byte stale variant using actual sink state.
    # ------------------=
    def test_missing_truncated_and_modified_artwork_fail(self):
        atlas = bytes(range(128))
        for content in (b"", b"absent", atlas[:-1], atlas[:63] + b"\xff" + atlas[64:]):
            with self.subTest(content=content):
                sink = BUNDLE.BinaryMatchSink(atlas)
                sink.write(content)
                with self.assertRaises(RuntimeError):
                    sink.require_match()

    # ------------------------=
    # FUNC: test_bounded_retention_and_single_byte_atlas
    # DESC: Bounds retained data independently of artifact length and handles the one-byte target edge case.
    # ------------------=
    def test_bounded_retention_and_single_byte_atlas(self):
        sink = BUNDLE.BinaryMatchSink(b"\xff" * 65)
        block = b"\0" * 1024
        for _ in range(256):
            sink.write(block)
            self.assertLessEqual(len(sink.tail), 64)
            self.assertLessEqual(len(sink.header), 65536)
        sink.write(b"\xff" * 65)
        sink.write(block)
        sink.require_match()
        self.assertEqual(sink.tail, b"")
        one = BUNDLE.BinaryMatchSink(b"\xff")
        one.write(block)
        self.assertEqual(one.tail, b"")
        one.write(b"\xff")
        one.require_match()

    # ------------------------=
    # FUNC: test_empty_artwork_rejected
    # DESC: Prevents an empty source asset from making every kernel pass parity.
    # ------------------=
    def test_empty_artwork_rejected(self):
        with self.assertRaises(ValueError):
            BUNDLE.BinaryMatchSink(b"")


if __name__ == "__main__":
    unittest.main()
