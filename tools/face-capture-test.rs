//! Behavioral harness executing the production capture loop with a bounded camera.
use std::cell::RefCell;
use std::collections::VecDeque;
#[path="../sdk/infinity-face-worker/capture.rs"] mod capture;
#[derive(Default)]
struct Camera {
    now:u64,opened:u64,starts:u32,stops:u32,sequence:u64,pinned:bool,frames:bool,command:u64,
    start_results:VecDeque<i64>,supersede_on_start:Option<u32>,
}
thread_local! {static CAMERA:RefCell<Camera>=RefCell::new(Camera::default());}
// ------------------------=
// FUNC: clock
// DESC: Supplies deterministic advancing service time to exercise deadlines.
// ------------------=
fn clock()->Result<u64,u32> {CAMERA.with(|c|{let mut c=c.borrow_mut();c.now+=10;Ok(c.now)})}
// ------------------------=
// FUNC: visit
// DESC: Models command-generation cancellation without changing camera ownership.
// ------------------=
fn visit(command:u64)->bool {CAMERA.with(|c|c.borrow().command==command)}
// ------------------------=
// FUNC: call
// DESC: Models asynchronous camera admission, command cancellation, pinned delivery and dark startup pixels after every restart.
// ------------------=
fn call(op:u64,args:[u64;6])->i64 {CAMERA.with(|cell|{
    let mut c=cell.borrow_mut();
    match op {
        0x1260=>{
            c.starts+=1;
            if c.supersede_on_start==Some(c.starts) {c.command+=1;}
            let status=c.start_results.pop_front().unwrap_or(0);
            if status==0 {c.opened=c.now;}
            status
        },
        0x1262=>{c.stops+=1;c.pinned=false;0},
        0x1261 if args[3]==0=>{
            if !c.frames {return -11;}
            assert!(!c.pinned);c.pinned=true;c.sequence+=1;c.now+=100;
            unsafe {let meta=args[4] as *mut u64;*meta=c.sequence;*meta.add(1)=8192;}16
        },
        0x1261=>{
            assert!(c.pinned);assert_eq!(args[0],c.sequence);
            let value=if c.now-c.opened<2000 {8}else{128};
            unsafe {std::slice::from_raw_parts_mut(args[2] as *mut u8,args[3] as usize).fill(value);}
            if args[1]+args[3]==8192 {c.pinned=false;}args[3] as i64
        },
        _=>panic!("unexpected operation"),
    }
})}
// ------------------------=
// FUNC: reset
// DESC: Creates an isolated camera with a valid enrollment command for each test.
// ------------------=
fn reset() {CAMERA.with(|c|*c.borrow_mut()=Camera{now:1,frames:true,command:1,..Camera::default()});}
// ------------------------=
// FUNC: preview_then_photo_preserves_exposure
// DESC: Verifies warmup drains dark frames, capture uses newer pixels and does not restart the preview camera.
// ------------------=
#[test]
fn preview_then_photo_preserves_exposure() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    let preview=capture::capture(1,&mut pixels,&mut stream,true).unwrap();
    assert!(pixels.iter().all(|v|*v==128));
    CAMERA.with(|c|c.borrow_mut().command=2);
    let photo=capture::capture(2,&mut pixels,&mut stream,false).unwrap();
    assert!(photo.1>preview.1);assert!(pixels.iter().all(|v|*v==128));
    CAMERA.with(|c|{let c=c.borrow();assert_eq!(c.starts,1);assert_eq!(c.stops,0);assert!(!c.pinned);});
}

// ------------------------=
// FUNC: login_keeps_exposure_between_comparisons
// DESC: Three fresh comparison photos reuse one settled stream and remove repeated warmup without admitting an old frame.
// ------------------=
#[test]
fn login_keeps_exposure_between_comparisons() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    capture::open(1,&mut stream).unwrap();
    // Model preparation runs with the sensor already open.
    CAMERA.with(|c|c.borrow_mut().now+=2100);
    let ready=clock().unwrap();
    let first=capture::photo(1,&mut pixels,&mut stream,true).unwrap();
    assert!(first.2-ready<500);assert!(pixels.iter().all(|v|*v==128));
    let second=capture::photo(1,&mut pixels,&mut stream,true).unwrap();
    let third=capture::photo(1,&mut pixels,&mut stream,true).unwrap();
    assert!(second.1>first.1 && third.1>second.1);
    assert!(third.2-first.2<1000);
    assert!(pixels.iter().all(|v|*v==128));
    capture::stop(&mut stream);
    CAMERA.with(|c|{let c=c.borrow();assert_eq!(c.starts,1);assert_eq!(c.stops,1);});
}
// ------------------------=
// FUNC: lease_renewal_settles_again
// DESC: Verifies bounded lease renewal cannot return a dark first frame.
// ------------------=
#[test]
fn lease_renewal_settles_again() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    capture::capture(1,&mut pixels,&mut stream,true).unwrap();
    CAMERA.with(|c|c.borrow_mut().now+=9000);
    capture::capture(1,&mut pixels,&mut stream,false).unwrap();
    assert!(pixels.iter().all(|v|*v==128));
    CAMERA.with(|c|{let c=c.borrow();assert_eq!(c.starts,2);assert_eq!(c.stops,1);});
}
// ------------------------=
// FUNC: analysis_releases_camera_then_retry_warms
// DESC: Recognition has no active camera polling; the next preview still waits for settled exposure.
// ------------------=
#[test]
fn analysis_releases_camera_then_retry_warms() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    capture::capture(1,&mut pixels,&mut stream,false).unwrap();
    capture::stop(&mut stream);capture::stop(&mut stream);
    CAMERA.with(|c|assert_eq!(c.borrow().stops,1));
    capture::capture(1,&mut pixels,&mut stream,true).unwrap();
    assert!(pixels.iter().all(|v|*v==128));
    CAMERA.with(|c|assert_eq!(c.borrow().starts,2));
}

