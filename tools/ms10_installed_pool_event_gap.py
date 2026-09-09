"""Final-runtime real IEF gap and authoritative rebuild acceptance."""
import time
from ms10_installed_pool import call
import ms10_installed_pool_parity as parity


# ------------------------=
# FUNC: change_pair
# DESC: Performs two genuine generation-fenced policy commits ending Critical without modifying content or injecting events.
# ------------------=
def change_pair(guest, object_id, generation, version):
    protected = call(guest, f"pool policy obj:{object_id} generation={generation} version={version} policy=protected", 0xe011)
    assert protected["object"] == object_id and protected["generation"] > generation
    assert protected["version"] == version and protected["data"][48] == 2
    critical = call(guest, f"pool policy obj:{object_id} generation={protected['generation']} version={version} policy=critical", 0xe011)
    assert critical["object"] == object_id and critical["generation"] > protected["generation"]
    assert critical["version"] == version and critical["data"][48] == 3
    return critical


# ------------------------=
# FUNC: verify
# DESC: Primes an actual policy topic, overwrites two committed events while Settings is closed, then requires gap detection and authoritative rebuild with unchanged full bytes.
# ------------------=
def verify(guest, distribution, verifier, created, owner):
    object_id = created["object_id"]
    row = distribution.object_state(guest, object_id, lambda r: r[5] == r[6] == 3,
                                    "event-gap-primer-ready")
    before = guest.state()
    guest.launch("command", 5)
    primed = change_pair(guest, object_id, row[3], row[2])
    distribution.object_state(guest, object_id, lambda r: r[3] >= primed["generation"] and r[5] == r[6] == 3,
                              "event-gap-primer-committed")
    observed = guest.wait(lambda s: s[510] > before[510] and s[507] == 0, "real policy event observed", timeout=60)
    current = distribution.object_state(guest, object_id, lambda r: r[5] == r[6] == 3, "event-gap-known-sequence")
    started = time.monotonic()
    guest.launch("command", 5)
    final = change_pair(guest, object_id, current[3], current[2])
    closed_seconds = time.monotonic() - started
    assert closed_seconds < 60, {"observer_lease_window_missed": closed_seconds}
    guest.launch("storage", 8, 8)
    rebuilt = guest.wait(lambda s: s[508] > observed[508] and s[509] > observed[509] and s[507] == 0,
                         "actual IEF gap and authoritative rebuild", timeout=90)
    row = distribution.object_state(guest, object_id, lambda r: r[3] >= final["generation"] and r[5] == r[6] == 3,
                                    "event-gap-critical-restored")
    assert row[2] == created["version"]
    console_settings = parity.verify(guest, distribution, object_id, "event-gap-console-parity")
    persisted = distribution.persisted_hash(guest, verifier, owner, created)
    return {"status": "TESTED", "closed_seconds": closed_seconds,
            "before": {"gaps": observed[508], "rebuilds": observed[509], "events": observed[510]},
            "after": {"gaps": rebuilt[508], "rebuilds": rebuilt[509], "events": rebuilt[510], "stale": rebuilt[507]},
            "policy_generation": final["generation"], "content_version_unchanged": True,
            "console_settings_parity": console_settings, "persisted_full_bytes": persisted}
