"""Installed local native Pool acceptance, not distributed/healing proof."""
import hashlib
import json
import struct

# ------------------------=
# FUNC: call
# DESC: Uses ordinary guest keyboard input and checks the binary native IOP response, never Console copy or log strings.
# ------------------=
def call(guest, command, operation):
    state = guest.command(command)
    assert state[110] == 1, "Native Pool request did not complete successfully"
    raw = struct.pack("<17Q", *state[111:128])
    schema, length, actual = struct.unpack_from("<HHI", raw)
    assert schema == 1 and actual == operation and length <= 64
    return {"object": raw[8:24].hex(), "authority": struct.unpack_from("<Q", raw, 24)[0],
            "generation": struct.unpack_from("<Q", raw, 32)[0], "version": struct.unpack_from("<Q", raw, 40)[0],
            "value": struct.unpack_from("<Q", raw, 64)[0], "data": raw[72:72+length]}

# ------------------------=
# FUNC: verify
# DESC: Creates and modifies an actual installed Pool object, verifies honest protection and integrity through IOP, then cold-boots without installer media and reads it again.
# ------------------=
def verify(guest):
    path = guest.work / "pool-local-result.json"
    if path.exists():
        result = json.loads(path.read_text())
        object_id = result["object_id"]
    else:
        payload = "CriticalDocument"
        created = call(guest, f"pool create nonce=1 policy=critical content={payload}", 0x3001)
        object_id = created["data"][:16].hex()
        assert object_id != "00" * 16
        assert created["data"][16:48] == hashlib.sha256(payload.encode()).digest()
        assert created["data"][48:50] == bytes((3, 2))
        assert (created["version"], created["generation"]) == (1, 1)
        inspected = call(guest, f"pool inspect obj:{object_id} offset=0 generation=1", 0x300a)
        assert inspected["data"][8:24].hex() == object_id
        assert struct.unpack_from("<QQ", inspected["data"], 24) == (1, len(payload))
        read = call(guest, f"pool read obj:{object_id} generation=1 version=1 offset=0 length={len(payload)}", 0x3002)
        assert read["data"] == payload.encode()
        policy = call(guest, f"pool policy obj:{object_id} generation=1 version=1 policy=protected", 0xe011)
        assert policy["data"][48:50] == bytes((2, 2)) and policy["generation"] == 2
        updated = call(guest, f"pool write obj:{object_id} generation=2 version=1 content=Changed", 0x3003)
        assert (updated["version"], updated["generation"]) == (2, 3)
        assert updated["data"][:16].hex() == object_id
        assert updated["data"][16:48] == hashlib.sha256(b"Changed").digest()
        empty = call(guest, "pool create nonce=2 policy=temporary", 0x3001)
        assert empty["data"][16:48] == hashlib.sha256(b"").digest()
        assert empty["data"][48:50] == bytes((1, 1)) and empty["value"] == 0
        assert empty["data"][:16].hex() != object_id
        copied = call(guest, f"pool copy obj:{object_id} generation=3 version=2 nonce=3", 0x3008)
        copy_id = copied["data"][:16].hex()
        assert copy_id not in (object_id, empty["data"][:16].hex(), "00" * 16)
        assert copied["data"][16:48] == hashlib.sha256(b"Changed").digest()
        assert (copied["version"], copied["generation"]) == (1, 1)
        copy_update = call(guest, f"pool write obj:{copy_id} generation=1 version=1 content=Independent", 0x3003)
        assert (copy_update["version"], copy_update["generation"]) == (2, 2)
        original = call(guest, f"pool read obj:{object_id} generation=3 version=2 offset=0 length=7", 0x3002)
        assert original["data"] == b"Changed"
        result = {"boundary": "installed QEMU local native Pool IOP", "object_id": object_id,
                  "empty_object_id": empty["data"][:16].hex(), "create_read_update_policy": True,
                  "copy_object_id": copy_id, "independent_copy_modification": True,
                  "critical_and_protected_report_degraded": True, "distributed_acceptance": False}
        # Preserve the committed identities before reboot so an interrupted
        # harness can resume verification without creating or rewriting data.
        path.write_text(json.dumps(result, indent=2))
    identity = guest.state()
    guest.cold_boot_proof(identity)
    guest.launch("command", 5)
    read = call(guest, f"pool read obj:{object_id} generation=3 version=2 offset=0 length=7", 0x3002)
    assert read["data"] == b"Changed" and read["object"] == object_id
    copy_read = call(guest, f"pool read obj:{result['copy_object_id']} generation=2 version=2 offset=0 length=11", 0x3002)
    assert copy_read["data"] == b"Independent"
    listed = call(guest, "pool list offset=0", 0xe010)
    assert listed["value"] == 3
    result["cold_boot_manifest_content_policy_persistence"] = True
    result["installer_detached"] = not guest.installer
    assert result["installer_detached"]
    guest.screenshot("pool-local-cold-read")
    path.write_text(json.dumps(result, indent=2))
    return result
