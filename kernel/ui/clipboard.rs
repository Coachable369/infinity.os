//! Capability-gated typed clipboard and drag payload foundation.

pub const MAX_CLIPBOARD_BYTES: usize = 16384;
pub const MAX_OBJECT_REFS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardKind {
    Empty,
    Utf8Text,
    ObjectRefs,
    ImageObjectRef,
}

#[derive(Clone, Copy)]
pub struct ClipboardPayload {
    pub kind: ClipboardKind,
    pub bytes: [u8; MAX_CLIPBOARD_BYTES],
    pub length: u16,
    pub source_context: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardError {
    AccessDenied,
    TooLarge,
    InvalidType,
    Empty,
}

pub struct ClipboardService {
    payload: ClipboardPayload,
}

impl ClipboardService {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty typed clipboard without ambient read or write authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            payload: ClipboardPayload {
                kind: ClipboardKind::Empty,
                bytes: [0; MAX_CLIPBOARD_BYTES],
                length: 0,
                source_context: 0,
                generation: 0,
            },
        }
    }

    // ------------------------=
    // FUNC: write
    // DESC: Replaces clipboard content only after an external capability decision authorizes the caller.
    // ------------------=
    pub fn write(
        &mut self,
        authorized: bool,
        caller: u32,
        kind: ClipboardKind,
        bytes: &[u8],
    ) -> Result<u32, ClipboardError> {
        if !authorized {
            return Err(ClipboardError::AccessDenied);
        }
        if bytes.len() > MAX_CLIPBOARD_BYTES {
            return Err(ClipboardError::TooLarge);
        }
        if kind == ClipboardKind::Empty && !bytes.is_empty() {
            return Err(ClipboardError::InvalidType);
        }
        if kind == ClipboardKind::Utf8Text && core::str::from_utf8(bytes).is_err() {
            return Err(ClipboardError::InvalidType);
        }
        self.payload.bytes.fill(0);
        self.payload.bytes[..bytes.len()].copy_from_slice(bytes);
        self.payload.length = bytes.len() as u16;
        self.payload.kind = kind;
        self.payload.source_context = caller;
        self.payload.generation = self.payload.generation.wrapping_add(1);
        Ok(self.payload.generation)
    }

    // ------------------------=
    // FUNC: read
    // DESC: Returns typed clipboard metadata and content only to an authorized caller.
    // ------------------=
    pub fn read(&self, authorized: bool) -> Result<&ClipboardPayload, ClipboardError> {
        if authorized {
            Ok(&self.payload)
        } else {
            Err(ClipboardError::AccessDenied)
        }
    }
}

/// Session-owned native clipboard. Contents never persist to disk or diagnostic output.
pub struct SessionClipboard {
    service: ClipboardService,
    owner: [u8; 16],
    unlocked: bool,
    epoch: u64,
    written_ms: u64,
    ttl_ms: u64,
    browser_read: bool,
    browser_write: bool,
    permits: [u64; 2],
}

