//! Bounded owner-thread tab identities and preview policy. This module grants no
//! network, storage or engine authority. A tab ID is never a reusable slot index.
use crate::Error;

pub const MAX_TABS: usize = 24;
const CLOSED_LIMIT: usize = 8;
pub const GLANCE_DELAY_MS: u64 = 500;
pub const RECOVERY_LIMIT: usize = 32 + MAX_TABS * 2312;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope { pub profile: u64, pub window: u64, pub private: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text<const N: usize> { bytes: [u8; N], length: usize }
impl<const N: usize> Text<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Rejects oversized metadata instead of truncating a URL or UTF-8 sequence.
    // ------------------=
    pub fn new(value: &str) -> Result<Self, Error> {
        if value.len() > N || value.chars().any(char::is_control) { return Err(Error::Invalid); }
        let mut result = Self { bytes: [0; N], length: value.len() };
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(result)
    }
    // ------------------------=
    // FUNC: as_str
    // DESC: Returns the validated bounded text without allocation.
    // ------------------=
    pub fn as_str(&self) -> &str { core::str::from_utf8(&self.bytes[..self.length]).unwrap_or("") }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub scope: Scope,
    pub url: Text<2048>,
    pub title: Text<256>,
    pub pinned: bool,
    pub muted: bool,
    pub audio_playing: bool,
    pub loading: bool,
    /// Engine-issued document revision; previews from an earlier document fail closed.
    pub document: u64,
}

pub struct Tabs {
    scope: Scope,
    entries: [Option<Tab>; MAX_TABS],
    length: usize,
    active: Option<TabId>,
    next_id: u64,
    closed: [Option<Tab>; CLOSED_LIMIT],
    closed_length: usize,
}

