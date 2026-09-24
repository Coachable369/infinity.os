"""Behavioral failure-preservation and completed-node receipt reuse checks."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

spec = importlib.util.spec_from_file_location("bootstrap", Path(__file__).with_name("ms10-installed-storage-bootstrap.py"))
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)

class ResumeTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_completed_receipt
    # DESC: Requires independent installed proof fields and preserves completed receipts without booting the guest.
    # ------------------=
    def test_completed_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            guest = SimpleNamespace(work=work, disk=work / "installed.raw")
            self.assertIsNone(bootstrap.prior_result(guest))
            with guest.disk.open("wb") as disk:
                disk.truncate(32 * 1024**3)
            receipt = {key: True for key in ("cold_boot_identity", "authenticated_desktop", "installer_detached",
                "replica_service_ready_after_cold_boot", "local_storage_iop")}
            receipt.update(node_id="01" * 32, resource_id="02" * 16, device_id="03" * 16)
            path = work / "recipient-result.json"
            path.write_text(json.dumps(receipt))
            self.assertEqual(bootstrap.prior_result(guest), receipt)
            receipt["installer_detached"] = False
            path.write_text(json.dumps(receipt))
            with self.assertRaises(AssertionError):
                bootstrap.prior_result(guest)

    # ------------------------=
    # FUNC: test_capture_failure_does_not_mask_original
    # DESC: Models ENOSPC during both the initial observation and error capture, retaining the original failure in a structured receipt.
    # ------------------=
    def test_capture_failure_does_not_mask_original(self):
        with tempfile.TemporaryDirectory() as directory:
            # ------------------------=
            # FUNC: unavailable
            # DESC: Reproduces unavailable diagnostic storage without writing to a real full disk.
            # ------------------=
            def unavailable():
                raise OSError(28, "capture unavailable")
            guest = SimpleNamespace(number=3, work=Path(directory), process=SimpleNamespace(poll=lambda: None), state=unavailable)
            failure = OSError(28, "first observation unavailable")
            with contextlib.redirect_stdout(io.StringIO()):
                bootstrap.capture_failure(guest, failure)
            result = json.loads((guest.work / "bootstrap-failure.json").read_text())
            self.assertEqual(result["failure_errno"], 28)
            self.assertEqual(result["node"], 3)
            self.assertEqual(result["diagnostic_errno"], 28)

if __name__ == "__main__":
    unittest.main()
