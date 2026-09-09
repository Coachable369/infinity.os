"""Explicit four-node authority graph for owner-offline installed acceptance.

These helpers grant only native operations through ordinary operator controls.
They do not establish completion, extend expired authority, or inject storage.
"""
from concurrent.futures import ThreadPoolExecutor
import struct
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
def resume_publication(guests, distribution):
    identities = [distribution.identity(guest) for guest in guests]
    with ThreadPoolExecutor(max_workers=4) as workers:
        grants = list(workers.map(lambda guest: publication_only(guest, identities), guests))
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
def establish_authority(guests, distribution):
    identities = [distribution.identity(guest) for guest in guests]
    assert len(guests) == len(set(identities)) == 4
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(grant_node, guest, identities) for guest in guests]
        grants = [future.result() for future in futures]
    with ThreadPoolExecutor(max_workers=4) as workers:
        futures = [workers.submit(configure_node, guest, identities, grants) for guest in guests]
        for future in futures:
            future.result()
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