impl SessionClipboard {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no session authority and no copied data.
    // ------------------=
    pub const fn new() -> Self {
        Self { service: ClipboardService::new(), owner: [0;16], unlocked: false,
            epoch: 1, written_ms: 0, ttl_ms: 0, browser_read: true, browser_write: true, permits: [0;2] }
    }
    // ------------------------=
    // FUNC: session
    // DESC: Purges content and outstanding browser grants when locked or changing sessions.
    // ------------------=
    pub fn session(&mut self, owner: [u8;16], unlocked: bool) {
        let unlocked = unlocked && owner != [0;16];
        if self.owner != owner || self.unlocked != unlocked {
            self.clear();
            self.epoch = self.epoch.wrapping_add(1).max(1);
            if self.owner != owner { self.ttl_ms=0; self.browser_read=true; self.browser_write=true; }
            self.owner=owner; self.unlocked=unlocked;
        }
    }
    // ------------------------=
    // FUNC: authorized
    // DESC: Checks full session identity rather than a truncated user identifier.
    // ------------------=
    pub fn authorized(&self, owner: [u8;16]) -> bool { self.unlocked && self.owner==owner }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases every content byte and revokes pending browser gestures.
    // ------------------=
    pub fn clear(&mut self) { let _=self.service.write(true,0,ClipboardKind::Empty,&[]); self.permits=[0;2]; }
    // ------------------------=
    // FUNC: expire
    // DESC: Removes expired data, including when the monotonic clock regresses.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        if self.service.payload.kind!=ClipboardKind::Empty && self.ttl_ms!=0
            && (now<self.written_ms || now-self.written_ms>=self.ttl_ms) { self.clear(); }
    }
    // ------------------------=
    // FUNC: configure
    // DESC: Applies bounded session-local retention and independent browser read/write restrictions.
    // ------------------=
    pub fn configure(&mut self, owner:[u8;16], seconds:u32, read:bool, write:bool, now:u64) -> bool {
        if !self.authorized(owner) || seconds>86400 {return false;}
        if self.browser_read!=read || self.browser_write!=write {self.revoke_browser();}
        self.ttl_ms=u64::from(seconds)*1000;self.browser_read=read;self.browser_write=write;
        self.permits=[0;2];self.expire(now);true
    }
    // ------------------------=
    // FUNC: policy
    // DESC: Returns non-content policy values for native settings and diagnostics.
    // ------------------=
    pub fn policy(&self) -> (u32,bool,bool) { ((self.ttl_ms/1000) as u32,self.browser_read,self.browser_write) }
    // ------------------------=
    // FUNC: write
    // DESC: Atomically replaces typed contents only for the unlocked owning session.
    // ------------------=
    pub fn write(&mut self, owner:[u8;16], kind:ClipboardKind, bytes:&[u8], now:u64) -> Result<(),ClipboardError> {
        let kind=if kind==ClipboardKind::Utf8Text && bytes.is_empty() {ClipboardKind::Empty}else{kind};
        self.service.write(self.authorized(owner),0,kind,bytes)?;
        self.written_ms=now;Ok(())
    }
    // ------------------------=
    // FUNC: read
    // DESC: Copies a complete typed payload or leaves the destination untouched on denial or overflow.
    // ------------------=
    pub fn read(&mut self, owner:[u8;16], kind:ClipboardKind, out:&mut [u8], now:u64) -> Result<usize,ClipboardError> {
        self.expire(now);
        let p=self.service.read(self.authorized(owner))?;
        if p.kind==ClipboardKind::Empty {return Err(ClipboardError::Empty);}
        if p.kind!=kind {return Err(ClipboardError::InvalidType);}
        let n=usize::from(p.length);
        if n>out.len() {return Err(ClipboardError::TooLarge);}
        out[..n].copy_from_slice(&p.bytes[..n]);Ok(n)
    }
    // ------------------------=
    // FUNC: epoch
    // DESC: Captures a session fence for an admitted browser input command.
    // ------------------=
    pub fn epoch(&self, owner:[u8;16]) -> u64 {if self.authorized(owner) {self.epoch}else{0}}
    // ------------------------=
    // FUNC: revoke_browser
    // DESC: Cancels pending gestures across tab focus, navigation and browser closure boundaries.
    // ------------------=
    pub fn revoke_browser(&mut self) {
        self.permits=[0;2];self.epoch=self.epoch.wrapping_add(1).max(1);
    }
    // ------------------------=
    // FUNC: grant
    // DESC: Grants one short-lived clipboard operation for a native keyboard gesture.
    // ------------------=
    pub fn grant(&mut self, epoch:u64, write:bool, now:u64) {
        if self.unlocked && epoch==self.epoch && epoch!=0 {
            self.permits[write as usize]=now.saturating_add(2000);
        }
    }
    // ------------------------=
    // FUNC: browser_read
    // DESC: Consumes a native paste grant; web scripts have no ambient clipboard read authority.
    // ------------------=
    pub fn browser_read(&mut self, out:&mut [u8], now:u64) -> Result<usize,ClipboardError> {
        let deadline=core::mem::replace(&mut self.permits[0],0);
        if !self.browser_read || deadline==0 || now>=deadline {return Err(ClipboardError::AccessDenied);}
        self.read(self.owner,ClipboardKind::Utf8Text,out,now)
    }
    // ------------------------=
    // FUNC: browser_write
    // DESC: Consumes a native copy/cut grant without accepting unsolicited website clipboard writes.
    // ------------------=
    pub fn browser_write(&mut self, bytes:&[u8], now:u64) -> Result<(),ClipboardError> {
        let deadline=core::mem::replace(&mut self.permits[1],0);
        if !self.browser_write || deadline==0 || now>=deadline {return Err(ClipboardError::AccessDenied);}
        self.write(self.owner,ClipboardKind::Utf8Text,bytes,now)
    }
}

struct Shared {
    locked: core::sync::atomic::AtomicBool,
    value: core::cell::UnsafeCell<SessionClipboard>,
}
// Only short, non-reentrant memory operations run while this lock is held.
unsafe impl Sync for Shared {}
static SHARED: Shared=Shared {locked:core::sync::atomic::AtomicBool::new(false),
    value:core::cell::UnsafeCell::new(SessionClipboard::new())};

// ------------------------=
// FUNC: with_shared
// DESC: Serializes desktop and browser-worker access without lending references outside the lock.
// ------------------=
pub fn with_shared<T>(f:impl FnOnce(&mut SessionClipboard)->T)->T {
    use core::sync::atomic::Ordering;
    while SHARED.locked.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed).is_err() {core::hint::spin_loop();}
    struct Guard;
    impl Drop for Guard {
        // ------------------------=
        // FUNC: drop
        // DESC: Releases shared clipboard ownership, including host-test unwinding.
        // ------------------=
        fn drop(&mut self) {SHARED.locked.store(false,Ordering::Release);}
    }
    let _guard=Guard;
    f(unsafe{&mut *SHARED.value.get()})
}
