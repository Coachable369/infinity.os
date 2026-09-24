"""Exact byte-set fencing for preserved installed acceptance cases."""
import hashlib
import importlib.util
import pathlib
import unittest
import copy
import struct
import ms10_installed_fixture as fixture

SPEC = importlib.util.spec_from_file_location("owner_case", pathlib.Path(__file__).with_name("ms10-installed-owner-offline.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CaseFence(unittest.TestCase):
    # ------------------------=
    # FUNC: test_startup_resume_rejects_mutated_or_different_installed_sets
    # DESC: Resumes the selected unfinished input check, never a configured or authority-bearing run.
    # ------------------=
    def test_startup_resume_rejects_mutated_or_different_installed_sets(self):
        failed = {"stage": "input-precheck", "failure": True, "identities": [1, 2, 3, 4],
                  "artifact_sha256": "cd"*32, "length": 65536, "seed": 17, "input_precheck_node": 3}
        MODULE.validate_startup_resume(failed, [1, 2, 3, 4], "cd"*32, 65536, 17, 3)
        for field, value in (("created", {}), ("authority", []), ("stage", "network-configuration"),
                             ("artifact_sha256", "00"*32), ("input_precheck_node", 2)):
            invalid = dict(failed)
            invalid[field] = value
            with self.assertRaises(AssertionError):
                MODULE.validate_startup_resume(invalid, [1, 2, 3, 4], "cd"*32, 65536, 17, 3)

    # ------------------------=
    # FUNC: test_explicit_serial_boot_budget_observation
    # DESC: Extended readiness is opt-in serial-only and preserves evidence that measured login exceeded the original deadline.
    # ------------------=
    def test_explicit_serial_boot_budget_observation(self):
        MODULE.validate_boot_budget(True, 300)
        MODULE.validate_boot_budget(False, 120)
        with self.assertRaises(AssertionError): MODULE.validate_boot_budget(False, 300)
        with self.assertRaises(AssertionError): MODULE.validate_boot_budget(True, 600)
        class Guest:
            number = 1
            # ------------------------=
            # FUNC: wait
            # DESC: Checks the exact requested readiness budget and rejects uninitialized firmware state.
            # ------------------=
            def wait(self, predicate, label, timeout):
                assert timeout == 300
                value = [0]*512
                assert not predicate(value)
                value[3:5] = [1, 9]
                assert predicate(value)
                return value
        observation = MODULE.await_boot_readiness(Guest(), 300, 10, clock=lambda: 222)
        self.assertEqual(observation["observed_seconds"], 212)
        self.assertFalse(observation["within_original_120s"])

    # ------------------------=
    # FUNC: test_boot_resume_requires_successful_independent_baseline
    # DESC: A failed pre-configuration boot can resume from a prior size, but object mutations and altered artifact/identity cannot.
    # ------------------=
    def test_boot_resume_requires_successful_independent_baseline(self):
        failed = {"stage": "boot", "failure": "recorded", "identities": [1, 2, 3, 4],
                  "artifact_sha256": "cd"*32, "length": 65536, "seed": 17}
        baseline = {"stage": "measurement-complete-owner-loss-not-tested", "identities": [1, 2, 3, 4],
                    "artifact_sha256": "cd"*32, "persisted": [{}, {}, {}], "length": 32768, "seed": 17,
                    "created": {"length": 32768, "object_id": "ab"*16,
                                "sha256": hashlib.sha256(fixture.expected_content(32768, 17)).hexdigest()},
                    "measurement": {"object": "ab"*16, "bytes": 32768}}
        MODULE.validate_boot_resume(failed, baseline, [1, 2, 3, 4], "cd"*32, 65536, 17)
        for field, value in (("created", {}), ("authority", []), ("stage", "authority"), ("artifact_sha256", "00"*32)):
            invalid = dict(failed)
            invalid[field] = value
            with self.assertRaises(AssertionError):
                MODULE.validate_boot_resume(invalid, baseline, [1, 2, 3, 4], "cd"*32, 65536, 17)
        with self.assertRaises(AssertionError):
            MODULE.validate_boot_resume(failed, baseline, [1, 2, 3, 5], "cd"*32, 65536, 17)
        baseline["failure"] = "not successful"
        with self.assertRaises(AssertionError):
            MODULE.validate_boot_resume(failed, baseline, [1, 2, 3, 4], "cd"*32, 65536, 17)

    # ------------------------=
    # FUNC: test_only_full_size_runs_loaded_desktop_probe
    # DESC: Smaller transfer cases do not run unrelated desktop work; the required full-size callback remains mandatory.
    # ------------------=
    def test_only_full_size_runs_loaded_desktop_probe(self):
        calls = []
        callback = lambda: calls.append(1)
        self.assertIsNone(MODULE.loaded_ui_probe(32768, callback))
        self.assertIsNone(MODULE.loaded_ui_probe(65536, callback))
        MODULE.loaded_ui_probe(262144, callback)()
        self.assertEqual(calls, [1])

    # ------------------------=
    # FUNC: test_live_count_does_not_replace_reconciled_peer_row
    # DESC: Rejects old cached rows even with a successful live count, then accepts the exact trusted row at the matching checkpoint.
    # ------------------=
    def test_live_count_does_not_replace_reconciled_peer_row(self):
        peer = "12"*32
        stale = [0]*512
        stale[20:26] = [5, 4, 0, 0, 1, 3]
        row = bytearray(128)
        row[:32], row[85] = bytes.fromhex(peer), 1
        stale[128:144] = struct.unpack("<16Q", row)
        refreshed = stale.copy()
        refreshed[21] = 5
        row[85] = 3
        refreshed[128:144] = struct.unpack("<16Q", row)
        class Guest:
            # ------------------------=
            # FUNC: wait
            # DESC: Evaluates the real predicate before and after a typed projection refresh.
            # ------------------=
            def wait(self, predicate, label, timeout):
                assert not predicate(stale)
                assert predicate(refreshed)
                return refreshed
        self.assertEqual(MODULE.wait_peer_projection(Guest(), peer, 3), refreshed)

    # ------------------------=
    # FUNC: test_pairing_resume_preserves_exact_trust_and_artifact
    # DESC: Pairing-only failure cannot reuse authority/object stages or mismatched artifacts; asymmetric trust requires explicit repair.
    # ------------------=
    def test_pairing_resume_preserves_exact_trust_and_artifact(self):
        prior = {"identities": [1, 2, 3, 4], "stage": "pair-all-six", "failure": "recorded", "artifact_sha256": "ab"*32}
        MODULE.validate_resume_pairing(prior, [1, 2, 3, 4], "ab"*32)
        for field, value in (("stage", "authority"), ("authority", []), ("created", {}), ("artifact_sha256", "cd"*32)):
            invalid = dict(prior)
            invalid[field] = value
            with self.assertRaises(AssertionError): MODULE.validate_resume_pairing(invalid, [1, 2, 3, 4], "ab"*32)
        legacy = dict(prior)
        legacy.pop("artifact_sha256")
        with self.assertRaises(AssertionError): MODULE.validate_resume_pairing(legacy, [1, 2, 3, 4], "ab"*32)
        MODULE.validate_resume_pairing(legacy, [1, 2, 3, 4], "ab"*32, "ab"*32)
        self.assertEqual(MODULE.pair_action(3, 3), "preserve")
        self.assertEqual(MODULE.pair_action(1, 0), "pair")
        for pair in ((3, 1), (1, 3), (5, 5), (2, 2)):
            with self.assertRaises(AssertionError): MODULE.pair_action(*pair)

    # ------------------------=
    # FUNC: test_failed_measurement_retry_is_exact_and_not_completed
    # DESC: Accepts a failed exact byte-set case and rejects different identities, stage, digest, or completed timing evidence.
    # ------------------=
    def test_failed_measurement_retry_is_exact_and_not_completed(self):
        prior = {"identities": [1, 2, 3, 4], "stage": "bounded-transfer-measurement",
                 "failure": "recorded", "length": 32768, "seed": 17,
                 "created": {"object_id": "12"*16, "length": 32768, "seed": 17,
                             "sha256": hashlib.sha256(fixture.expected_content(32768, 17)).hexdigest(),
                             "version": 1, "manifest_generation": 1}}
        MODULE.validate_retry_measurement(prior, [1, 2, 3, 4], 32768, 17)
        for kind in ("identity", "stage", "hash", "completed", "no_failure", "zero_object"):
            invalid = copy.deepcopy(prior)
            if kind == "identity": invalid["identities"][0] = 5
            if kind == "stage": invalid["stage"] = "authority"
            if kind == "hash": invalid["created"]["sha256"] = "00"*32
            if kind == "completed": invalid["measurement"] = {"elapsed_seconds": 1}
            if kind == "no_failure": invalid["failure"] = None
            if kind == "zero_object": invalid["created"]["object_id"] = "00"*16
            with self.subTest(kind=kind), self.assertRaises(AssertionError):
                MODULE.validate_retry_measurement(invalid, [1, 2, 3, 4], 32768, 17)

    # ------------------------=
    # FUNC: test_published_resume_cannot_skip_object_evidence
    # DESC: Only the recorded publication failure before object creation can resume without reissuing grants.
    # ------------------=
    def test_published_resume_cannot_skip_object_evidence(self):
        prior = {"identities": [1, 2, 3, 4], "stage": "restore-explicit-publication-only",
                 "failure": "recorded", "resume_prepared": True}
        MODULE.validate_published(prior, [1, 2, 3, 4])
        for field, value in (("created", {}), ("stage", "boot"), ("failure", None),
                             ("identities", [4, 3, 2, 1]), ("resume_prepared", False)):
            with self.assertRaises(AssertionError):
                MODULE.validate_published(dict(prior, **{field: value}), [1, 2, 3, 4])

    # ------------------------=
    # FUNC: test_reuse_and_resume_modes_are_distinct
    # DESC: Configured reuse permits a new measurement, exact-object resume cannot create a new measurement, and both modes cannot coexist.
    # ------------------=
    def test_reuse_and_resume_modes_are_distinct(self):
        for resume, reuse, measurement in ((False, True, True), (False, True, False),
                                            (True, False, False), (False, False, True)):
            MODULE.validate_mode(resume, reuse, measurement)
        for resume, reuse, measurement in ((True, True, False), (True, True, True), (True, False, True)):
            with self.assertRaises(AssertionError):
                MODULE.validate_mode(resume, reuse, measurement)

    # ------------------------=
    # FUNC: test_size_seed_hash_and_identity_are_exact
    # DESC: Each supported actual byte count passes only its own metadata; altered size, seed, hash or object rejects resume.
    # ------------------=
    def test_size_seed_hash_and_identity_are_exact(self):
        for length in (32768, 65536, 262144):
            prior = {"stage": "measurement-complete-owner-loss-not-tested", "identities": [1, 2, 3, 4],
                     "persisted": [{}, {}, {}], "length": length, "seed": 17,
                     "created": {"length": length, "object_id": "ab" * 16,
                                 "sha256": hashlib.sha256(fixture.expected_content(length, 17)).hexdigest()},
                     "measurement": {"object": "ab" * 16, "bytes": length}}
            MODULE.validate_case(prior, [1, 2, 3, 4], length, 17)
            for other_length, seed in ((length + 1, 17), (length, 18)):
                with self.assertRaises(AssertionError):
                    MODULE.validate_case(prior, [1, 2, 3, 4], other_length, seed)
            prior["created"]["sha256"] = "00" * 32
            with self.assertRaises(AssertionError):
                MODULE.validate_case(prior, [1, 2, 3, 4], length, 17)


if __name__ == "__main__":
    unittest.main()
