#!/usr/bin/env python3
"""Check startup wait routing without launching a VM or accepting guest prose."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("installed", ROOT / "tools/ms9-installed-acceptance.py")
INSTALLED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLED)


class StopAtWait(Exception):
    pass


class BootTimeoutTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_default_boot_budget
    # DESC: Preserves the existing default for callers that do not opt into the larger full-bundle allowance.
    # ------------------=
    def test_default_boot_budget(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "build/tmp", prefix="boot-timeout-") as work:
            guest = INSTALLED.Guest(Path(work), 1, "unused-firmware")
            self.assertEqual(guest.boot_timeout_seconds, 120)

    # ------------------------=
    # FUNC: test_boot_waits_preserve_state_requirements
    # DESC: Exercises production startup methods with both boot budgets and verifies their actual predicates and timeout arguments.
    # ------------------=
    def test_boot_waits_preserve_state_requirements(self):
        for seconds in (120, 300):
            for method, installed, mode in (("install", 0, 0), ("onboard", 1, 4),
                                             ("authenticate", 1, 9)):
                with self.subTest(seconds=seconds, operation=method):
                    guest = INSTALLED.Guest.__new__(INSTALLED.Guest)
                    guest.boot_timeout_seconds = seconds
                    guest.boot = Mock()
                    guest.wait = Mock(side_effect=StopAtWait)
                    with self.assertRaises(StopAtWait):
                        getattr(guest, method)()
                    predicate, _label, timeout = guest.wait.call_args.args
                    self.assertEqual(timeout, seconds)
                    ready = [0] * 512
                    ready[3], ready[4] = installed, mode
                    self.assertTrue(predicate(ready))
                    ready[3] = 1 - installed
                    self.assertFalse(predicate(ready))


if __name__ == "__main__":
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use ./build-kit run python3 tools/boot-timeout-test.py")
    unittest.main()
