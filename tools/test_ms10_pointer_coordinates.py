"""Behavioral title hit-point scaling across native framebuffer dimensions."""
import unittest
from ms10_installed_pointer import title_target


class PointerCoordinates(unittest.TestCase):
    # ------------------------=
    # FUNC: test_normalized_point_hits_title_at_each_scale
    # DESC: Maps normalized input back through the native framebuffer transform and requires the actual title region.
    # ------------------=
    def test_normalized_point_hits_title_at_each_scale(self):
        for width, height in ((2048, 2048), (1920, 1080), (2560, 1440), (3840, 2160)):
            scale = 2 if width >= 2560 and height >= 1440 else 1
            rect = (width//10, height//10, width*7//10, height*7//10)
            nx, ny = title_target(rect, width, height)
            px, py = width*nx//1000, height*ny//1000
            with self.subTest(width=width, height=height):
                self.assertTrue(0 <= nx <= 1000 and 0 <= ny <= 1000)
                self.assertLess(abs(px - (rect[0]+rect[2]//2)), width//1000+2)
                self.assertTrue(rect[1] <= py < rect[1]+54*scale)
        with self.assertRaises(AssertionError):
            title_target((2000, 0, 1000, 600), 2048, 2048)
