"""Actual session restoration must not be mistaken for failed authentication."""
import importlib.util
import pathlib
import unittest

SPEC = importlib.util.spec_from_file_location("installed", pathlib.Path(__file__).with_name("ms9-installed-acceptance.py"))
API = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(API)


class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Starts at a real authentication-state fixture with a selectable restored surface.
    # ------------------=
    def __init__(self, restored):
        self.snapshot = [0] * 512
        self.snapshot[3], self.snapshot[4], self.snapshot[8] = 1, 9, 1
        self.restored, self.keys = restored, []

    # ------------------------=
    # FUNC: wait
    # DESC: Requires the predicate to accept the current structured transition, never the diagnostic label.
    # ------------------=
    def wait(self, predicate, label):
        assert predicate(self.snapshot)
        return tuple(self.snapshot)

    # ------------------------=
    # FUNC: text
    # DESC: Keeps credential entry outside the state-verification oracle.
    # ------------------=
    def text(self, value):
        pass

    # ------------------------=
    # FUNC: key
    # DESC: Models successful authentication restoring Settings and the ordinary Escape return to Desktop.
    # ------------------=
    def key(self, key):
        self.keys.append(key)
        if key == "ret":
            self.snapshot[4], self.snapshot[9] = self.restored, 3
        elif key == "esc":
            assert self.snapshot[9] & 3 == 3
            self.snapshot[4] = 5


class RestoredAuthentication(unittest.TestCase):
    # ------------------------=
    # FUNC: test_restored_settings_and_desktop_both_reach_authenticated_desktop
    # DESC: Proves restored app navigation remains explicit while an already restored Desktop needs no extra key.
    # ------------------=
    def test_restored_settings_and_desktop_both_reach_authenticated_desktop(self):
        for mode in (5, 8):
            guest = Guest(mode)
            state = API.Guest.authenticate(guest)
            self.assertEqual(state[4], 5)
            self.assertEqual(state[9] & 3, 3)
            self.assertEqual(guest.keys, ["ret"] if mode == 5 else ["ret", "esc"])
