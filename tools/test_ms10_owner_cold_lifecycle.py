"""Behavioral ordering of cold Critical proofs before owner mutation/deletion."""
import types
import unittest
from unittest.mock import patch, Mock
import ms10_installed_owner_lifecycle as lifecycle


class Guest:
    # ------------------------=
    # FUNC: __init__
    # DESC: Gives each fake cold-boot participant a stable typed identity and detached media state.
    # ------------------=
    def __init__(self, number, events):
        self.number, self.events = number, events
        self.installer = False

    # ------------------------=
    # FUNC: state
    # DESC: Returns a stable identity fixture for the cold boot boundary.
    # ------------------=
    def state(self):
        return self.number

    # ------------------------=
    # FUNC: cold_boot_proof
    # DESC: Records a cold boot and returns structured identity proof.
    # ------------------=
    def cold_boot_proof(self, before):
        assert before == self.number
        self.events.append(("cold", self.number))
        return {"cold_boot_identity": True}

    # ------------------------=
    # FUNC: boot
    # DESC: Records the original owner's return after survivor proofs.
    # ------------------=
    def boot(self, installer):
        assert not installer
        self.events.append(("return", self.number))

    # ------------------------=
    # FUNC: authenticate
    # DESC: Supplies the ordinary authenticated boot boundary.
    # ------------------=
    def authenticate(self):
        pass

    # ------------------------=
    # FUNC: launch
    # DESC: Supplies normal Console activation without touching any VM.
    # ------------------=
    def launch(self, *_):
        pass


class ColdOrder(unittest.TestCase):
    # ------------------------=
    # FUNC: test_retirement_requires_tombstone_absence_and_drained_outbox
    # DESC: A tombstone alone is insufficient until actual object removal and persistent retirement acknowledgements are observed.
    # ------------------=
    def test_retirement_requires_tombstone_absence_and_drained_outbox(self):
        states = [
            {"tombstone": {"present": True, "deleted": True}, "object_present": True,
             "outbox": {"present": True, "pending": {"manifest_generation": 2, "acknowledged": 1, "placements": 3}}},
            {"tombstone": {"present": True, "deleted": True}, "object_present": False,
             "outbox": {"present": True, "pending": {"manifest_generation": 2, "acknowledged": 1, "placements": 3}}},
            {"tombstone": {"present": True, "deleted": True}, "object_present": False,
             "outbox": {"present": True, "pending": None}}]
        with patch.object(lifecycle, "inspect_lifecycle", side_effect=states) as inspect, patch.object(lifecycle.time, "sleep"):
            result = lifecycle.wait_retired(None, None, None, None, [])
        self.assertEqual(inspect.call_count, 3)
        self.assertIsNone(result["outbox"]["pending"])

    # ------------------------=
    # FUNC: test_all_critical_replicas_cold_before_owner_mutation
    # DESC: Executes the real continuation until the first post-return read and proves no mutation preceded survivor cold/hash/read verification.
    # ------------------=
    def test_all_critical_replicas_cold_before_owner_mutation(self):
        events = []
        guests = [Guest(i, events) for i in range(1, 5)]
        distribution = types.SimpleNamespace(identity=lambda g: g.number, open_session=lambda a, b: None,
                                             API=types.SimpleNamespace(symbol=None))
        # ------------------------=
        # FUNC: persisted
        # DESC: Records full-byte native verification as a typed fixture result.
        # ------------------=
        def persisted(guest, *_):
            events.append(("hash", guest.number))
            return {"verified": True, "bytes": 32768}
        distribution.persisted_hash = persisted
        # ------------------------=
        # FUNC: read
        # DESC: Records authorized survivor reads and stops precisely at the original owner's first read.
        # ------------------=
        def read(guest, *_args, **_kwargs):
            if guest.number == 1:
                raise StopIteration()
            events.append(("read", guest.number))
            return {"version": 1, "readiness_attempts": []}
        report = {"identities": [1, 2, 3, 4], "created": {"object_id": "ab" * 16, "version": 1},
                  "length": 32768, "seed": 17, "namespace_path": "/Shared/Case"}
        mutate = Mock()
        with patch.object(lifecycle, "read_path", read), patch.object(lifecycle, "read_path_ready", read), patch.object(lifecycle, "invoke", mutate):
            with self.assertRaises(StopIteration):
                lifecycle.run(guests, distribution, None, report)
        mutate.assert_not_called()
        self.assertEqual(events, [("cold", 2), ("hash", 2), ("read", 2), ("read", 2),
                                  ("cold", 3), ("hash", 3), ("read", 3), ("read", 3),
                                  ("cold", 4), ("hash", 4), ("return", 1)])
        self.assertEqual(len(report["critical_cold_proof"]), 3)


if __name__ == "__main__":
    unittest.main()
