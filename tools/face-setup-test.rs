//! Behavioral protocol tests only. Synthetic reports are not facial recognition evidence.
#[path="../kernel/runtime/face_setup.rs"]
mod face_setup;
use face_setup::{Setup,Stage,Error,wire as p};

// ------------------------=
// FUNC: start
// DESC: Creates a fresh typed workflow without a camera, credentials, or login authority.
// ------------------=
fn start()->Setup {
    let mut setup=Setup::new();setup.begin(1,2,[3;32],"Everyday",100).unwrap();
    let command=setup.control().command_id;
    setup.accept(p::Report::new(command,p::READY,0,0,0),101).unwrap();setup
}
// ------------------------=
// FUNC: sequential_photos_then_fresh_verification
// DESC: Requires all three captures and all three separate verification reports before showing a passed comparison.
// ------------------=
#[test]
fn sequential_photos_then_fresh_verification() {
    let mut setup=start();let lease=setup.control().lease;
    for count in 0..3 {
        setup.capture(102+count).unwrap();let command=setup.control().command_id;
        assert_eq!(setup.capture(102+count),Err(Error::Busy));
        setup.accept(p::Report::new(command,p::PROCESSING,0,count as u32,0),103+count).unwrap();
        setup.accept(p::Report::new(command,if count==2 {p::REVIEW}else{p::READY},0,count as u32+1,0),104+count).unwrap();
    }
    assert_eq!(setup.stage(),Stage::Review);setup.verify(110).unwrap();
    assert_ne!(lease,setup.control().lease);let command=setup.control().command_id;
    assert_eq!(setup.accept(p::Report::new(command,p::PASSED,0,3,3),111),Err(Error::Protocol));
    for n in 1..=3 {
        setup.accept(p::Report::new(command,if n==3 {p::PASSED}else{p::VERIFYING},0,3,n),112+n as u64).unwrap();
    }
    assert_eq!(setup.stage(),Stage::Passed);assert!(!setup.pending());
}
// ------------------------=
// FUNC: stale_and_cancelled_reports_cannot_complete
// DESC: Rejects replayed and cancelled commands without mutating verified progress.
// ------------------=
#[test]
fn stale_and_cancelled_reports_cannot_complete() {
    let mut setup=start();setup.capture(102).unwrap();let id=setup.control().command_id;
    assert_eq!(setup.accept(p::Report::new(id-1,p::READY,0,1,0),103),Err(Error::Stale));
    setup.cancel();assert_eq!(setup.accept(p::Report::new(id,p::READY,0,1,0),104),Err(Error::Stale));
    assert_eq!(setup.stage(),Stage::Cancelled);assert!(setup.preview().is_none());
}
// ------------------------=
// FUNC: partial_preview_is_never_visible
// DESC: Checks exact thumbnail publication, sequential writes and privacy erasure on cancellation.
// ------------------=
#[test]
fn partial_preview_is_never_visible() {
    let mut setup=start();setup.capture(102).unwrap();let id=setup.control().command_id;
    let row=[71u8;480];
    assert_eq!(setup.write_preview(id,480,&row,103),Err(Error::Invalid));
    for n in 0..120 {
        setup.write_preview(id,n*480,&row,103).unwrap();
        assert_eq!(setup.preview().is_some(),n==119);
    }
    assert_eq!(setup.preview().unwrap().len(),160*120*3);
    assert!(setup.preview().unwrap().iter().all(|&v|v==71));
    setup.cancel();assert!(setup.preview().is_none());
}
// ------------------------=
// FUNC: command_timeout_cancels_and_wipes
// DESC: Expires unresponsive work without waiting for another input frame or report.
// ------------------=
#[test]
fn command_timeout_cancels_and_wipes() {
    let mut setup=start();setup.capture(102).unwrap();
    assert_eq!(setup.tick(60_102),Err(Error::Expired));
    assert_eq!(setup.stage(),Stage::Failed(111));assert_eq!(setup.control().command,p::CANCEL);
    assert!(!setup.pending());assert!(setup.preview().is_none());
}
