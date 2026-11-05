//! Honest footer states; no inferred network phases or invented percentages.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(u32)]
pub enum Status {Ready,Loading,Complete,Blocked,Busy,StartFailed,NavigationFailed,RenderFailed,ResourceFailed,
    EntropyMissing,ClockMissing,StartupFailed,SaveFailed,Corrupt,Unavailable,Full,Invalid,EngineMemory,EngineStopped}
pub const ENGINE_MEMORY: u32 = 0x200;
pub const ENGINE_STOPPED: u32 = 0x201;
// ------------------------=
// FUNC: engine_error
// DESC: Separates worker termination from page/resource error numbers so memory exhaustion never masquerades as TLS failure.
// ------------------=
pub fn engine_error(failure: u32) -> u32 {
    match failure { 0 => 0, 4 => ENGINE_MEMORY, _ => ENGINE_STOPPED }
}
// ------------------------=
// FUNC: current
// DESC: Gives durable storage and page failures precedence over completion notifications.
// ------------------=
pub fn current(favorite_error:u8,permission:u8,busy:bool,error:u32,loading:bool,has_page:bool)->Status {
    use Status::*;
    if favorite_error!=0 {return match favorite_error {1=>SaveFailed,2=>Corrupt,3=>Unavailable,4=>Full,_=>Invalid};}
    if permission!=0 {return Blocked;}if busy {return Busy;}
    if error!=0 {return match error {1=>StartFailed,2=>NavigationFailed,3=>RenderFailed,4=>ResourceFailed,
        0x102=>EntropyMissing,0x103=>ClockMissing,ENGINE_MEMORY=>EngineMemory,ENGINE_STOPPED=>EngineStopped,_=>StartupFailed};}
    if loading {Loading}else if has_page {Complete}else{Ready}
}
impl Status {
    // ------------------------=
    // FUNC: label
    // DESC: Supplies recovery guidance for the precise state known by the native browser.
    // ------------------=
    pub fn label(self)->&'static [u8] {match self {
        Self::Ready=>b"Ready",Self::Loading=>b"Loading ",Self::Complete=>b"Done",
        Self::Blocked=>b"Access blocked by Network Settings or the current session.",
        Self::Busy=>b"Browser is busy. Please retry the last input.",
        Self::StartFailed=>b"Browser could not start. Close this window and try again.",
        Self::NavigationFailed=>b"Navigation could not be accepted. Check the address and try again.",
        Self::RenderFailed=>b"Page rendering failed. Reload the page to retry.",
        Self::ResourceFailed=>b"Unable to load this page. Check the address, connection and site certificate; then reload.",
        Self::EntropyMissing=>b"Secure randomness unavailable. Enable firmware RNG or TPM 2.0 and restart.",
        Self::ClockMissing=>b"Trusted clock unavailable. Correct the firmware time and retry.",
        Self::EngineMemory=>b"Browser ran out of memory. Reload or enter another address to restart it.",
        Self::EngineStopped=>b"Browser engine stopped. Reload or enter another address to restart it.",
        Self::StartupFailed=>b"Browser unavailable. See the page above for recovery instructions.",
        Self::SaveFailed=>b"Favorite not saved. Storage commit failed; click the star to retry.",
        Self::Corrupt=>b"Favorites data is invalid. Existing saved data has not been overwritten.",
        Self::Unavailable=>b"Favorites storage unavailable. Click the star to retry.",
        Self::Full=>b"Favorites are full. Open a saved page and click its star to remove it.",
        Self::Invalid=>b"Only HTTP or HTTPS pages can be saved as favorites.",
    }}
}
#[cfg(test)] mod tests {
    use super::*;
    // ------------------------=
    // FUNC: loading_failure_and_recovery_precedence
    // DESC: Exercises navigation completion, denial, storage failure and successful retry projections.
    // ------------------=
    #[test] fn loading_failure_and_recovery_precedence() {
        assert_eq!(current(0,0,false,engine_error(4),false,true),Status::EngineMemory);
        assert_eq!(current(0,0,false,engine_error(3),false,true),Status::EngineStopped);
        assert_eq!(engine_error(0),0);
        assert_eq!(current(0,0,false,0,false,false),Status::Ready);
        assert_eq!(current(0,0,false,0,true,true),Status::Loading);
        assert_eq!(current(0,0,false,4,true,true),Status::ResourceFailed);
        assert_eq!(current(0,0,false,4,false,true),Status::ResourceFailed);
        assert_eq!(current(0,0,false,0,false,true),Status::Complete);
        assert_eq!(current(0,2,false,0,true,true),Status::Blocked);
        assert_eq!(current(1,0,false,0,false,true),Status::SaveFailed);
        assert_eq!(current(2,0,false,0,false,true),Status::Corrupt);
    }
}


#[derive(Clone, Copy)]
pub struct LoadFeedback { started: u64, frame: u64 }
impl LoadFeedback {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes pointer feedback without implying a document has loaded.
    // ------------------=
    pub const fn new() -> Self { Self { started: 0, frame: 0 } }
    // ------------------------=
    // FUNC: begin
    // DESC: Records the visible frame boundary at navigation admission.
    // ------------------=
    pub fn begin(&mut self, now: u64, frame: u64) { self.started=now; self.frame=frame; }
    // ------------------------=
    // FUNC: waiting
    // DESC: Shows transient pointer feedback only until new pixels arrive or two seconds elapse; footer completion remains engine-owned.
    // ------------------=
    pub fn waiting(&self, now: u64, frame: u64, loading: bool, input_busy: bool) -> bool {
        input_busy || (loading && frame==self.frame && now.saturating_sub(self.started)<2_000_000_000)
    }
}
#[test]
// ------------------------=
// FUNC: progressive_page_does_not_hold_pointer_hostage
// DESC: Keeps load status honest while releasing the pointer on first paint, timeout, or completion.
// ------------------=
fn progressive_page_does_not_hold_pointer_hostage() {
    let mut feedback=LoadFeedback::new();feedback.begin(10,7);
    assert!(feedback.waiting(11,7,true,false));
    assert!(!feedback.waiting(12,8,true,false));
    assert!(!feedback.waiting(2_000_000_010,7,true,false));
    assert!(!feedback.waiting(13,7,false,false));
    assert!(feedback.waiting(14,8,false,true));
    assert_eq!(current(0,0,false,0,true,true),Status::Loading);
    feedback.begin(3_000_000_000,8);
    assert!(feedback.waiting(3_000_000_001,8,true,false));
}
