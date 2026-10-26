//! Behavioral protocol tests only. Synthetic reports are not facial recognition evidence.
#[path="../kernel/runtime/face_setup.rs"]
mod face_setup;
use face_setup::{Setup,Stage,Error,wire as p};

// ------------------------=
// FUNC: login_cannot_skip_comparisons_or_export_enrollment
// DESC: Tests the private login command independently from enrollment, including stale reports, early success and payload erasure.
// ------------------=
#[test]
fn login_cannot_skip_comparisons_or_export_enrollment() {
    let mut setup=Setup::new();setup.begin_login(7,[3;32],&[1;p::ENROLLMENT_BYTES],100).unwrap();
    let id=setup.control().command_id;
    assert_eq!(setup.control().command,p::LOGIN);
    assert!(setup.login_template().is_some());
    assert_eq!(setup.accept(p::Report::new(id,p::PASSED,0,3,3),101),Err(Error::Protocol));
    assert_eq!(setup.accept(p::Report::new(id+1,p::VERIFYING,0,3,1),101),Err(Error::Stale));
    assert_eq!(setup.write_enrollment(id,0,&[1;4],102),Err(Error::Stale));
    for n in 1..=3 {setup.accept(p::Report::new(id,if n==3 {p::PASSED}else{p::VERIFYING},0,3,n),102+n as u64).unwrap();}
    assert_eq!(setup.stage(),Stage::Passed);assert!(setup.login_template().is_none());assert!(setup.enrollment().is_none());assert!(!setup.saved());
    setup.cancel();assert_eq!(setup.stage(),Stage::Cancelled);
}

// ------------------------=
// FUNC: accepted_photos_survive_preview_and_retry
// DESC: Exercises all three captures, rejected side-view retries and cancellation with distinct actual RGB payloads.
// ------------------=
#[test]
fn accepted_photos_survive_preview_and_retry() {
    let mut setup=start();
    for index in 0..3 {
        let now=110+index*20;
        setup.capture(now).unwrap();let id=setup.control().command_id;
        assert_eq!(setup.capture(now+1),Err(Error::Busy));
        for offset in (0..face_setup::PREVIEW_BYTES).step_by(3840) {
            setup.write_preview(id,offset,&[30+index as u8;3840],now+2).unwrap();
        }
        if index==1 {
            setup.accept(p::Report::new(id,p::REJECTED,204,1,0),now+3).unwrap();
            assert_eq!(setup.stage(),Stage::Photos(1));assert!(setup.photo(1).is_none());
            assert!(setup.photo(0).unwrap().iter().all(|&p|p==30));
            setup.capture(now+4).unwrap();let retry=setup.control().command_id;
            assert_eq!(setup.accept(p::Report::new(id,p::READY,0,2,0),now+5),Err(Error::Stale));
            for offset in (0..face_setup::PREVIEW_BYTES).step_by(3840) {
                setup.write_preview(retry,offset,&[31;3840],now+5).unwrap();
            }
        }
        let id=setup.control().command_id;
        setup.accept(p::Report::new(id,if index==2 {p::REVIEW}else{p::READY},0,index as u32+1,0),now+6).unwrap();
        assert!(setup.photo(index as usize).unwrap().iter().all(|&p|p==30+index as u8));
    }
    assert_eq!(setup.stage(),Stage::Review);
    setup.verify(160).unwrap();
    for index in 0..3 {assert!(setup.photo(index).is_some());}
    setup.cancel();
    for index in 0..3 {assert!(setup.photo(index).is_none());}
}

