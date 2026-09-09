"""Native selection behavior and framebuffer geometry tests."""
import struct
import unittest
from ms10_installed_pool_parity import select_object, selection_points


class Selection(unittest.TestCase):
    # ------------------------=
    # FUNC: test_three_objects_are_selected_by_identity
    # DESC: A real action callback advances the binary selected row; reading the target without selection does not pass.
    # ------------------=
    def test_three_objects_are_selected_by_identity(self):
        main = [0] * 512
        main[4] = main[8] = 8
        main[11:13] = [2048, 2048]
        pool = [0] * 256
        pool[18], pool[24] = 3, 1
        pool[246:250] = [200, 200, 1400, 1000]
        for index in range(3):
            pool[32+index*16:34+index*16] = struct.unpack("<2Q", bytes([index+1])*16)
        class Guest:
            # ------------------------=
            # FUNC: state
            # DESC: Returns real-shape framebuffer and Settings mode fields.
            # ------------------=
            def state(self): return main
            # ------------------------=
            # FUNC: launch
            # DESC: Represents the documented reset of expansion and scroll on launch.
            # ------------------=
            def launch(self, *args): pass
        actions = []
        # ------------------------=
        # FUNC: click
        # DESC: Applies only the exact computed native action point to the selected row.
        # ------------------=
        def click(guest, point):
            actions.append(point)
            if point == selection_points(main, pool)[1]:
                pool[19] = (pool[19]+1) % pool[18]
        result = select_object(Guest(), None, "03"*16, read=lambda: pool.copy(), click=click)
        self.assertEqual(result[19], 2)
        self.assertEqual(len(actions), 4)
        pool[249] = 400
        with self.assertRaises(AssertionError): selection_points(main, pool)