// ------------------------=
// FUNC: asynchronous_reopen_waits_then_admits_fresh_photo
// DESC: A completed photo survives bounded shutdown/admission waits; the next capture settles again and receives a newer enrollment ticket.
// ------------------=
#[test]
fn asynchronous_reopen_waits_then_admits_fresh_photo() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    let first=capture::photo(1,&mut pixels,&mut stream,false).unwrap();
    capture::stop(&mut stream);
    CAMERA.with(|c|{
        let mut c=c.borrow_mut();c.command=2;c.sequence=0;
        c.start_results=VecDeque::from([-16,-11,-16,-11]);
    });
    pixels.fill(0xa5);
    let next=capture::photo(2,&mut pixels,&mut stream,false).unwrap();
    assert!(next.1>first.1);assert!(next.2>first.2 && next.2-first.2<10_000);
    assert!(pixels.iter().all(|&v|v==128));
    CAMERA.with(|c|{
        let c=c.borrow();assert_eq!(c.starts,6);assert_eq!(c.stops,1);
        assert!(c.start_results.is_empty());assert!(!c.pinned);
    });
}

// ------------------------=
// FUNC: superseded_capture_exits_during_reopen_wait
// DESC: A command cancelled while camera admission is pending returns without reading pixels or stopping a newer command; a later capture can resume.
// ------------------=
#[test]
fn superseded_capture_exits_during_reopen_wait() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    let first=capture::photo(1,&mut pixels,&mut stream,false).unwrap();
    capture::stop(&mut stream);
    let previous_sequence=CAMERA.with(|c|{
        let mut c=c.borrow_mut();c.command=2;
        c.start_results=VecDeque::from([-16,-11,0]);
        c.supersede_on_start=Some(c.starts+2);c.sequence
    });
    pixels.fill(0xa5);
    assert_eq!(capture::photo(2,&mut pixels,&mut stream,false),Err(105));
    assert!(pixels.iter().all(|&v|v==0xa5));
    CAMERA.with(|c|{
        let c=c.borrow();assert_eq!(c.starts,3);assert_eq!(c.stops,1);
        assert_eq!(c.command,3);assert_eq!(c.sequence,previous_sequence);
        assert_eq!(c.start_results.len(),1);assert!(!c.pinned);
    });
    let next=capture::photo(3,&mut pixels,&mut stream,false).unwrap();
    assert!(next.1>first.1);assert!(pixels.iter().all(|&v|v==128));
    CAMERA.with(|c|{let c=c.borrow();assert_eq!(c.starts,4);assert_eq!(c.stops,1);});
}
// ------------------------=
// FUNC: cancellation_preserves_stream_and_timeout_closes
// DESC: Verifies command supersession does not reset exposure while genuine failure stops capture within its deadline.
// ------------------=
#[test]
fn cancellation_preserves_stream_and_timeout_closes() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    capture::capture(1,&mut pixels,&mut stream,true).unwrap();
    CAMERA.with(|c|c.borrow_mut().command=2);
    assert_eq!(capture::capture(1,&mut pixels,&mut stream,true),Err(105));
    CAMERA.with(|c|{assert_eq!(c.borrow().stops,0);c.borrow_mut().frames=false;});
    assert_eq!(capture::capture(2,&mut pixels,&mut stream,false),Err(105));
    CAMERA.with(|c|assert_eq!(c.borrow().stops,1));
}

// ------------------------=
// FUNC: restarted_camera_keeps_enrollment_tickets_monotonic
// DESC: Fresh front, left and right photos remain distinct when hardware restarts its frame counter.
// ------------------=
#[test]
fn restarted_camera_keeps_enrollment_tickets_monotonic() {
    reset();let mut stream=capture::Stream::default();let mut pixels=[0;8192];
    let mut last=0;
    for _ in 0..3 {
        CAMERA.with(|c|c.borrow_mut().sequence=0);
        let photo=capture::photo(1,&mut pixels,&mut stream,false).unwrap();
        assert!(photo.1>last);last=photo.1;
        assert!(pixels.iter().all(|v|*v==128));capture::stop(&mut stream);
    }
}
