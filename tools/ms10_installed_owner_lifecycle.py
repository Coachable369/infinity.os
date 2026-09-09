"""Finite owner-return/mutation continuation after actual automatic repair."""
import hashlib
import json
import subprocess
import time
import ms10_installed_fixture as fixture
from ms10_installed_metadata import invoke, read_path, read_path_ready
from ms10_installed_pool import call


# ------------------------=
# FUNC: return_coverage
# DESC: Separates an offline owner's repaired-placement return from actual newer-content reconciliation using observed immutable versions.
# ------------------=
def return_coverage(owner_version_before, peer_version_before_return, owner_version_after,
                    original_generation, observed_generation):
    return {
        "original_owner_return": owner_version_after == peer_version_before_return,
        "newer_content_existed_before_owner_return": peer_version_before_return > owner_version_before,
        "stale_content_version_reconciled": (
            peer_version_before_return > owner_version_before
            and owner_version_after == peer_version_before_return
        ),
        "newer_placement_generation_observed": observed_generation > original_generation,
        "owner_version_before": owner_version_before,
        "peer_version_before_return": peer_version_before_return,
        "owner_version_after": owner_version_after,
    }


# ------------------------=
# FUNC: inspect_lifecycle
# DESC: Pauses an owned guest, verifies paused state, then invokes only the native read-only tombstone/outbox/audit decoder.
# ------------------=
def inspect_lifecycle(guest, verifier, owner, object_id, content_ids, label):
    guest.qmp("stop")
    try:
        status = guest.qmp("query-status")
        assert status["status"] == "paused" and not status["running"]
        result = subprocess.run([str(verifier), "--lifecycle", "--vm-paused", str(guest.disk), owner,
                                 object_id, *content_ids], check=True, capture_output=True, text=True, timeout=60)
        observed = json.loads(result.stdout)
        (guest.work / f"pool-lifecycle-{label}.json").write_text(json.dumps(observed, indent=2))
        return observed
    finally:
        guest.qmp("cont")


# ------------------------=
# FUNC: wait_retired
# DESC: Polls actual durable retirement state to a fixed deadline; content retention is reported separately and never inferred from an absent namespace entry.
# ------------------=
def wait_retired(guest, verifier, owner, object_id, content_ids, timeout=180):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        observed = inspect_lifecycle(guest, verifier, owner, object_id, content_ids, "retirement-progress")
        if observed["tombstone"]["present"] and observed["tombstone"]["deleted"] and not observed["object_present"]:
            if not observed["outbox"]["present"] or observed["outbox"]["pending"] is None:
                return observed
        time.sleep(1)
    raise AssertionError({"durable_retirement_deadline": object_id, "observed": observed})


# ------------------------=
# FUNC: cold_critical
# DESC: Cold-boots all three surviving replicas before any content mutation, verifies native full bytes, and exercises authorized shared readers while original owner remains off.
# ------------------=
def cold_critical(survivors, distribution, verifier, report):
    owner = report["identities"][0]
    created = report["created"]
    expected = fixture.expected_content(report["length"], report["seed"])
    proof = []
    report["critical_cold_proof"] = proof
    for index, guest in enumerate(survivors):
        before = guest.state()
        boot = guest.cold_boot_proof(before)
        assert distribution.identity(guest) == report["identities"][guest.number - 1]
        assert not guest.installer
        guest.fast_commands = True
        for peer in survivors:
            if peer is not guest:
                distribution.open_session(guest, peer)
        item = {"node": guest.number, "boot": boot, "installer_detached": True}
        proof.append(item)
        item["persisted_full_bytes"] = distribution.persisted_hash(guest, verifier, owner, created)
        if index < 2:
            guest.launch("command", 5)
            first = read_path_ready(guest, report["namespace_path"], created["object_id"], expected[:64],
                                    lambda: fixture.read_state(guest, distribution.API.symbol))
            last = read_path(guest, report["namespace_path"], created["object_id"], expected[-64:],
                             lambda: fixture.read_state(guest, distribution.API.symbol), offset=len(expected)-64)
            assert first["version"] == last["version"] == created["version"]
            item["first_and_last_shared_reads"] = True
            item["namespace_readiness"] = first["readiness_attempts"]
        else:
            item["shared_read"] = "NOT AUTHORIZED: destination has no reader grant"
    return proof


