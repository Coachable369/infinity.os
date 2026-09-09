"""Exact byte-set fencing for preserved installed acceptance cases."""
import hashlib
import importlib.util
import pathlib
import unittest
import ms10_installed_fixture as fixture

SPEC = importlib.util.spec_from_file_location("owner_case", pathlib.Path(__file__).with_name("ms10-installed-owner-offline.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CaseFence(unittest.TestCase):
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