impl Tabs {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a window's tabs to one profile and private-session boundary.
    // ------------------=
    pub const fn new(scope: Scope) -> Self {
        Self { scope, entries: [None; MAX_TABS], length: 0, active: None,
            next_id: 1, closed: [None; CLOSED_LIMIT], closed_length: 0 }
    }
    // ------------------------=
    // FUNC: index
    // DESC: Resolves a stable tab identity only while it is live.
    // ------------------=
    fn index(&self, id: TabId) -> Result<usize, Error> {
        self.entries[..self.length].iter().position(|tab| tab.as_ref().map(|tab| tab.id) == Some(id))
            .ok_or(Error::Stale)
    }
    // ------------------------=
    // FUNC: get
    // DESC: Inspects live metadata without exposing mutable identity or isolation fields.
    // ------------------=
    pub fn get(&self, id: TabId) -> Option<&Tab> { self.entries[self.index(id).ok()?].as_ref() }
    // ------------------------=
    // FUNC: ordered
    // DESC: Enumerates visible order without requiring engine recreation when tabs move.
    // ------------------=
    pub fn ordered(&self) -> impl Iterator<Item = &Tab> { self.entries[..self.length].iter().flatten() }
    // ------------------------=
    // FUNC: active
    // DESC: Returns the selected identity, independent of order and background updates.
    // ------------------=
    pub fn active(&self) -> Option<TabId> { self.active }
    // ------------------------=
    // FUNC: create
    // DESC: Creates a bounded tab; background creation preserves current keyboard focus.
    // ------------------=
    pub fn create(&mut self, url: &str, foreground: bool) -> Result<TabId, Error> {
        if self.length == MAX_TABS { return Err(Error::Full); }
        let url = Text::new(url)?;
        let next = self.next_id.checked_add(1).ok_or(Error::Exhausted)?;
        let id = TabId(self.next_id);
        self.entries[self.length] = Some(Tab { id, scope: self.scope, url, title: Text::new("")?,
            pinned: false, muted: false, audio_playing: false, loading: false, document: 0 });
        self.next_id = next;
        self.length += 1;
        if foreground || self.active.is_none() { self.active = Some(id); }
        Ok(id)
    }
    // ------------------------=
    // FUNC: select
    // DESC: Switches only to an existing tab; closing or stale hover events cannot resurrect it.
    // ------------------=
    pub fn select(&mut self, id: TabId) -> Result<(), Error> { self.index(id)?; self.active = Some(id); Ok(()) }
    // ------------------------=
    // FUNC: cycle
    // DESC: Wraps keyboard traversal in either direction without altering visual order.
    // ------------------=
    pub fn cycle(&mut self, previous: bool) -> Option<TabId> {
        let at = self.index(self.active?).ok()?;
        let next = if previous { (at + self.length - 1) % self.length } else { (at + 1) % self.length };
        self.active = self.entries[next].map(|tab| tab.id);
        self.active
    }
    // ------------------------=
    // FUNC: close
    // DESC: Removes a tab and chooses its neighbor; private tabs never enter reopen history.
    // ------------------=
    pub fn close(&mut self, id: TabId) -> Result<(), Error> {
        let at = self.index(id)?;
        let tab = self.entries[at].take().ok_or(Error::Stale)?;
        if !self.scope.private {
            if self.closed_length == CLOSED_LIMIT {
                self.closed.copy_within(1..CLOSED_LIMIT, 0);
                self.closed_length -= 1;
            }
            self.closed[self.closed_length] = Some(tab);
            self.closed_length += 1;
        }
        self.entries.copy_within(at + 1..self.length, at);
        self.length -= 1;
        self.entries[self.length] = None;
        if self.active == Some(id) {
            self.active = if self.length == 0 { None } else { self.entries[at.min(self.length - 1)].map(|tab| tab.id) };
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: reopen
    // DESC: Opens closed metadata under a fresh identity, rejecting late events from the old engine.
    // ------------------=
    pub fn reopen(&mut self) -> Result<TabId, Error> {
        let at = self.closed_length.checked_sub(1).ok_or(Error::Stale)?;
        let old = self.closed[at].ok_or(Error::Stale)?;
        let id = self.create(old.url.as_str(), true)?;
        self.entries[self.length - 1].as_mut().unwrap().title = old.title;
        self.set_pinned(id, old.pinned)?;
        self.closed[at] = None;
        self.closed_length = at;
        Ok(id)
    }
    // ------------------------=
    // FUNC: duplicate
    // DESC: Duplicates the current URL without sharing mutable history, media or engine state.
    // ------------------=
    pub fn duplicate(&mut self, id: TabId) -> Result<TabId, Error> {
        let tab = *self.get(id).ok_or(Error::Stale)?;
        self.create(tab.url.as_str(), true)
    }
    // ------------------------=
    // FUNC: reorder
    // DESC: Moves one tab within its pinned or ordinary partition and preserves active identity.
    // ------------------=
    pub fn reorder(&mut self, id: TabId, target: usize) -> Result<(), Error> {
        let at = self.index(id)?;
        if target >= self.length || self.entries[at].unwrap().pinned != self.entries[target].unwrap().pinned {
            return Err(Error::Invalid);
        }
        let tab = self.entries[at];
        if at < target { self.entries.copy_within(at + 1..target + 1, at); }
        else { self.entries.copy_within(target..at, target + 1); }
        self.entries[target] = tab;
        Ok(())
    }
    // ------------------------=
    // FUNC: set_pinned
    // DESC: Keeps pinned tabs contiguous at the leading edge without recreating the page.
    // ------------------=
    pub fn set_pinned(&mut self, id: TabId, pinned: bool) -> Result<(), Error> {
        let at = self.index(id)?;
        if self.entries[at].unwrap().pinned == pinned { return Ok(()); }
        let count = self.ordered().filter(|tab| tab.pinned).count();
        let target = if pinned { count } else { count - 1 };
        let mut tab = self.entries[at].unwrap();
        tab.pinned = pinned;
        if at < target { self.entries.copy_within(at + 1..target + 1, at); }
        else { self.entries.copy_within(target..at, target + 1); }
        self.entries[target] = Some(tab);
        Ok(())
    }
    // ------------------------=
    // FUNC: begin_navigation
    // DESC: Invalidates cached document metadata at navigation start before a new frame arrives.
    // ------------------=
    pub fn begin_navigation(&mut self, id: TabId) -> Result<u64, Error> {
        let at = self.index(id)?;
        let tab = self.entries[at].as_mut().unwrap();
        tab.document = tab.document.checked_add(1).ok_or(Error::Exhausted)?;
        tab.loading = true;
        tab.audio_playing = false;
        Ok(tab.document)
    }
    // ------------------------=
    // FUNC: update
    // DESC: Applies engine metadata only to the matching tab and document generation.
    // ------------------=
    pub fn update(&mut self, id: TabId, document: u64, url: &str, title: &str, loading: bool,
        audio_playing: bool) -> Result<(), Error> {
        let at = self.index(id)?;
        let url = Text::new(url)?;
        let title = Text::new(title)?;
        let tab = self.entries[at].as_mut().unwrap();
        if tab.document != document { return Err(Error::Stale); }
        tab.url = url; tab.title = title; tab.loading = loading; tab.audio_playing = audio_playing;
        Ok(())
    }
    // ------------------------=
    // FUNC: set_muted
    // DESC: Records a successful engine mute operation; callers must not treat this as audio enforcement.
    // ------------------=
    pub fn set_muted(&mut self, id: TabId, muted: bool) -> Result<(), Error> {
        let at = self.index(id)?; self.entries[at].as_mut().unwrap().muted = muted; Ok(())
    }
    // ------------------------=
    // FUNC: encode_recovery
    // DESC: Encodes normal-window URL/order/pin/selection state for an atomic native-object commit; never engine or form state.
    // ------------------=
    pub fn encode_recovery(&self, output: &mut [u8]) -> Result<usize, Error> {
        if self.scope.private { return Err(Error::Invalid); }
        let needed = 32 + self.ordered().map(|tab| 8 + tab.url.length + tab.title.length).sum::<usize>();
        if output.len() < needed { return Err(Error::Full); }
        output[..needed].fill(0);
        output[..4].copy_from_slice(b"IBS1");
        output[4..8].copy_from_slice(&(needed as u32).to_le_bytes());
        output[8..16].copy_from_slice(&self.scope.profile.to_le_bytes());
        output[16] = self.length as u8;
        output[17] = self.active.and_then(|id| self.index(id).ok()).map(|at| at as u8).unwrap_or(255);
        let mut at = 32;
        for tab in self.ordered() {
            output[at] = tab.pinned as u8;
            output[at + 1] = tab.muted as u8;
            output[at + 2..at + 4].copy_from_slice(&(tab.url.length as u16).to_le_bytes());
            output[at + 4..at + 6].copy_from_slice(&(tab.title.length as u16).to_le_bytes());
            at += 8;
            output[at..at + tab.url.length].copy_from_slice(tab.url.as_str().as_bytes());
            at += tab.url.length;
            output[at..at + tab.title.length].copy_from_slice(tab.title.as_str().as_bytes());
            at += tab.title.length;
        }
        let checksum = recovery_checksum(&output[..needed]);
        output[24..32].copy_from_slice(&checksum.to_le_bytes());
        Ok(needed)
    }
    // ------------------------=
    // FUNC: decode_recovery
    // DESC: Validates the entire bounded versioned record before returning any recovered tabs; checksum detects torn writes, not tampering.
    // ------------------=
    pub fn decode_recovery(profile: u64, window: u64, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 32 || bytes.len() > RECOVERY_LIMIT || &bytes[..4] != b"IBS1"
            || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize != bytes.len()
            || u64::from_le_bytes(bytes[8..16].try_into().unwrap()) != profile
            || bytes[16] as usize > MAX_TABS || bytes[18..24].iter().any(|b| *b != 0)
            || u64::from_le_bytes(bytes[24..32].try_into().unwrap()) != recovery_checksum(bytes) {
            return Err(Error::Invalid);
        }
        let count = bytes[16] as usize;
        let active = bytes[17] as usize;
        if (count == 0 && active != 255) || (count != 0 && active >= count) { return Err(Error::Invalid); }
        let mut result = Self::new(Scope { profile, window, private: false });
        let mut at = 32;
        let mut unpinned = false;
        for index in 0..count {
            let header = bytes.get(at..at + 8).ok_or(Error::Invalid)?;
            if header[0] > 1 || header[1] > 1 || header[6] != 0 || header[7] != 0 { return Err(Error::Invalid); }
            let pinned = header[0] == 1;
            if unpinned && pinned { return Err(Error::Invalid); }
            unpinned |= !pinned;
            let url_length = u16::from_le_bytes([header[2], header[3]]) as usize;
            let title_length = u16::from_le_bytes([header[4], header[5]]) as usize;
            at += 8;
            let url = core::str::from_utf8(bytes.get(at..at + url_length).ok_or(Error::Invalid)?).map_err(|_| Error::Invalid)?;
            at += url_length;
            let title = core::str::from_utf8(bytes.get(at..at + title_length).ok_or(Error::Invalid)?).map_err(|_| Error::Invalid)?;
            at += title_length;
            let id = result.create(url, index == active)?;
            result.update(id, 0, url, title, false, false)?;
            result.set_pinned(id, pinned)?;
            result.set_muted(id, header[1] == 1)?;
        }
        if at != bytes.len() { return Err(Error::Invalid); }
        Ok(result)
    }
}

// ------------------------=
// FUNC: recovery_checksum
// DESC: Detects accidental corruption including header changes while omitting the checksum field itself.
// ------------------=
fn recovery_checksum(bytes: &[u8]) -> u64 {
    bytes.iter().enumerate().filter(|(at, _)| !(24..32).contains(at))
        .fold(0xcbf29ce484222325u64, |value, (_, byte)| (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3))
}

/// Non-owning retained-surface token, valid only for the exact profile/tab/document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preview { pub tab: TabId, pub document: u64, pub surface: u64, pub scope: Scope }
pub struct Glance { hover: Option<(TabId, u64)> }
impl Glance {
    // ------------------------=
    // FUNC: new
    // DESC: Starts dismissed without allocating a WebView or surface.
    // ------------------=
    pub const fn new() -> Self { Self { hover: None } }
    // ------------------------=
    // FUNC: hover
    // DESC: Starts the intentional delay only when the pointer enters a different tab.
    // ------------------=
    pub fn hover(&mut self, tab: Option<TabId>, now_ms: u64) {
        if self.hover.map(|entry| entry.0) != tab { self.hover = tab.map(|id| (id, now_ms)); }
    }
    // ------------------------=
    // FUNC: visible
    // DESC: Borrows a ready retained preview without changing focus or accepting stale or cross-profile pixels.
    // ------------------=
    pub fn visible(&self, tabs: &Tabs, now_ms: u64, preview: Preview) -> bool {
        let Some((id, started)) = self.hover else { return false; };
        let Some(tab) = tabs.get(id) else { return false; };
        now_ms.checked_sub(started).map(|elapsed| elapsed >= GLANCE_DELAY_MS).unwrap_or(false)
            && tabs.active() != Some(id) && preview.tab == id && preview.document == tab.document
            && preview.scope == tab.scope && preview.surface != 0
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    const NORMAL: Scope = Scope { profile: 1, window: 1, private: false };
    // ------------------------=
    // FUNC: reorder_pin_cycle_close_reopen_preserve_identity
    // DESC: Exercises real ordered state transitions and rejects events from a closed lifetime.
    // ------------------=
    #[test]
    fn reorder_pin_cycle_close_reopen_preserve_identity() {
        let mut tabs = Tabs::new(NORMAL);
        let a = tabs.create("https://example.com/a", true).unwrap();
        let b = tabs.create("https://example.com/b", false).unwrap();
        let c = tabs.duplicate(a).unwrap();
        tabs.set_pinned(b, true).unwrap();
        assert_eq!(tabs.ordered().map(|tab| tab.id).collect::<std::vec::Vec<_>>(), [b, a, c]);
        assert_eq!(tabs.reorder(a, 0), Err(Error::Invalid));
        tabs.reorder(a, 2).unwrap();
        assert_eq!(tabs.active(), Some(c));
        assert_eq!(tabs.cycle(true), Some(b));
        tabs.close(b).unwrap();
        let replacement = tabs.reopen().unwrap();
        assert_ne!(replacement, b);
        assert!(tabs.get(replacement).unwrap().pinned);
        assert_eq!(tabs.select(b), Err(Error::Stale));
        tabs.set_pinned(replacement, false).unwrap();
        assert!(!tabs.get(replacement).unwrap().pinned);
    }
    // ------------------------=
    // FUNC: bounds_and_private_reopen
    // DESC: Verifies fixed capacity and private closed-tab data exclusion.
    // ------------------=
    #[test]
    fn bounds_and_private_reopen() {
        let mut tabs = Tabs::new(Scope { profile: 2, window: 2, private: true });
        for _ in 0..MAX_TABS { tabs.create("about:blank", false).unwrap(); }
        assert_eq!(tabs.create("about:blank", false), Err(Error::Full));
        tabs.close(tabs.active().unwrap()).unwrap();
        assert_eq!(tabs.reopen(), Err(Error::Stale));
        assert_eq!(Text::<2>::new("€"), Err(Error::Invalid));
        assert_eq!(Text::<10>::new("a\nb"), Err(Error::Invalid));
    }
    // ------------------------=
    // FUNC: glance_delay_dismissal_and_document_isolation
    // DESC: Rejects premature, closed, stale, private-crossing and wrong-profile previews without focus changes.
    // ------------------=
    #[test]
    fn glance_delay_dismissal_and_document_isolation() {
        let mut tabs = Tabs::new(NORMAL);
        let a = tabs.create("about:blank", true).unwrap();
        let b = tabs.create("https://example.com", false).unwrap();
        let mut glance = Glance::new();
        let preview = Preview { tab: b, document: 0, surface: 1, scope: NORMAL };
        glance.hover(Some(b), 100);
        assert!(!glance.visible(&tabs, 599, preview));
        glance.hover(Some(b), 599);
        assert!(glance.visible(&tabs, 600, preview));
        assert_eq!(tabs.active(), Some(a));
        assert!(!glance.visible(&tabs, 600, Preview { scope: Scope { profile: 2, ..NORMAL }, ..preview }));
        assert!(!glance.visible(&tabs, 600, Preview { scope: Scope { window: 2, ..NORMAL }, ..preview }));
        assert!(!glance.visible(&tabs, 600, Preview { scope: Scope { private: true, ..NORMAL }, ..preview }));
        let doc = tabs.begin_navigation(b).unwrap();
        assert!(!glance.visible(&tabs, 600, preview));
        assert_eq!(tabs.update(b, 0, "https://example.com/old", "old", false, false), Err(Error::Stale));
        let preview = Preview { document: doc, ..preview };
        assert!(glance.visible(&tabs, 600, preview));
        glance.hover(None, 601);
        assert!(!glance.visible(&tabs, 601, preview));
        glance.hover(Some(b), 602);
        tabs.close(b).unwrap();
        assert!(!glance.visible(&tabs, 2000, preview));
    }
    // ------------------------=
    // FUNC: recovery_round_trip_and_corruption
    // DESC: Verifies profile-bound restart state, every truncated prefix, corrupt bytes and private-session non-writing.
    // ------------------=
    #[test]
    fn recovery_round_trip_and_corruption() {
        let mut tabs = Tabs::new(NORMAL);
        let a = tabs.create("https://example.com/one", true).unwrap();
        let b = tabs.create("https://example.com/two", true).unwrap();
        tabs.set_pinned(a, true).unwrap();
        tabs.set_muted(b, true).unwrap();
        tabs.update(b, 0, "https://example.com/two", "Two", false, true).unwrap();
        let mut bytes = [0; RECOVERY_LIMIT];
        let len = tabs.encode_recovery(&mut bytes).unwrap();
        let recovered = Tabs::decode_recovery(1, 2, &bytes[..len]).unwrap();
        let current = recovered.get(recovered.active().unwrap()).unwrap();
        assert_eq!(current.url, tabs.get(b).unwrap().url);
        assert!(current.muted);
        assert!(!current.audio_playing);
        assert!(recovered.ordered().next().unwrap().pinned);
        assert_eq!(current.scope.window, 2);
        assert!(Tabs::decode_recovery(2, 2, &bytes[..len]).is_err());
        for cut in 0..len { assert!(Tabs::decode_recovery(1, 2, &bytes[..cut]).is_err()); }
        for index in 0..len {
            bytes[index] ^= 1;
            assert!(Tabs::decode_recovery(1, 2, &bytes[..len]).is_err());
            bytes[index] ^= 1;
        }
        let private = Tabs::new(Scope { private: true, ..NORMAL });
        bytes.fill(42);
        assert_eq!(private.encode_recovery(&mut bytes), Err(Error::Invalid));
        assert!(bytes.iter().all(|byte| *byte == 42));
    }
}
