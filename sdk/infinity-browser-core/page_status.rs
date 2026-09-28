//! Honest footer states; no inferred network phases or invented percentages.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(u32)]
pub enum Status {Ready,Loading,Complete,Blocked,Busy,StartFailed,NavigationFailed,RenderFailed,ResourceFailed,
    EntropyMissing,ClockMissing,StartupFailed,SaveFailed,Corrupt,Unavailable,Full,Invalid}
// ------------------------=
// FUNC: current
// DESC: Gives durable storage and page failures precedence over completion notifications.
// ------------------=
pub fn current(favorite_error:u8,permission:u8,busy:bool,error:u32,loading:bool,has_page:bool)->Status {
    use Status::*;
    if favorite_error!=0 {return match favorite_error {1=>SaveFailed,2=>Corrupt,3=>Unavailable,4=>Full,_=>Invalid};}
    if permission!=0 {return Blocked;}if busy {return Busy;}
    if error!=0 {return match error {1=>StartFailed,2=>NavigationFailed,3=>RenderFailed,4=>ResourceFailed,
        0x102=>EntropyMissing,0x103=>ClockMissing,_=>StartupFailed};}
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
