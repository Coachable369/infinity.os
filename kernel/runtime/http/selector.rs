//! Bounded edge-triggered native readiness bookkeeping, not a host descriptor API.
//! The service serializes access and owns socket authority and wait/wakeup delivery.
use crate::transport::Readiness;

/// Scoped to its owning selector; this identity is not a network capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration { slot: usize, generation: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event { pub token: usize, pub ready: Readiness }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Full, Stale, EmptyInterest }
#[derive(Clone, Copy)]
struct Entry {
    generation: u64,
    active: bool,
    token: usize,
    interest: Readiness,
    observed: Readiness,
    pending: Readiness,
}
const EMPTY: Readiness = Readiness { readable: false, writable: false };
pub struct Selector<const N: usize> { entries: [Entry; N], cursor: usize }

impl<const N: usize> Selector<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates fixed-capacity registration storage without allocating or granting socket authority.
    // ------------------=
    pub const fn new() -> Self {
        Self { entries: [Entry { generation: 0, active: false, token: 0,
            interest: EMPTY, observed: EMPTY, pending: EMPTY }; N], cursor: 0 }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Allocates a generation-tagged source and queues currently ready requested operations.
    // ------------------=
    pub fn register(&mut self, token: usize, interest: Readiness, ready: Readiness)
        -> Result<Registration, Error> {
        if !interest.readable && !interest.writable { return Err(Error::EmptyInterest); }
        for (slot, entry) in self.entries.iter_mut().enumerate() {
            if entry.active { continue; }
            let Some(generation) = entry.generation.checked_add(1) else { continue; };
            *entry = Entry { generation, active: true, token, interest,
                observed: ready, pending: mask(ready, interest) };
            return Ok(Registration { slot, generation });
        }
        Err(Error::Full)
    }

    // ------------------------=
    // FUNC: entry
    // DESC: Rejects deregistered and reused identities before accessing selector state.
    // ------------------=
    fn entry(&mut self, id: Registration) -> Result<&mut Entry, Error> {
        self.entries.get_mut(id.slot)
            .filter(|entry| entry.active && entry.generation == id.generation)
            .ok_or(Error::Stale)
    }

    // ------------------------=
    // FUNC: reregister
    // DESC: Replaces the token and interest atomically and rearms readiness from the current socket snapshot.
    // ------------------=
    pub fn reregister(&mut self, id: Registration, token: usize,
        interest: Readiness, ready: Readiness) -> Result<(), Error> {
        if !interest.readable && !interest.writable { return Err(Error::EmptyInterest); }
        let entry = self.entry(id)?;
        entry.token = token;
        entry.interest = interest;
        entry.observed = ready;
        entry.pending = mask(ready, interest);
        Ok(())
    }

    // ------------------------=
    // FUNC: observe
    // DESC: Coalesces newly ready edges; callers also report not-ready state after IO drains or returns WouldBlock.
    // ------------------=
    pub fn observe(&mut self, id: Registration, ready: Readiness) -> Result<(), Error> {
        let entry = self.entry(id)?;
        entry.pending.readable |= ready.readable && !entry.observed.readable && entry.interest.readable;
        entry.pending.writable |= ready.writable && !entry.observed.writable && entry.interest.writable;
        entry.observed = ready;
        Ok(())
    }

    // ------------------------=
    // FUNC: wake
    // DESC: Coalesces explicit control notifications without requiring a socket readiness transition.
    // ------------------=
    pub fn wake(&mut self, id: Registration) -> Result<(), Error> {
        let entry = self.entry(id)?;
        entry.pending.readable |= entry.interest.readable;
        entry.pending.writable |= entry.interest.writable;
        Ok(())
    }

    // ------------------------=
    // FUNC: deregister
    // DESC: Removes pending events before releasing a source slot; late notifications cannot target its replacement.
    // ------------------=
    pub fn deregister(&mut self, id: Registration) -> Result<(), Error> {
        let entry = self.entry(id)?;
        entry.active = false;
        entry.pending = EMPTY;
        Ok(())
    }

    // ------------------------=
    // FUNC: drain
    // DESC: Delivers at most the caller's capacity in round-robin order without losing undelivered events.
    // ------------------=
    pub fn drain(&mut self, output: &mut [Event]) -> usize {
        if N == 0 || output.is_empty() { return 0; }
        let start = self.cursor;
        let mut count = 0;
        for offset in 0..N {
            let slot = (start + offset) % N;
            let entry = &mut self.entries[slot];
            if !entry.active || (!entry.pending.readable && !entry.pending.writable) { continue; }
            output[count] = Event { token: entry.token, ready: entry.pending };
            entry.pending = EMPTY;
            count += 1;
            self.cursor = (slot + 1) % N;
            if count == output.len() { break; }
        }
        count
    }
}

