"""Finite owner-return/mutation continuation after actual automatic repair."""
import hashlib
import ms10_installed_fixture as fixture
from ms10_installed_metadata import invoke, read_path
from ms10_installed_pool import call


# ------------------------=
# FUNC: run
# DESC: Returns the stale original owner, uses ordinary fresh shared operations, and verifies copy independence without claiming untested garbage collection.
# ------------------=
def run(guests, distribution, verifier, report):
    a, b, c, replacement = guests
    owner = report["identities"][0]
    object_id = report["created"]["object_id"]
    path = report["namespace_path"]
    a.boot(False)
    a.authenticate()
    assert distribution.identity(a) == owner and not a.installer
    a.fast_commands = True
    for peer in (b, c, replacement):
        distribution.open_session(a, peer)
    a.launch("command", 5)
    fresh = read_path(a, path, object_id, fixture.expected_content(64, report["seed"]),
                      lambda: fixture.read_state(a, distribution.API.symbol))
    assert fresh["version"] == report["created"]["version"]
    assert fresh["generation"] >= report["created"]["manifest_generation"]
    updated_bytes = b"MS10OwnerUpdated"
    updated = invoke(a, f"pool write obj:{object_id} generation={fresh['generation']} version={fresh['version']} content={updated_bytes.decode()}",
                     lambda: fixture.read_state(a, distribution.API.symbol))
    assert updated["operation"] == 0x3003 and updated["object"] == object_id
    assert updated["version"] == fresh["version"] + 1
    assert updated["generation"] > fresh["generation"]
    b.launch("command", 5)
    current = read_path(b, path, object_id, updated_bytes,
                        lambda: fixture.read_state(b, distribution.API.symbol))
    assert current["version"] == updated["version"]
    a.launch("command", 5)
    copied = invoke(a, f"pool copy obj:{object_id} generation={current['generation']} version={current['version']} nonce=901",
                    lambda: fixture.read_state(a, distribution.API.symbol))
    assert copied["operation"] == 0x3008
    copy_id = copied["data"][:16].hex()
    assert copy_id not in (object_id, "00" * 16)
    assert copied["data"][16:48] == hashlib.sha256(updated_bytes).digest()
    original_description = {"object_id": object_id, "sha256": hashlib.sha256(updated_bytes).hexdigest(),
                            "version": current["version"]}
    copy_description = {"object_id": copy_id, "sha256": original_description["sha256"], "version": copied["version"]}
    original_disk = distribution.persisted_hash(a, verifier, owner, original_description)
    copy_disk = distribution.persisted_hash(a, verifier, owner, copy_description)
    assert original_disk["chunks"] == copy_disk["chunks"]
    independent_bytes = b"MS10IndependentCopy"
    changed = call(a, f"pool write obj:{copy_id} generation={copied['generation']} version={copied['version']} content={independent_bytes.decode()}", 0x3003)
    assert changed["version"] == copied["version"] + 1
    read_path(a, path, object_id, updated_bytes, lambda: fixture.read_state(a, distribution.API.symbol))
    independent = call(a, f"pool read obj:{copy_id} generation={changed['generation']} version={changed['version']} offset=0 length={len(independent_bytes)}", 0x3002)
    assert independent["data"] == independent_bytes
    copy_description.update({"sha256": hashlib.sha256(independent_bytes).hexdigest(), "version": changed["version"]})
    distribution.persisted_hash(a, verifier, owner, copy_description)
    original = read_path(a, path, object_id, updated_bytes, lambda: fixture.read_state(a, distribution.API.symbol))
    deleted = invoke(a, f"pool delete obj:{object_id} generation={original['generation']} version={original['version']} confirm=true",
                     lambda: fixture.read_state(a, distribution.API.symbol))
    assert deleted["operation"] == 0x3009 and deleted["object"] == object_id
    independent = call(a, f"pool read obj:{copy_id} generation={changed['generation']} version={changed['version']} offset=0 length={len(independent_bytes)}", 0x3002)
    assert independent["data"] == independent_bytes
    return {"stale_original_owner_return": True, "fresh_shared_update": True,
            "copy_id": copy_id, "immutable_chunk_sharing": True, "independent_copy_edit": True,
            "correlated_delete_completion": True, "copy_survives_original_delete": True,
            "cold_tombstone_proof": "NOT TESTED", "garbage_collection": "NOT TESTED",
            "full_ms10_acceptance": False}
