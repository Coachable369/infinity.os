"""Typed Console manifest pages versus live Settings object projection."""
import struct
import ms10_installed_fixture as fixture
from ms10_installed_pool import call
from ms10_installed_metadata import invoke


# ------------------------=
# FUNC: decode_manifest_projection
# DESC: Decodes bounded canonical header/placement pages into the same object facts exposed by Settings, deduplicating verified nodes.
# ------------------=
def decode_manifest_projection(header, placements):
    assert len(header) == 128 and header[:8] == b"INFPMF02"
    assert len(placements) == 1024
    object_id = header[8:24].hex()
    version = struct.unpack_from("<Q", header, 24)[0]
    generation = struct.unpack_from("<Q", header, 80)[0]
    assert object_id != "00" * 16 and version > 0 and generation > 0
    result = {"object": object_id, "version": version, "generation": generation,
              "desired": header[72], "verified": 0, "offline": 0}
    verified_nodes = set()
    for offset in range(0, 1024, 128):
        row = placements[offset:offset+128]
        state = row[0]
        assert 0 <= state <= 5
        if state == 2 and struct.unpack_from("<Q", row, 80)[0] == version and row[88:120] == header[40:72]:
            verified_nodes.add(bytes(row[8:40]))
        if state == 3:
            result["offline"] += 1
    result["verified"] = len(verified_nodes)
    return result


# ------------------------=
# FUNC: assert_row
# DESC: Compares exact machine identity and protection fields without relying on rendered labels or prose.
# ------------------=
def assert_row(console, row):
    observed = {"object": struct.pack("<2Q", *row[:2]).hex(), "version": row[2],
                "generation": row[3], "desired": row[5], "verified": row[6], "offline": row[7]}
    assert console == observed, {"console": console, "settings": observed}


# ------------------------=
# FUNC: observed_projection
# DESC: Decodes the explicit observed-health response while retaining canonical identity/version/generation authority.
# ------------------=
def observed_projection(reply, canonical, content_hash):
    data = reply["data"]
    assert reply["operation"] == 0xe010 and len(data) == 64
    assert reply["object"] == data[:16].hex() == canonical["object"]
    assert reply["version"] == canonical["version"] and reply["generation"] == canonical["generation"]
    assert data[16:48] == content_hash and data[48] == canonical["desired"]
    return {"object": canonical["object"], "version": canonical["version"],
            "generation": canonical["generation"], "desired": data[48],
            "verified": data[49], "offline": data[50]}


# ------------------------=
# FUNC: verify
# DESC: Inspects one generation-fenced object via ordinary Console pages, then requires a ready matching Settings row; a changing generation fails explicitly.
# ------------------=
def verify(guest, distribution, object_id, label, timeout=90, shared=False):
    initial = distribution.object_state(guest, object_id, lambda row: True, f"{label}-settings-before", timeout)
    before = fixture.read_state(guest, distribution.API.symbol)
    assert before[24] and before[19] < min(before[18], 8)
    assert struct.pack("<2Q", *before[32+before[19]*16:34+before[19]*16]).hex() == object_id, {"settings_object_not_selected": object_id}
    generation = initial[3]
    guest.launch("command", 5)
    pages = []
    for offset in (0, 64, *range(4352, 5376, 64)):
        command = f"pool inspect obj:{object_id} generation={generation} offset={offset}"
        reply = invoke(guest, command, lambda: fixture.read_state(guest, distribution.API.symbol)) if shared else call(guest, command, 0x300a)
        if shared:
            assert reply["operation"] == 0x300a
        assert reply["object"] == object_id and reply["generation"] == generation
        assert len(reply["data"]) == 64 and reply["value"] == 5376
        observed = guest.state()
        assert observed[110] == 1
        assert struct.unpack_from("<Q", struct.pack("<17Q", *observed[111:128]), 48)[0] == offset
        pages.append(reply["data"])
    placements = b"".join(pages[2:])
    console = decode_manifest_projection(b"".join(pages[:2]), placements)
    canonical = dict(console)
    if shared:
        health = invoke(guest, f"pool health obj:{object_id}", lambda: fixture.read_state(guest, distribution.API.symbol))
        console = observed_projection(health, canonical, b"".join(pages[:2])[40:72])
    current = distribution.object_state(guest, object_id, lambda row: True, f"{label}-settings-after", timeout)
    assert_row(console, current)
    pool = fixture.read_state(guest, distribution.API.symbol)
    selected = pool[32 + pool[19]*16:48 + pool[19]*16] if pool[19] < min(pool[18], 8) else None
    assert selected is not None and struct.pack("<2Q", *selected[:2]).hex() == object_id
    for index in range(8):
        encoded = placements[index*128:(index+1)*128]
        row = pool[160+index*10:170+index*10]
        expected = (encoded[8:40], encoded[40:56], encoded[56:72], struct.unpack_from("<Q", encoded, 80)[0], encoded[0])
        actual = (struct.pack("<4Q", *row[:4]), struct.pack("<2Q", *row[4:6]), struct.pack("<2Q", *row[6:8]), row[8], row[9])
        assert actual[:4] == expected[:4], {"placement_identity_parity_mismatch": index}
        if not shared:
            assert actual[4] == expected[4], {"placement_state_parity_mismatch": index}
    return {"status": "TESTED", "fields": console,
            "canonical_signed_fields": canonical, "health_source": "typed observed summary" if shared else "canonical local manifest",
            "selected_object_matches": True, "placement_identities_individually_compared": True,
            "placement_states_individually_compared": not shared}