// ------------------------=
// FUNC: mask
// DESC: Limits observable readiness to the registered operation interests.
// ------------------=
fn mask(ready: Readiness, interest: Readiness) -> Readiness {
    Readiness { readable: ready.readable && interest.readable,
        writable: ready.writable && interest.writable }
}

#[cfg(test)]
mod tests {
    use super::*;
    const READ: Readiness = Readiness { readable: true, writable: false };
    const WRITE: Readiness = Readiness { readable: false, writable: true };
    const BOTH: Readiness = Readiness { readable: true, writable: true };
    // ------------------------=
    // FUNC: coalesces_edges_and_rearms_without_idle_events
    // DESC: Exercises edge delivery, interest replacement, coalescing and explicit rearm behavior.
    // ------------------=
    #[test]
    fn coalesces_edges_and_rearms_without_idle_events() {
        let mut selector = Selector::<1>::new();
        let id = selector.register(7, BOTH, WRITE).unwrap();
        let mut output = [Event { token: 0, ready: EMPTY }; 1];
        assert_eq!(selector.drain(&mut output), 1);
        assert_eq!(output[0], Event { token: 7, ready: WRITE });
        for _ in 0..100 { selector.observe(id, WRITE).unwrap(); }
        assert_eq!(selector.drain(&mut output), 0);
        selector.observe(id, EMPTY).unwrap();
        selector.observe(id, READ).unwrap();
        selector.observe(id, BOTH).unwrap();
        assert_eq!(selector.drain(&mut output), 1);
        assert_eq!(output[0].ready, BOTH);
        selector.reregister(id, 9, READ, BOTH).unwrap();
        assert_eq!(selector.drain(&mut output), 1);
        assert_eq!(output[0], Event { token: 9, ready: READ });
        assert_eq!(selector.drain(&mut output), 0);
    }
    // ------------------------=
    // FUNC: reuse_invalidates_late_events_and_preserves_capacity
    // DESC: Checks stale source rejection, full and zero-capacity behavior and undelivered event retention.
    // ------------------=
    #[test]
    fn reuse_invalidates_late_events_and_preserves_capacity() {
        let mut selector = Selector::<1>::new();
        let old = selector.register(1, READ, READ).unwrap();
        assert_eq!(selector.register(2, READ, READ), Err(Error::Full));
        selector.deregister(old).unwrap();
        let current = selector.register(2, READ, READ).unwrap();
        assert_ne!(old, current);
        assert_eq!(selector.observe(old, BOTH), Err(Error::Stale));
        assert_eq!(selector.deregister(old), Err(Error::Stale));
        assert_eq!(selector.drain(&mut []), 0);
        let mut out = [Event { token: 0, ready: EMPTY }];
        assert_eq!(selector.drain(&mut out), 1);
        assert_eq!(out[0].token, 2);
        assert_eq!(Selector::<0>::new().register(0, READ, EMPTY), Err(Error::Full));
        assert_eq!(Selector::<0>::new().drain(&mut out), 0);
    }
    // ------------------------=
    // FUNC: bounded_delivery_is_fair
    // DESC: Ensures a frequently rearmed source cannot starve other ready sources in a small output buffer.
    // ------------------=
    #[test]
    fn bounded_delivery_is_fair() {
        let mut selector = Selector::<3>::new();
        let first = selector.register(1, READ, READ).unwrap();
        selector.register(2, READ, READ).unwrap();
        selector.register(3, READ, READ).unwrap();
        let mut out = [Event { token: 0, ready: EMPTY }];
        for expected in 1..=3 {
            assert_eq!(selector.drain(&mut out), 1);
            assert_eq!(out[0].token, expected);
            selector.reregister(first, 1, READ, READ).unwrap();
        }
    }
    // ------------------------=
    // FUNC: control_wakes_coalesce_but_survive_delivery
    // DESC: Verifies a wake after draining produces another event without busy-looping in between.
    // ------------------=
    #[test]
    fn control_wakes_coalesce_but_survive_delivery() {
        let mut selector = Selector::<1>::new();
        let id = selector.register(4, READ, EMPTY).unwrap();
        let mut out = [Event { token: 0, ready: EMPTY }];
        for _ in 0..100 { selector.wake(id).unwrap(); }
        assert_eq!(selector.drain(&mut out), 1);
        assert_eq!(out[0], Event { token: 4, ready: READ });
        assert_eq!(selector.drain(&mut out), 0);
        selector.wake(id).unwrap();
        assert_eq!(selector.drain(&mut out), 1);
        selector.deregister(id).unwrap();
        assert_eq!(selector.wake(id), Err(Error::Stale));
        assert_eq!(selector.drain(&mut out), 0);
    }
}
