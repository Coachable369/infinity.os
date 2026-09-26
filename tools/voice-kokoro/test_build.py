"""Behavioral build orchestration tests; not guest or installed acceptance."""
import os
import unittest
from unittest.mock import call, patch

import build


class BuildRoutingTests(unittest.TestCase):
    # ------------------------=
    # FUNC: invoke
    # DESC: Runs the actual CLI routing with only external build stages replaced by a recording seam.
    # ------------------=
    def invoke(self, *arguments):
        with patch.dict(os.environ, {"INFINITY_BUILD_KIT_ACTIVE": "1"}), \
             patch("sys.argv", ["build.py", *arguments]), \
             patch.object(build, "stage") as stage:
            build.main()
            return stage.call_args_list

    # ------------------------=
    # FUNC: test_x86_modes
    # DESC: Requires the real x86 probe and architecture-specific comparison, keeping deadline verification the default.
    # ------------------=
    def test_x86_modes(self):
        for mode in ("deadline", "correctness"):
            arguments = [] if mode == "deadline" else ["--x86-probe-mode", mode]
            calls = self.invoke("--target", "x86_64", *arguments)
            self.assertEqual(calls[-2:], [
                call("guest-x86", "tools/voice-kokoro/probe/run-x86.py", "--clock", "realtime", "--mode", mode),
                call("comparison", "tools/voice-kokoro/compare-reference.py", "--target", "x86_64")])

    # ------------------------=
    # FUNC: test_arm_and_build_only
    # DESC: Preserves the ARM verification route and skips execution only when build-only was requested.
    # ------------------=
    def test_arm_and_build_only(self):
        calls = self.invoke("--target", "aarch64")
        self.assertEqual(calls[-2:], [
            call("guest", "tools/voice-kokoro/probe/run.py"),
            call("comparison", "tools/voice-kokoro/compare-reference.py", "--target", "aarch64")])
        for target in ("aarch64", "x86_64"):
            calls = self.invoke("--target", target, "--build-only")
            self.assertEqual(len(calls), 5)
            self.assertEqual(calls[-1], call("link", "tools/voice-kokoro/link-native.py"))


if __name__ == "__main__":
    unittest.main()
