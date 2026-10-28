//! Behavioral harness executing the production capture loop with a bounded camera.
use std::cell::RefCell;
#[path="../sdk/infinity-face-worker/capture.rs"] mod capture;
#[derive(Default)]
struct Camera { now:u64,opened:u64,starts:u32,stops:u32,sequence:u64,pinned:bool,frames:bool,command:u64 }
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
// DESC: Models pinned paged camera delivery and dark startup pixels after every restart.
// ------------------=
fn call(op:u64,args:[u64;6])->i64 {CAMERA.with(|cell|{
    let mut c=cell.borrow_mut();
    match op {
        0x1260=>{c.starts+=1;c.opened=c.now;0},
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
    assert!(photo.1>preview.1+1);assert!(pixels.iter().all(|v|*v==128));
    CAMERA.with(|c|{let c=c.borrow();assert_eq!(c.starts,1);assert_eq!(c.stops,0);assert!(!c.pinned);});
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
