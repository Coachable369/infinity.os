//! Event-driven visual history, not historical DOM execution. Captures are denied
//! until the trusted engine adapter has assessed the entire composited page,
//! including subframes. Unknown or editable/form content is not captured.
use crate::{Error, tabs::{Scope, TabId, Text}};

const HARD_BYTE_LIMIT: usize = 64 * 1024 * 1024;
const HARD_COUNT_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment { LoadComplete, BeforeNavigation, BeforeReload, StateTransition }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Safety { Unknown, SensitiveOrEditable, AssessedNonSensitive }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits { pub bytes: usize, pub count: usize, pub age_seconds: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotId(u64);

pub struct Capture<'a> {
    pub scope: Scope,
    pub tab: TabId,
    pub document: u64,
    pub moment: Moment,
    pub safety: Safety,
    pub timestamp: u64,
    pub url: &'a str,
    pub title: &'a str,
    pub scroll: (i32, i32),
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub tab: TabId,
    pub document: u64,
    pub moment: Moment,
    pub timestamp: u64,
    pub url: Text<2048>,
    pub title: Text<256>,
    pub scroll: (i32, i32),
    pub width: u32,
    pub height: u32,
    start: usize,
    length: usize,
}

pub struct Timeline<'a> {
    profile: u64,
    limits: Limits,
    next: u64,
    used: usize,
    snapshots: [Option<Snapshot>; HARD_COUNT_LIMIT],
    count: usize,
    arena: &'a mut [u8],
}
impl<'a> Timeline<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded normal-profile timeline with explicit storage and expiry limits.
    // ------------------=
    pub fn new(profile: u64, limits: Limits, arena: &'a mut [u8]) -> Result<Self, Error> {
        if limits.bytes == 0 || limits.bytes > HARD_BYTE_LIMIT || limits.count == 0
            || limits.count > HARD_COUNT_LIMIT || limits.age_seconds == 0 || arena.len() < limits.bytes {
            return Err(Error::Invalid);
        }
        Ok(Self { profile, limits, next: 1, used: 0, snapshots: [None; HARD_COUNT_LIMIT], count: 0, arena })
    }
    // ------------------------=
    // FUNC: capture
    // DESC: Copies an explicitly approved event frame once, evicting oldest records to meet the byte/count budget.
    // ------------------=
    pub fn capture(&mut self, input: Capture<'_>) -> Result<SnapshotId, Error> {
        if input.scope.private || input.scope.profile != self.profile || input.safety != Safety::AssessedNonSensitive
            || input.timestamp == 0 || input.width == 0 || input.height == 0 { return Err(Error::Invalid); }
        let length = (input.width as usize).checked_mul(input.height as usize).and_then(|n| n.checked_mul(4))
            .ok_or(Error::Invalid)?;
        if length != input.rgba.len() || length > self.limits.bytes { return Err(Error::Full); }
        let url = Text::new(input.url)?;
        let title = Text::new(input.title)?;
        // Do not let wall-clock rollback silently extend the lifetime of older captures.
        if self.count > 0 && input.timestamp < self.snapshots[self.count - 1].unwrap().timestamp {
            return Err(Error::Invalid);
        }
        self.expire(input.timestamp);
        if let Some(last) = self.count.checked_sub(1).and_then(|at| self.snapshots[at]) {
            if last.tab == input.tab && last.document == input.document && last.moment == input.moment
                && last.scroll == input.scroll && last.width == input.width && last.height == input.height
                && last.url == url && last.title == title
                && &self.arena[last.start..last.start + last.length] == input.rgba { return Ok(last.id); }
        }
        let next = self.next.checked_add(1).ok_or(Error::Exhausted)?;
        while self.count >= self.limits.count || self.used > self.limits.bytes - length {
            self.remove_oldest();
        }
        let id = SnapshotId(self.next);
        self.next = next;
        let start = self.used;
        self.arena[start..start + length].copy_from_slice(input.rgba);
        self.used += length;
        self.snapshots[self.count] = Some(Snapshot { id, tab: input.tab, document: input.document, moment: input.moment,
            timestamp: input.timestamp, url, title, scroll: input.scroll, width: input.width, height: input.height, start, length });
        self.count += 1;
        Ok(id)
    }
    // ------------------------=
    // FUNC: remove_oldest
    // DESC: Compacts bounded caller-owned storage after an event-driven eviction; no heap allocation is required.
    // ------------------=
    fn remove_oldest(&mut self) {
        if self.count == 0 { return; }
        let length = self.snapshots[0].unwrap().length;
        self.arena.copy_within(length..self.used, 0);
        self.arena[self.used - length..self.used].fill(0);
        self.used -= length;
        self.snapshots.copy_within(1..self.count, 0);
        self.count -= 1;
        self.snapshots[self.count] = None;
        for snapshot in self.snapshots[..self.count].iter_mut().flatten() { snapshot.start -= length; }
    }
    // ------------------------=
    // FUNC: expire
    // DESC: Enforces expiration on the owner clock without continuous image capture.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        while self.snapshots[0].map(|snapshot| now.saturating_sub(snapshot.timestamp) >= self.limits.age_seconds)
            .unwrap_or(false) { self.remove_oldest(); }
    }
    // ------------------------=
    // FUNC: inspect
    // DESC: Requires the matching non-private profile and enforces expiry even when the periodic sweep has not run.
    // ------------------=
    pub fn inspect(&self, scope: Scope, id: SnapshotId, now: u64) -> Option<(&Snapshot, &[u8])> {
        if scope.private || scope.profile != self.profile { return None; }
        let snapshot = self.snapshots[..self.count].iter().flatten().find(|entry| entry.id == id && now >= entry.timestamp
            && now - entry.timestamp < self.limits.age_seconds)?;
        Some((snapshot, &self.arena[snapshot.start..snapshot.start + snapshot.length]))
    }
    // ------------------------=
    // FUNC: retained_bytes
    // DESC: Exposes exact retained pixel-byte accounting for resource governance tests.
    // ------------------=
    pub fn retained_bytes(&self) -> usize { self.used }
    // ------------------------=
    // FUNC: clear
    // DESC: Drops all visual history when the user clears data or deletes the profile.
    // ------------------=
    pub fn clear(&mut self) {
        self.arena[..self.used].fill(0); self.snapshots.fill(None); self.count = 0; self.used = 0;
    }
}

