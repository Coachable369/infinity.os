"""Behavioral fencing of saved native approval graph reuse."""
import copy
import unittest
from unittest.mock import patch
import ms10_installed_closure_setup as setup


# ------------------------=
# FUNC: receipt
# DESC: Constructs four directed independent durable approval rows.
# ------------------=
def receipt():
    identities = [f"{value:064x}" for value in range(1, 5)]
    grants = [{peer: {"metadata": (1 << 63) + 1, "publication": (1 << 63) + 2,
                      **({"replica": [(1 << 63) + i for i in range(3, 9)]} if peer == identities[0] else {})}
               for peer in identities if peer != local} for local in identities]
    return {"schema": 1, "stage": "configuration-submitted-before-sessions",
            "artifact_sha256": "ab" * 32, "identities": identities, "grants": grants}


class Prepared(unittest.TestCase):
    # ------------------------=
    # FUNC: test_exact_graph_and_generation_fence
    # DESC: A changed installed executable, identity, missing edge or ephemeral handle cannot authorize reuse.
    # ------------------=
    def test_exact_graph_and_generation_fence(self):
        value = receipt()
        self.assertEqual(setup.validate_prepared_receipt(value, value["identities"], "ab" * 32), value["grants"])
        for change in ("artifact", "identity", "edge", "handle"):
            invalid = copy.deepcopy(value)
            if change == "artifact": invalid["artifact_sha256"] = "cd" * 32
            if change == "identity": invalid["identities"][0] = "ff" * 32
            if change == "edge": invalid["grants"][0].pop(value["identities"][1])
            if change == "handle": invalid["grants"][0][value["identities"][1]]["metadata"] = 7
            with self.subTest(change=change), self.assertRaises(AssertionError):
                setup.validate_prepared_receipt(invalid, value["identities"], "ab" * 32)

    # ------------------------=
    # FUNC: test_checkpoint_precedes_session_failure
    # DESC: Completed issuance/configuration survives the first later session failure; no publication has run yet.
    # ------------------=
    def test_checkpoint_precedes_session_failure(self):
        value = receipt()
        saved = []
        class Distribution:
            # ------------------------=
            # FUNC: identity
            # DESC: Maps the fixture guest to its stable node identity.
            # ------------------=
            @staticmethod
            def identity(guest): return value["identities"][guest]
            # ------------------------=
            # FUNC: open_session
            # DESC: Stops at the first post-checkpoint transition.
            # ------------------=
            @staticmethod
            def open_session(a, b): raise RuntimeError("fixture interruption")
        with patch.object(setup, "grant_node", side_effect=lambda g, ids: value["grants"][g]), \
             patch.object(setup, "configure_node") as configured, patch.object(setup, "publish_node") as published:
            with self.assertRaises(RuntimeError):
                setup.establish_authority(list(range(4)), Distribution,
                                          checkpoint=lambda ids, grants: saved.append((ids, grants)))
        self.assertEqual(configured.call_count, 4)
        self.assertEqual(saved, [(value["identities"], value["grants"])])
        published.assert_not_called()
