"""Behavioral checks of command submission across a real-UI authentication boundary."""
import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location("installed", pathlib.Path(__file__).with_name("ms9-installed-acceptance.py"))
installed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installed)


class CommandContext(unittest.TestCase):
    # ------------------------=
    # FUNC: test_locked_operator_authenticates_before_submission
    # DESC: Exercises the actual submission method; a locked editor must never receive the command payload.
    # ------------------=
    def test_locked_operator_authenticates_before_submission(self):
        for mode in (9, 10, 5):
            guest = ControlledGuest(mode)
            result = installed.Guest.command(guest, "request")
            self.assertEqual(result[71], 1)
            self.assertEqual(guest.submissions, 1)
            self.assertEqual(guest.authentications, int(mode != 5))
            self.assertEqual(guest.launched, int(mode != 5))


class ControlledGuest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Provides a bounded stateful UI fixture, not an output-text oracle.
    # ------------------=
    def __init__(self, mode):
        self.values = [0] * 512
        self.values[3], self.values[4], self.values[71] = 1, mode, int(mode == 5)
        self.authentications = self.launched = self.submissions = 0

    # ------------------------=
    # FUNC: wait
    # DESC: Fails immediately when the implementation requests an impossible state transition.
    # ------------------=
    def wait(self, predicate, label):
        assert predicate(self.values)
        return self.values

    # ------------------------=
    # FUNC: authenticate
    # DESC: Models successful normal credential verification into the desktop only.
    # ------------------=
    def authenticate(self):
        assert self.values[4] in (9, 10)
        self.authentications += 1
        self.values[4] = 5

    # ------------------------=
    # FUNC: launch
    # DESC: Opens the command editor after successful authentication.
    # ------------------=
    def launch(self, query, mode):
        assert self.values[4] == 5
        self.launched += 1
        self.values[71] = 1

    # ------------------------=
    # FUNC: text
    # DESC: Rejects command delivery into any unauthenticated or occupied editor.
    # ------------------=
    def text(self, value):
        assert self.values[4] == 5 and self.values[71] == 1
        self.values[71] += len(value)

    # ------------------------=
    # FUNC: key
    # DESC: Commits exactly one prepared submission and clears the editor state.
    # ------------------=
    def key(self, code):
        assert self.values[4] == 5 and self.values[71] > 1
        self.submissions += 1
        self.values[71] = 1


if __name__ == "__main__":
    unittest.main()
