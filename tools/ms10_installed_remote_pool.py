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
    result = {"boundary": "installed QEMU authenticated remote Pool IOP", "owner": aid,
              "storage_host": bid, "object_id": object_id, "remote_create_update_read": True,
              "critical_reported_degraded": True, "replica_distribution_acceptance": False}
    (a.work.parent / "remote-pool-result.json").write_text(json.dumps(result, indent=2))
    a.screenshot("remote-pool-result")
    return result