// ------------------------=
// FUNC: preparation_preview_is_not_an_enrollment
// DESC: Allows an explicit start-camera preview without counting it as a photo or enabling authentication.
// ------------------=
#[test]
fn preparation_preview_is_not_an_enrollment() {
    let mut setup=Setup::new();setup.begin(1,2,[3;32],"Everyday",100).unwrap();
    let id=setup.control().command_id;
    for row in 0..120 {setup.write_preview(id,row*480,&[17;480],101).unwrap();}
    assert!(setup.preview().is_some());assert_eq!(setup.stage(),Stage::Loading);
    setup.accept(p::Report::new(id,p::READY,0,0,0),102).unwrap();
    assert_eq!(setup.stage(),Stage::Photos(0));
    setup.cancel();assert!(setup.preview().is_none());
}

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
// FUNC: rejected_side_view_preserves_front_and_rejects_early_export
// DESC: Keeps accepted work after recoverable capture rejection and refuses enrollment transfer outside final verification.
// ------------------=
#[test]
fn rejected_side_view_preserves_front_and_rejects_early_export() {
    let mut setup=start();setup.capture(102).unwrap();
    let command=setup.control().command_id;
    setup.accept(p::Report::new(command,p::READY,0,1,0),103).unwrap();
    setup.capture(104).unwrap();let command=setup.control().command_id;
    assert_eq!(setup.write_enrollment(command,0,&[1;16],105),Err(Error::Stale));
    setup.accept(p::Report::new(command,p::REJECTED,216,1,0),105).unwrap();
    assert_eq!(setup.stage(),Stage::Photos(1));assert_eq!(setup.rejection(),Some(216));assert!(!setup.pending());
    setup.capture(106).unwrap();assert_eq!(setup.rejection(),None);
    setup.cancel();assert!(setup.enrollment().is_none());assert!(!setup.saved());
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
        if n==3 {
            assert_eq!(setup.accept(p::Report::new(command,p::PASSED,0,3,3),115),Err(Error::Protocol));
            let payload=[7;p::ENROLLMENT_BYTES];
            for offset in (0..payload.len()).step_by(4096) {
                let end=(offset+4096).min(payload.len());
                setup.write_enrollment(command,offset,&payload[offset..end],115).unwrap();
            }
        }
        setup.accept(p::Report::new(command,if n==3 {p::PASSED}else{p::VERIFYING},0,3,n),112+n as u64).unwrap();
    }
    assert_eq!(setup.stage(),Stage::Passed);assert!(!setup.pending());
    assert!(!setup.saved());assert!(setup.enrollment().is_some());
    setup.finish_save(true);assert!(setup.saved());assert!(setup.enrollment().is_none());
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

// ------------------------=
// FUNC: live_frames_are_atomic_and_never_enroll
// DESC: Streams repeated frames while idle, retains the previous complete frame during delivery, and counts only explicit captures.
// ------------------=
#[test]
fn live_frames_are_atomic_and_never_enroll() {
    let mut setup=start();let id=setup.control().command_id;
    for n in 0..15 {setup.write_preview(id,n*3840,&[11;3840],102).unwrap();}
    assert_eq!(setup.preview_revision(),1);
    for n in 0..15 {
        setup.write_preview(id,n*3840,&[22;3840],103).unwrap();
        assert!(setup.preview().unwrap().iter().all(|&v|v==if n==14 {22}else{11}));
    }
    assert_eq!(setup.preview_revision(),2);
    assert_eq!(setup.stage(),Stage::Photos(0));assert!(!setup.pending());
    setup.write_preview(id,0,&[33;3840],104).unwrap();
    setup.capture(105).unwrap();let capture=setup.control().command_id;
    assert_eq!(setup.write_preview(id,3840,&[33;3840],106),Err(Error::Stale));
    assert!(setup.preview().unwrap().iter().all(|&v|v==22));
    for n in 0..15 {setup.write_preview(capture,n*3840,&[44;3840],107).unwrap();}
    assert_eq!(setup.stage(),Stage::Photos(0));
    setup.accept(p::Report::new(capture,p::READY,0,1,0),108).unwrap();
    assert_eq!(setup.stage(),Stage::Photos(1));
    setup.cancel();assert!(setup.preview().is_none());
    assert_eq!(setup.write_preview(capture,0,&[55;3840],109),Err(Error::Stale));
}

// ------------------------=
// FUNC: preview_failure_is_terminal_and_clears_pixels
// DESC: Surfaces a disconnected idle preview without permitting stale or successful unsolicited enrollment reports.
// ------------------=
#[test]
fn preview_failure_is_terminal_and_clears_pixels() {
    let mut setup=start();let id=setup.control().command_id;
    assert_eq!(setup.accept(p::Report::new(id,p::READY,0,1,0),102),Err(Error::Stale));
    for n in 0..15 {setup.write_preview(id,n*3840,&[11;3840],103).unwrap();}
    setup.accept(p::Report::new(id,p::FAILED,104,0,0),104).unwrap();
    assert_eq!(setup.stage(),Stage::Failed(104));assert!(setup.preview().is_none());
    assert_eq!(setup.write_preview(id,0,&[11;3840],105),Err(Error::Stale));
}
