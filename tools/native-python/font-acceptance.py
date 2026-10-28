"""Real native FontKit user ABI, exercised through a separately loaded extension."""
import sys
import infinity_native_test as native
assert sys.platform == "infinityos"
for flags in range(4):
    for height in (8, 17, 28, 53, 96):
        width, actual_height, advance, ascent, descent, pixels = native.glyph(65, height, flags)
        assert actual_height == height and len(pixels) == width * height
        assert advance > 0 and ascent + descent == height
        assert any(0 < alpha < 255 for alpha in pixels)
        assert len(set(pixels)) > 2
assert native.glyph(0x2603, 28, 0) == -88
assert native.glyph(65, 28, 4) == -88
assert native.glyph(65, 97, 0) == -22
assert native.glyph_bad_addresses() == (-14, -14, -14, 1)
sys.exit(0)
