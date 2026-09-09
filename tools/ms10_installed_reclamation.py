"""Native recipient inventory and retirement proofs for explicitly owned QA guests."""
import json
import subprocess


# ------------------------=
# FUNC: observe
# DESC: Runs only the read-only recipient decoder while the guest is confirmed paused.
# ------------------=
def observe(guest, verifier, owner, object_id, ids, label):
    guest.qmp("stop")
    try:
        status = guest.qmp("query-status")
        assert status["status"] == "paused" and not status["running"]
        result = subprocess.run([str(verifier), "--recipient-reclamation", "--vm-paused",
                                 str(guest.disk), owner, object_id, *ids], check=True,
                                capture_output=True, text=True, timeout=60)
        state = json.loads(result.stdout)
        assert state["inspected"] and state["read_only"]
        (guest.work / f"pool-reclamation-{label}-{object_id}.json").write_text(json.dumps(state, indent=2))
        return state
    finally:
        guest.qmp("cont")


# ------------------------=
# FUNC: capture
# DESC: Captures each recipient's own physical identities before deletion; absent optional replicas remain explicitly uncovered.
# ------------------=
def capture(guests, verifier, owner, object_id, required):
    result = []
    for guest in guests:
        state = observe(guest, verifier, owner, object_id, [], "before")
        live = [b for b in state["bindings"] if not b["retired"]]
        if required:
            assert state["catalog_present"] and live, {"missing_recipient_inventory": guest.number}
        ids = sorted({identity for b in live for identity in (b["backing"], b["extent"])})
        assert all(identity and identity != "00" * 16 for identity in ids)
        result.append({"node": guest.number, "object": object_id, "before": state,
                       "ids": ids, "covered": bool(live)})
    return result


# ------------------------=
# FUNC: validate_retired
# DESC: Requires the exact previously observed versions' retirement markers and every captured physical identity absent.
# ------------------=
def validate_retired(before, after):
    assert before["covered"] and before["ids"], "empty inventory is not reclamation proof"
    assert after["catalog_present"]
    for old in before["before"]["bindings"]:
        matching = [b for b in after["bindings"] if b["version"] == old["version"]
                    and b["manifest_generation"] == old["manifest_generation"]]
        assert len(matching) == 1 and matching[0]["retired"]
        assert matching[0]["backing"] == "00" * 16 and matching[0]["extent"] is None
    assert len(after["physical_objects"]) == len(before["ids"])
    assert {p["id"] for p in after["physical_objects"]} == set(before["ids"])
    assert all(not p["present"] for p in after["physical_objects"])


# ------------------------=
# FUNC: finish
# DESC: Verifies retirement after acknowledged owner deletion, then cold-boots each recipient and repeats the same native identity observations.
# ------------------=
def finish(guests, distribution, verifier, owner, inventories):
    proof = []
    for guest in guests:
        items = [i for inventory in inventories for i in inventory if i["node"] == guest.number]
        covered = [i for i in items if i["covered"]]
        observations = []
        for item in covered:
            after = observe(guest, verifier, owner, item["object"], item["ids"], "retired")
            validate_retired(item, after)
            observations.append({"object": item["object"], "before": item["before"], "after": after})
        identity = distribution.identity(guest) if covered else None
        boot = guest.cold_boot_proof(guest.state()) if covered else None
        if covered:
            assert distribution.identity(guest) == identity and identity != owner and not guest.installer
        for item, observed in zip(covered, observations):
            cold = observe(guest, verifier, owner, item["object"], item["ids"], "cold-retired")
            validate_retired(item, cold)
            observed["cold"] = cold
        proof.append({"node": guest.number, "boot": boot, "objects": observations,
                      "not_present_before_delete": [i["object"] for i in items if not i["covered"]]})
    return proof
