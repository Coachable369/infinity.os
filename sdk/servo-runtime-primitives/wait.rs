//! Bounded compare-and-park bookkeeping for a single execution owner.
//! The owner must serialize prepare/wake/expire with context state transitions.
//! This is not a cross-CPU futex implementation: external wake requests must be
//! queued to that owner, which drains them before choosing the next context.
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Changed, TimedOut, Woken }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Full, AlreadyWaiting, InvalidIdentity }
#[derive(Clone, Copy)]
struct Slot { thread: u64, address: usize, deadline: Option<u64>, result: Option<Outcome> }
const EMPTY: Slot = Slot { thread: 0, address: 0, deadline: None, result: None };
pub struct Waits<const N: usize> { slots: [Slot; N] }
impl<const N: usize> Waits<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates fixed-capacity wait registrations without allocation.
    // ------------------=
    pub const fn new() -> Self { Self { slots: [EMPTY; N] } }
    // ------------------------=
    // FUNC: prepare
    // DESC: Registers before rechecking the value; caller parks only when no immediate result is returned.
    // ------------------=
    pub fn prepare(&mut self, thread: u64, word: &AtomicU32, expected: u32,
        deadline: Option<u64>, now: u64) -> Result<Option<Outcome>, Error> {
        if thread == 0 { return Err(Error::InvalidIdentity); }
        if self.slots.iter().any(|s| s.thread == thread) { return Err(Error::AlreadyWaiting); }
        if word.load(Ordering::Acquire) != expected { return Ok(Some(Outcome::Changed)); }
        if deadline.is_some_and(|d| d <= now) { return Ok(Some(Outcome::TimedOut)); }
        let slot = self.slots.iter_mut().find(|s| s.thread == 0).ok_or(Error::Full)?;
        *slot = Slot { thread, address: word as *const AtomicU32 as usize, deadline, result: None };
        if word.load(Ordering::Acquire) != expected {
            *slot = EMPTY;
            return Ok(Some(Outcome::Changed));
        }
        Ok(None)
    }
    // ------------------------=
    // FUNC: wake
    // DESC: Marks one or all still-parked registrations ready; repeated wakes do not inflate counts.
    // ------------------=
    pub fn wake(&mut self, word: &AtomicU32, all: bool) -> usize {
        let address = word as *const AtomicU32 as usize;
        let mut count = 0;
        for slot in &mut self.slots {
            if slot.thread != 0 && slot.address == address && slot.result.is_none() {
                slot.result = Some(Outcome::Woken);
                count += 1;
                if !all { break; }
            }
        }
        count
    }
    // ------------------------=
    // FUNC: expire
    // DESC: Applies monotonic deadlines without overriding a wake already delivered.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        for slot in &mut self.slots {
            if slot.thread != 0 && slot.result.is_none() && slot.deadline.is_some_and(|d| d <= now) {
                slot.result = Some(Outcome::TimedOut);
            }
        }
    }
    // ------------------------=
    // FUNC: take
    // DESC: Consumes a completed registration, releasing capacity only when its owner observes completion.
    // ------------------=
    pub fn take(&mut self, thread: u64) -> Option<Outcome> {
        let slot = self.slots.iter_mut().find(|s| s.thread == thread && thread != 0)?;
        let result = slot.result?;
        *slot = EMPTY;
        Some(result)
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Removes an exiting thread registration so address reuse cannot wake a stale owner.
    // ------------------=
    pub fn cancel(&mut self, thread: u64) {
        for slot in &mut self.slots { if slot.thread == thread { *slot = EMPTY; } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: wake_deadline_and_capacity_are_observable
    // DESC: Exercises full capacity, wake-one/all, deadline ordering, reuse, cancellation and changed-value refusal.
    // ------------------=
    #[test]
    fn wake_deadline_and_capacity_are_observable() {
        let word = AtomicU32::new(7);
        let other = AtomicU32::new(7);
        let mut waits = Waits::<2>::new();
        assert_eq!(waits.prepare(0, &word, 7, None, 0), Err(Error::InvalidIdentity));
        assert_eq!(waits.prepare(1, &word, 8, None, 0), Ok(Some(Outcome::Changed)));
        assert_eq!(waits.prepare(1, &word, 7, Some(0), 0), Ok(Some(Outcome::TimedOut)));
        assert_eq!(waits.prepare(1, &word, 7, Some(10), 0), Ok(None));
        assert_eq!(waits.prepare(1, &word, 7, None, 0), Err(Error::AlreadyWaiting));
        assert_eq!(waits.prepare(2, &word, 7, Some(10), 0), Ok(None));
        assert_eq!(waits.prepare(3, &word, 7, None, 0), Err(Error::Full));
        assert_eq!(waits.wake(&other, true), 0);
        assert_eq!(waits.wake(&word, false), 1);
        waits.expire(10);
        assert_eq!(waits.take(1), Some(Outcome::Woken));
        assert_eq!(waits.take(2), Some(Outcome::TimedOut));
        assert_eq!(waits.take(1), None);
        for id in 1..=2 { assert_eq!(waits.prepare(id, &word, 7, None, 0), Ok(None)); }
        assert_eq!(waits.wake(&word, true), 2);
        assert_eq!(waits.wake(&word, true), 0);
        waits.cancel(1);
        assert_eq!(waits.take(1), None);
        assert_eq!(waits.take(2), Some(Outcome::Woken));
        word.store(9, Ordering::Release);
        assert_eq!(waits.prepare(1, &word, 7, None, 0), Ok(Some(Outcome::Changed)));
    }
}