# ------------------------=
# FUNC: run
# DESC: Returns the stale original owner, uses ordinary fresh shared operations, and verifies copy independence without claiming untested garbage collection.
# ------------------=
def run(guests, distribution, verifier, report):
    a, b, c, replacement = guests
    owner = report["identities"][0]
    object_id = report["created"]["object_id"]
    path = report["namespace_path"]
    report["stage"] = "cold-critical-before-mutation"
    cold_proof = cold_critical((b, c, replacement), distribution, verifier, report)
    report["stage"] = "owner-return-after-cold-critical-proof"
    a.boot(False)
    a.authenticate()
    assert distribution.identity(a) == owner and not a.installer
    a.fast_commands = True
    for peer in (b, c, replacement):
        distribution.open_session(a, peer)
    stale_local = inspect_lifecycle(a, verifier, owner, object_id, [], "owner-before-fresh-read")
    assert stale_local["local_manifest"] is not None
    a.launch("command", 5)
    fresh = read_path(a, path, object_id, fixture.expected_content(64, report["seed"]),
                      lambda: fixture.read_state(a, distribution.API.symbol))
    assert fresh["version"] == report["created"]["version"]
    assert fresh["generation"] >= report["created"]["manifest_generation"]
    reconciled_local = inspect_lifecycle(a, verifier, owner, object_id, [], "owner-after-fresh-read")
    assert reconciled_local["local_manifest"]["generation"] > stale_local["local_manifest"]["generation"]
    assert reconciled_local["local_manifest"]["version"] == stale_local["local_manifest"]["version"]
    assert reconciled_local["local_manifest"]["hash"] == stale_local["local_manifest"]["hash"]
    returned = return_coverage(report["created"]["version"], report["created"]["version"],
                               fresh["version"], stale_local["local_manifest"]["generation"], reconciled_local["local_manifest"]["generation"])
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
    independent_disk = distribution.persisted_hash(a, verifier, owner, copy_description)
    original = read_path(a, path, object_id, updated_bytes, lambda: fixture.read_state(a, distribution.API.symbol))
    content_ids = [chunk["content"] for chunk in original_disk["chunks"]]
    before_delete = inspect_lifecycle(a, verifier, owner, object_id, content_ids, "before-delete")
    assert before_delete["object_present"]
    deleted = invoke(a, f"pool delete obj:{object_id} generation={original['generation']} version={original['version']} confirm=true",
                     lambda: fixture.read_state(a, distribution.API.symbol))
    assert deleted["operation"] == 0x3009 and deleted["object"] == object_id
    independent = call(a, f"pool read obj:{copy_id} generation={changed['generation']} version={changed['version']} offset=0 length={len(independent_bytes)}", 0x3002)
    assert independent["data"] == independent_bytes
    retired = wait_retired(a, verifier, owner, object_id, content_ids)
    assert retired["tombstone"]["generation"] >= deleted["generation"]
    report["delete_native_proof"] = retired
    report["stage"] = "cold-tombstone-after-retirement"
    tombstone_boot = a.cold_boot_proof(a.state())
    assert distribution.identity(a) == owner and not a.installer
    a.fast_commands = True
    cold_deleted = inspect_lifecycle(a, verifier, owner, object_id, content_ids, "cold-tombstone")
    assert cold_deleted["tombstone"]["present"] and cold_deleted["tombstone"]["deleted"]
    assert cold_deleted["tombstone"]["generation"] == retired["tombstone"]["generation"]
    assert not cold_deleted["object_present"]
    a.launch("command", 5)
    independent = call(a, f"pool read obj:{copy_id} generation={changed['generation']} version={changed['version']} offset=0 length={len(independent_bytes)}", 0x3002)
    assert independent["data"] == independent_bytes
    final_content_ids = sorted(set(content_ids + [chunk["content"] for chunk in independent_disk["chunks"]]))
    copy_deleted = call(a, f"pool delete obj:{copy_id} generation={changed['generation']} version={changed['version']} confirm=true", 0x3009)
    assert copy_deleted["object"] == copy_id
    final_reclamation = inspect_lifecycle(a, verifier, owner, copy_id, final_content_ids, "last-copy-reference-deleted")
    assert not final_reclamation["object_present"]
    assert all(not content["present"] for content in final_reclamation["content_objects"])
    return {"stale_original_owner_return": returned["stale_content_version_reconciled"],
            "owner_return_observations": returned, "fresh_shared_update": True,
            "stale_placement_manifest_reconciled_by_read": True,
            "owner_manifest_before_read": stale_local["local_manifest"],
            "owner_manifest_after_read": reconciled_local["local_manifest"],
            "owner_offline_content_mutation": "UNSUPPORTED: reader and placement-repair grants do not authorize content mutation",
            "critical_cold_reboot_before_mutation": cold_proof,
            "copy_id": copy_id, "immutable_chunk_sharing": True, "independent_copy_edit": True,
            "correlated_delete_completion": True, "copy_survives_original_delete": True,
            "cold_tombstone_proof": {"boot": tombstone_boot, "native": cold_deleted},
            "retirement_outbox_drained": True,
            "content_retention_observations": cold_deleted["content_objects"],
            "last_copy_reference_retired": True,
            "local_final_reference_reclamation": final_reclamation,
            "reclamation_nodes_inspected": [a.number],
            "remote_content_absence_verified": False,
            "garbage_collection": "TESTED: listed local immutable content identities absent after final copy deletion; remote retirement uses acknowledged outbox, not a secure-erasure claim",
            "full_ms10_acceptance": False}
