"""Independent installed Pool caller-fixture verification, never storage injection."""
import hashlib
import json
import struct
import time


# ------------------------=
# FUNC: expected_content
# DESC: Independently generates the documented deterministic acceptance bytes, bounded to fixture maximum.
# ------------------=
def expected_content(length, seed):
    assert 0 <= length <= 262144
    return bytes((offset * 73 + (seed & 0xffffffff) * 19 + (offset >> 8)) & 255
                 for offset in range(length))


# ------------------------=
# FUNC: read_state
# DESC: Reads the separate Pool diagnostic generation using a coherent stopped-guest binary snapshot.
# ------------------=
def read_state(guest, symbol):
    elf = guest.work.parent / "artifacts/installed-kernel.elf"
    if not hasattr(guest, "pool_diagnostic_symbol"):
        guest.pool_diagnostic_symbol = symbol(elf, "INFINITY_POOL_DIAGNOSTIC_SNAPSHOT")
    address, size = guest.pool_diagnostic_symbol
    assert size == 256 * 8
    guest.qmp("stop")
    try:
        state = struct.unpack("<256Q", guest.memory(address, size))
    finally:
        guest.qmp("cont")
    assert state[0] == 0x494e46504f4f4c31 and state[1] == 1
    assert state[2] == state[255] and state[2] % 2 == 0
    return state


# ------------------------=
# FUNC: create
# DESC: Invokes one privileged bounded native caller and verifies completed identity, byte count and independently computed hash.
# ------------------=
def create(guest, symbol, length=32768, seed=17, policy="critical"):
    assert policy in ("temporary", "protected", "critical")
    expected = expected_content(length, seed)
    guest.command(f"pool fixture length={length} seed={seed} policy={policy} confirm=true")
    deadline = time.monotonic() + 180
    last_offset = 0
    last_phase = 0
    while True:
        state = read_state(guest, symbol)
        assert state[3] != 255, {"fixture_phase": state[3], "offset": state[4], "fixture_error": state[31]}
        assert state[3] in (1, 2, 3, 4, 5), {"fixture_not_admitted": state[:16]}
        assert state[5] == length and state[4] <= length
        assert state[3] >= last_phase
        if state[3] == last_phase:
            assert last_offset <= state[4]
        last_offset, last_phase = state[4], state[3]
        if state[3] == 5:
            break
        assert time.monotonic() < deadline, {"fixture_timeout": state[:16]}
        # State-based polling; delay does not authorize a transition or hide failure.
        time.sleep(.1)
    object_id = struct.pack("<2Q", *state[6:8]).hex()
    assert object_id != "00" * 16
    assert struct.pack("<4Q", *state[8:12]) == hashlib.sha256(expected).digest()
    assert state[12] > 0 and state[13] > 0
    result = {"boundary": "installed native caller fixture through owned IOP",
              "object_id": object_id, "length": length, "seed": seed,
              "sha256": hashlib.sha256(expected).hexdigest(),
              "version": state[12], "manifest_generation": state[13],
              "distributed_acceptance": False}
    (guest.work / "large-fixture-result.json").write_text(json.dumps(result, indent=2))
    return result
