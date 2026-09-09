"""Behavioral coverage classification; no VM control or source-text oracles."""
import unittest
from ms10_installed_owner_lifecycle import return_coverage


class OwnerReturnCoverage(unittest.TestCase):
    # ------------------------=
    # FUNC: test_placement_repair_is_not_new_content
    # DESC: Prevents a placement-only advance from satisfying stale-content reconciliation acceptance.
    # ------------------=
    def test_placement_repair_is_not_new_content(self):
        result = return_coverage(1, 1, 1, 1, 2)
        self.assertTrue(result["original_owner_return"])
        self.assertTrue(result["newer_placement_generation_observed"])
        self.assertFalse(result["newer_content_existed_before_owner_return"])
        self.assertFalse(result["stale_content_version_reconciled"])

    # ------------------------=
    # FUNC: test_reconciliation_requires_current_remote_version
    # DESC: Requires both a genuinely newer remote version before return and the owner's observed adoption of that version.
    # ------------------=
    def test_reconciliation_requires_current_remote_version(self):
        self.assertFalse(return_coverage(1, 2, 1, 1, 3)["stale_content_version_reconciled"])
        self.assertTrue(return_coverage(1, 2, 2, 1, 3)["stale_content_version_reconciled"])


if __name__ == "__main__":
    unittest.main()
