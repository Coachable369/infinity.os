"""Behavioral coverage for native recipient inventory acceptance orchestration."""
import copy
import types
import unittest
from unittest.mock import Mock, patch
import ms10_installed_reclamation as reclaim


# ------------------------=
# FUNC: states
# DESC: Creates distinct typed pre-retirement and retired physical identity observations.
# ------------------=
def states():
    ids = ["11" * 16, "22" * 16]
    before = {"catalog_present": True, "bindings": [{"version": 2, "manifest_generation": 3,
              "retired": False, "backing": ids[0], "extent": ids[1]}], "physical_objects": []}
    after = {"catalog_present": True, "bindings": [{"version": 2, "manifest_generation": 3,
             "retired": True, "backing": "00" * 16, "extent": None}],
             "physical_objects": [{"id": i, "present": False} for i in ids]}
    item = {"node": 2, "object": "aa" * 16, "before": before, "ids": ids, "covered": True}
    return item, after


class ReclamationTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_absence_requires_exact_retained_versions_and_physical_ids
    # DESC: Rejects empty, incomplete, stale and still-present evidence instead of counting failed reads as deletion.
    # ------------------=
    def test_absence_requires_exact_retained_versions_and_physical_ids(self):
        item, after = states()
        reclaim.validate_retired(item, after)
        failures = []
        for key, value in (("catalog_present", False), ("bindings", []), ("physical_objects", [])):
            bad = copy.deepcopy(after)
            bad[key] = value
            failures.append(bad)
        bad = copy.deepcopy(after)
        bad["bindings"][0]["manifest_generation"] += 1
        failures.append(bad)
        bad = copy.deepcopy(after)
        bad["physical_objects"][0]["present"] = True
        failures.append(bad)
        for bad in failures:
            with self.assertRaises(AssertionError):
                reclaim.validate_retired(item, bad)
        with self.assertRaises(AssertionError):
            reclaim.validate_retired(dict(item, covered=False), after)

    # ------------------------=
    # FUNC: test_optional_missing_copy_is_uncovered_not_success
    # DESC: Missing copy replicas are explicitly excluded while required original recipient inventory must exist.
    # ------------------=
    def test_optional_missing_copy_is_uncovered_not_success(self):
        guest = types.SimpleNamespace(number=2)
        empty = {"catalog_present": True, "bindings": [], "physical_objects": []}
        with patch.object(reclaim, "observe", return_value=empty):
            observed = reclaim.capture([guest], None, "owner", "object", False)
            self.assertFalse(observed[0]["covered"])
            self.assertEqual(observed[0]["ids"], [])
            with self.assertRaises(AssertionError):
                reclaim.capture([guest], None, "owner", "object", True)

    # ------------------------=
    # FUNC: test_finish_rechecks_same_ids_after_cold_boot
    # DESC: Uses real orchestration with typed observations to prove both post-delete and cold-start checks occur.
    # ------------------=
    def test_finish_rechecks_same_ids_after_cold_boot(self):
        item, after = states()
        guest = types.SimpleNamespace(number=2, installer=False, state=Mock(return_value=7),
                                      cold_boot_proof=Mock(return_value={"cold": True}))
        distribution = types.SimpleNamespace(identity=lambda _: "recipient")
        with patch.object(reclaim, "observe", return_value=after) as observe:
            result = reclaim.finish([guest], distribution, "verifier", "owner", [[item]])
        guest.cold_boot_proof.assert_called_once_with(7)
        self.assertEqual(observe.call_count, 2)
        self.assertEqual(observe.call_args_list[0].args[4], item["ids"])
        self.assertEqual(observe.call_args_list[1].args[4], item["ids"])
        self.assertEqual(result[0]["objects"][0]["cold"], after)


if __name__ == "__main__":
    unittest.main()