impl Drop for Timeline<'_> {
    // ------------------------=
    // FUNC: drop
    // DESC: Clears retained pixels before returning the native arena to its owner.
    // ------------------=
    fn drop(&mut self) { self.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::Tabs;
    // ------------------------=
    // FUNC: privacy_pixels_and_limits
    // DESC: Exercises pixel ownership, profile isolation, fail-closed admission, duplicate suppression and automatic eviction.
    // ------------------=
    #[test]
    fn privacy_pixels_and_limits() {
        let scope = Scope { profile: 1, window: 1, private: false };
        let tab = Tabs::new(scope).create("https://example.com", true).unwrap();
        let mut arena = [0; 32];
        let mut timeline = Timeline::new(1, Limits { bytes: 32, count: 2, age_seconds: 10 }, &mut arena).unwrap();
        let pixels = [64; 16];
        let capture = |timestamp, safety, scope| Capture { scope, tab, document: 1,
            moment: Moment::LoadComplete, safety, timestamp, url: "https://example.com", title: "Example",
            scroll: (0, 10), width: 2, height: 2, rgba: &pixels };
        for safety in [Safety::Unknown, Safety::SensitiveOrEditable] {
            assert_eq!(timeline.capture(capture(1, safety, scope)), Err(Error::Invalid));
        }
        assert_eq!(timeline.capture(capture(1, Safety::AssessedNonSensitive, Scope { private: true, ..scope })), Err(Error::Invalid));
        assert_eq!(timeline.capture(capture(1, Safety::AssessedNonSensitive, Scope { profile: 2, ..scope })), Err(Error::Invalid));
        assert_eq!(timeline.retained_bytes(), 0);
        let first = timeline.capture(capture(1, Safety::AssessedNonSensitive, scope)).unwrap();
        assert_eq!(timeline.capture(capture(2, Safety::AssessedNonSensitive, scope)), Ok(first));
        assert_eq!(timeline.inspect(scope, first, 2).unwrap().1, &pixels);
        assert!(timeline.inspect(Scope { private: true, ..scope }, first, 2).is_none());
        assert!(timeline.inspect(Scope { profile: 2, ..scope }, first, 2).is_none());
        let second = timeline.capture(Capture { document: 2, ..capture(3, Safety::AssessedNonSensitive, scope) }).unwrap();
        timeline.capture(Capture { document: 3, ..capture(4, Safety::AssessedNonSensitive, scope) }).unwrap();
        assert!(timeline.inspect(scope, first, 4).is_none());
        assert_eq!(timeline.retained_bytes(), 32);
        assert!(timeline.inspect(scope, second, 13).is_none());
        timeline.expire(14);
        assert_eq!(timeline.retained_bytes(), 0);
        timeline.clear();
    }
}
