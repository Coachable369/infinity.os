"""Installed authenticated remote Pool IOP; not replica/healing acceptance."""
import hashlib
import json
import struct
import time

# ------------------------=
# FUNC: invoke
# DESC: Submits once through ordinary Console input, then collects the same owned request and verifies its actual correlated protocol completion.
# ------------------=
def invoke(guest, peer, grant, command, operation, expected=1):
    before = guest.state()[489]
    guest.command(f"{command} peer=node:{peer} grant={grant}")
    request = guest.wait(lambda state: state[489] > before, "remote Pool admission")[489]
    deadline = time.monotonic() + 120
    while True:
        assert time.monotonic() < deadline, {"uncollected_storage_request": request}
        state = guest.command(f"pool result request={request}")
        if state[490] == request:
            break
    assert state[491] != 0 and state[492] != 0
    assert state[493] == expected, {"request": request, "expected": expected, "actual": state[493]}
    print(json.dumps({"node": guest.number, "pool_request": request, "operation": operation,
                      "completion_status": state[493], "correlation": state[491],
                      "causation": state[492]}), flush=True)
    if expected != 1:
        return None
    assert state[110] == 1
    raw = struct.pack("<17Q", *state[111:128])
    schema, length, actual = struct.unpack_from("<HHI", raw)
    assert schema == 1 and length <= 64 and actual == operation
    return raw[72:72+length]

# ------------------------=
# FUNC: verify
# DESC: Creates an A-owned object durably on B using exact peer grants, then remotely reads and changes it without bypassing the native service.
# ------------------=
def verify(a, b):
    aid = struct.pack("<4Q", *a.state()[16:20]).hex()
    bid = struct.pack("<4Q", *b.state()[16:20]).hex()
    for guest in (a, b):
        guest.key("esc")
        guest.launch("command", 5)
    b.peer_policy(aid, "object", "allow")
    b.peer_policy(aid, "namespace", "allow")
    create = b.peer_grant(aid, "object-create")
    data = invoke(a, bid, create, "pool create nonce=71 policy=critical content=RemoteDocument", 0x3001)
    object_id = data[:16].hex()
    assert object_id != "00" * 16 and data[16:48] == hashlib.sha256(b"RemoteDocument").digest()
    assert data[48:50] == bytes((3, 2))  # One real host is DEGRADED, never three replicas.
    read = b.peer_grant(aid, "object-read")
    data = invoke(a, bid, read, f"pool read obj:{object_id} generation=1 version=1 offset=0 length=14", 0x3002)
    assert data == b"RemoteDocument"
    update = b.peer_grant(aid, "object-update")
    data = invoke(a, bid, update, f"pool write obj:{object_id} generation=1 version=1 content=RemoteChanged", 0x3003)
    assert data[:16].hex() == object_id and data[16:48] == hashlib.sha256(b"RemoteChanged").digest()
    data = invoke(a, bid, read, f"pool read obj:{object_id} generation=2 version=2 offset=0 length=13", 0x3002)
    assert data == b"RemoteChanged"
    advertisement = verify_advertisement(a, b, aid, bid)
    result = {"boundary": "installed QEMU authenticated remote Pool IOP", "owner": aid,
              "storage_host": bid, "object_id": object_id, "remote_create_update_read": True,
              "critical_reported_degraded": True, "resource_advertisement": advertisement,
              "replica_distribution_acceptance": False}
    (a.work.parent / "remote-pool-result.json").write_text(json.dumps(result, indent=2))
    a.screenshot("remote-pool-result")
    return result

# ------------------------=
# FUNC: verify_advertisement
# DESC: Compares received resource identity and allocator capacity against the installed sender, verifies a newer lease sequence, then revokes authority and observes expiry.
# ------------------=
def verify_advertisement(a, b, aid, bid):
    observed = a.command("storage status")
    assert observed[110] == 1
    raw = struct.pack("<17Q", *observed[111:128])
    resource = raw[72:88].hex()
    device = raw[88:104].hex()
    capacity = struct.unpack_from("<Q", raw, 40)[0]
    grant = b.peer_grant(aid, "resource-advertise")
    a.command(f"pool advertise peer=node:{bid} grant={grant}")
    received = b.wait(lambda s: s[496] == 1, "authenticated measured resource received", timeout=90)
    assert struct.pack("<2Q", *received[497:499]).hex() == resource
    assert struct.pack("<2Q", *received[499:501]).hex() == device
    assert received[501] == capacity and received[502] <= capacity
    assert received[503] <= capacity - received[502]
    a.fast_input_probe()
    a.screenshot("resource-publisher-active-input")
    a.frame_report("resource-publisher-active-input")
    a.key("esc")
    renewed = b.wait(lambda s: s[505] > received[505], "new resource sequence renewed", timeout=90)
    assert renewed[506] > received[506]
    a.wait(lambda s: s[494] >= 2 and s[495] == 0, "authenticated publication completions")
    b.command(f"node capability-revoke {grant}")
    a.wait(lambda s: s[495] != 0, "publication stopped after authority revocation", timeout=90)
    expired = b.wait(lambda s: s[496] == 0, "revoked resource lease expired", timeout=90)
    assert expired[109] == 1  # Known resource is retained, not deleted.
    assert struct.pack("<2Q", *expired[497:499]).hex() == resource
    return {"real_identity_capacity": True, "renewed_sequence": True,
            "revocation_stops_renewal": True, "offline_identity_retained": True,
            "rapid_input_with_publisher_enabled": True}
