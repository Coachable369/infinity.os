//! Bounded sequenced IEF hints; only authoritative snapshot reads clear stale state.
use super::*;
use event::{EventFilter, OverflowPolicy};
use iop::storage_protocol::{
    EVENT_OBJECT_CHANGED, EVENT_POLICY_CHANGED, EVENT_REPLICA_CHANGED, EVENT_RESOURCE_CHANGED,
};
const TOPICS: [u32; 4] = [
    EVENT_OBJECT_CHANGED,
    EVENT_POLICY_CHANGED,
    EVENT_REPLICA_CHANGED,
    EVENT_RESOURCE_CHANGED,
];
pub(super) struct State {
    caps: [Option<u64>; 4],
    leases: [Option<u64>; 4],
    expires: [u64; 4],
    last: [u64; 4],
    cursor: usize,
    epoch: u64,
    scan: u64,
    pub stale: bool,
    pub gaps: u64,
    pub rebuilt: u64,
    pub received: u64,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates four fixed topic observers with no queued renderer work.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            caps: [None; 4],
            leases: [None; 4],
            expires: [0; 4],
            last: [0; 4],
            cursor: 0,
            epoch: 0,
            scan: 0,
            stale: false,
            gaps: 0,
            rebuilt: 0,
            received: 0,
        }
    }
}
// ------------------------=
// FUNC: poll
// DESC: Receives at most one exact-source sequenced hint per tick, marking gaps stale without treating event content as storage authority.
// ------------------=
#[inline(never)]
pub(super) fn poll(r: &mut InfinityRuntime, now: u64) {
    let Some(holder) = r.service_identity(SERVICE_SETTINGS) else {
        return;
    };
    let Some(source) = r.service_identity(SERVICE_REPLICA_STORAGE) else {
        return;
    };
    let i = r.storage_view.events.cursor;
    r.storage_view.events.cursor = (i + 1) % 4;
    let cap = if let Some(cap) = r.storage_view.events.caps[i] {
        cap
    } else {
        let Ok(cap) = r.capabilities.grant(
            CapabilityType::EventSubscribe,
            TOPICS[i] as u64,
            1,
            0,
            source,
            holder,
            None,
            0,
        ) else {
            return;
        };
        r.storage_view.events.caps[i] = Some(cap);
        cap
    };
    if r.capabilities
        .validate(
            cap,
            holder,
            CapabilityType::EventSubscribe,
            TOPICS[i] as u64,
            1,
            0,
            now,
        )
        .is_err()
    {
        r.storage_view.events.caps[i] = None;
        r.storage_view.events.leases[i] = None;
        return;
    }
    if now >= r.storage_view.events.expires[i] {
        r.storage_view.events.leases[i] = None;
    }
    let lease = if let Some(lease) = r.storage_view.events.leases[i] {
        lease
    } else {
        let expires = now.saturating_add(60);
        let Ok(lease) = r.events.subscribe(
            holder,
            cap,
            EventFilter {
                type_id: TOPICS[i],
                scope: None,
            },
            expires,
            OverflowPolicy::LatestOnly,
            1,
            &r.capabilities,
            now,
        ) else {
            return;
        };
        r.storage_view.events.leases[i] = Some(lease);
        r.storage_view.events.expires[i] = expires;
        lease
    };
    let Ok(e) = r.events.receive(lease, now) else {
        return;
    };
    if e.source != source || e.type_id != TOPICS[i] || e.payload_len != 40 {
        return;
    }
    let v = &mut r.storage_view;
    let last = v.events.last[i];
    let reconstruct = u32::from_le_bytes(e.payload[36..40].try_into().unwrap())
        & iop::storage_protocol::transition::RECONSTRUCT
        != 0;
    if (last != 0 && e.sequence != last.saturating_add(1)) || reconstruct {
        v.events.gaps = v.events.gaps.saturating_add(1);
    }
    v.events.last[i] = e.sequence;
    v.events.received = v.events.received.saturating_add(1);
    v.events.epoch = v.events.epoch.saturating_add(1);
    v.events.stale = true;
    v.next = 0;
}
// ------------------------=
// FUNC: begin
// DESC: Fences a new typed snapshot against the latest consumed event epoch.
// ------------------=
pub(super) fn begin(v: &mut View) {
    v.events.scan = v.events.epoch;
}
// ------------------------=
// FUNC: complete
// DESC: Clears stale status only after authoritative reads finish without intervening events; otherwise restarts bounded reconstruction.
// ------------------=
pub(super) fn complete(v: &mut View, now: u64) -> bool {
    if v.events.scan != v.events.epoch {
        v.phase = 0;
        v.next = now;
        return false;
    }
    if v.events.stale {
        v.events.rebuilt = v.events.rebuilt.saturating_add(1);
        v.events.stale = false;
    }
    true
}
