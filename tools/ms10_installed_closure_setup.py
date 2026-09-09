"""Explicit four-node authority graph for owner-offline installed acceptance.

These helpers grant only native operations through ordinary operator controls.
They do not establish completion, extend expired authority, or inject storage.
"""
from concurrent.futures import ThreadPoolExecutor
import struct
import hashlib
import json
from ms10_installed_durable_grants import grant


# ------------------------=
# FUNC: grant_node
# DESC: Issues one node's directed metadata and resource-publication authority, plus A's exact replica operations on recipients.
# ------------------=
def grant_node(guest, identities):
    guest.launch("command", 5)
    records = {}
    for index, peer in enumerate(identities):
        if index + 1 == guest.number:
            continue
        guest.peer_policy(peer, "object", "allow")
        guest.peer_policy(peer, "namespace", "allow")
        item = {"metadata": grant(guest, peer, "pool-metadata"),
                "publication": grant(guest, peer, "resource-advertise")}
        if index == 0:
            item["replica"] = [grant(guest, peer, operation) for operation in
                               ("transfer-begin", "transfer-chunk", "transfer-commit",
                                "replica-inspect", "object-read", "replica-delete")]
        records[peer] = item
    return records


# ------------------------=
# FUNC: configure_node
# DESC: Configures caller-side grants from each receiving peer, maintaining the direction of capability issuance.
# ------------------=
def configure_node(guest, identities, grants):
    local = identities[guest.number - 1]
    guest.launch("command", 5)
    for index, peer in enumerate(identities):
        if peer == local:
            continue
        issued = grants[index][local]
        guest.command(f"pool metadata-authority peer=node:{peer} grant={issued['metadata']} durable=true confirm=true")
        if guest.number == 1:
            replica = issued["replica"]
            guest.command(f"pool participate peer=node:{peer} begin={replica[0]} chunk={replica[1]} commit={replica[2]} inspect={replica[3]} read={replica[4]} durable=true confirm=true")
            guest.command(f"pool retire-authority peer=node:{peer} grant={replica[5]} durable=true confirm=true")


# ------------------------=
# FUNC: publish_node
# DESC: Starts explicitly durable publication after authenticated sessions exist.
# ------------------=
def publish_node(guest, identities, grants):
    local = identities[guest.number - 1]
    guest.launch("command", 5)
    for index, peer in enumerate(identities):
        if peer != local:
            approval = grants[index][local]["publication"]
            guest.command(f"pool advertise peer=node:{peer} grant={approval} durable=true confirm=true")


# ------------------------=
# FUNC: publication_only
# DESC: Issues only explicit resource-publication grants after a verified prepared-state interruption; never modifies metadata or replica authority.
# ------------------=
def publication_only(guest, identities):
    local = identities[guest.number - 1]
    guest.launch("command", 5)
    return {peer: {"publication": grant(guest, peer, "resource-advertise")}
            for peer in identities if peer != local}


# ------------------------=
# FUNC: resume_publication
# DESC: Restores twelve explicitly granted publication directions without replaying trust, metadata configuration, or replica grants.
# ------------------=
def resume_publication(guests, distribution, grants=None):
    identities = [distribution.identity(guest) for guest in guests]
    if grants is None:
        raise AssertionError("Prepared authority receipt missing; refuse duplicate durable approval issuance")
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(publish_node, guest, identities, grants) for guest in guests]
        for future in futures:
            future.result()
    observations = []
    for guest in guests:
        state = guest.wait(lambda s: s[496] == 4, "local plus three restored real resources", timeout=90)
        observations.append({"node": guest.number, "online_resources": state[496],
                             "observed_clock": state[10], "sessions": state[26]})
    return {"publication_only": True, "observations": observations,
            "metadata_replica_configuration": "UNVERIFIED until actual transfer"}


# ------------------------=
# FUNC: establish_authority
# DESC: Builds all twelve directed metadata/publication relationships in parallel per node, then opens real sessions last.
# ------------------=
def establish_authority(guests, distribution, checkpoint=None):
    identities = [distribution.identity(guest) for guest in guests]
    assert len(guests) == len(set(identities)) == 4
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(grant_node, guest, identities) for guest in guests]
        grants = [future.result() for future in futures]
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(configure_node, guest, identities, grants) for guest in guests]
        for future in futures:
            future.result()
    if checkpoint is not None:
        checkpoint(identities, grants)
    for left in range(4):
        for right in range(left + 1, 4):
            distribution.open_session(guests[left], guests[right])
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(publish_node, guest, identities, grants) for guest in guests]
        for future in futures:
            future.result()
    observations = []
    for guest in guests:
        state = guest.wait(lambda s: s[496] == 4, "local plus three actual resource publishers", timeout=90)
        assert struct.pack("<4Q", *state[16:20]).hex() == identities[guest.number - 1]
        observations.append({"node": guest.number, "online_resources": state[496],
                             "observed_clock": state[10], "sessions": state[26]})
    return observations


# ------------------------=
# FUNC: artifact_hash
# DESC: Fences reusable approval evidence to the exact installed-generation executable artifact.
# ------------------=
def artifact_hash(work):
    with (work / "artifacts/installed-kernel.elf").open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


# ------------------------=
# FUNC: save_prepared
# DESC: Atomically saves issued typed approval handles after requester commands, without claiming transfer or configuration verification.
# ------------------=
def save_prepared(work, identities, grants):
    receipt = {"schema": 1, "identities": identities, "artifact_sha256": artifact_hash(work),
               "stage": "configuration-submitted-before-sessions", "grants": grants,
               "configuration_verified": False, "transfer_verified": False}
    target = work / "prepared-authority.json"
    assert not target.exists(), "Prepared authority receipt already exists; resume it explicitly"
    temporary = target.with_suffix(".pending")
    temporary.write_text(json.dumps(receipt, indent=2))
    temporary.replace(target)


# ------------------------=
# FUNC: validate_prepared_receipt
# DESC: Rejects wrong artifacts, missing directed approvals and non-durable handles before any VM is controlled.
# ------------------=
def validate_prepared_receipt(receipt, identities, digest):
    assert receipt["schema"] == 1 and receipt["stage"] == "configuration-submitted-before-sessions"
    assert receipt["identities"] == identities and len(set(identities)) == 4
    assert receipt["artifact_sha256"] == digest
    grants = receipt["grants"]
    assert len(grants) == 4
    for index, row in enumerate(grants):
        assert set(row) == set(identities) - {identities[index]}
        for peer, item in row.items():
            handles = [item["metadata"], item["publication"]]
            if peer == identities[0]:
                assert len(item["replica"]) == 6
                handles += item["replica"]
            assert all(isinstance(value, int) and 1 << 63 <= value < 1 << 64 for value in handles)
    return grants
