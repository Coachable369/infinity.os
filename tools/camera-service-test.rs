#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
// ------------------------=
// FUNC: output_text
// DESC: Provides a diagnostic sink for the host-only service-boundary harness.
// ------------------=
fn output_text(_: &[u8]) {}

// ------------------------=
// FUNC: main
// DESC: Tests real captured JPEG delivery through native page-table checks, owner isolation, revocation and cancellation.
// ------------------=
fn main() {
    use runtime::{camera::*, capability::*, execution::SecurityIdentity, iop::*, native_memory::*};
    use std::alloc::{alloc_zeroed,dealloc,Layout};
    let jpeg=std::fs::read(std::env::args().nth(1).expect("real guest JPEG path required")).unwrap();
    assert!(jpeg.len()>4 && jpeg.len()<MAX_FRAME_BYTES);
    let layout=Layout::from_size_align(4*1024*1024,4096).unwrap();
    let arena=unsafe {alloc_zeroed(layout)};
    assert!(!arena.is_null());
    let mut memory=NativeMemory::empty();
    memory.initialize(arena as u64,layout.size() as u64).unwrap();
    let mut tables=X86AddressSpace::new(&mut memory,11).unwrap();
    let page=memory.allocate(1,11).unwrap();
    let address=0x1_0000_0000;
    let rw=PagePermissions {user:true,writable:true,executable:false};
    tables.map_range(&mut memory,address,page.physical_address,4096,rw).unwrap();
    let owner=SecurityIdentity([11;16]);
    let stranger=SecurityIdentity([12;16]);
    let mut caps=CapabilityManager::new();
    let cap=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(10),0).unwrap();
    let request=|op,caller| IopMessage::request(op,1,caller,cap,5,0,&[]).unwrap();
    let mut storage=vec![0xa5;MAX_FRAME_BYTES];
    {
        let mut camera=Camera::new(&mut storage).unwrap();
        assert_eq!(camera.start(&request(OperationId::CameraCaptureStart,owner),stranger,&caps,0),Err(Error::Invalid));
        camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,0).unwrap();
        assert_eq!(camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,0),Err(Error::Busy));
        let (sequence,length)=camera.publish(&jpeg,&caps,0).unwrap();
        assert_eq!(length,jpeg.len());
        let read=request(OperationId::CameraFrameRead,owner);
        assert_eq!(camera.read_to_user(&read,stranger,&caps,0,sequence,0,address,100,&mut tables),Err(Error::Invalid));
        assert_eq!(camera.read_to_user(&read,owner,&caps,0,sequence,0,address+4090,100,&mut tables),Err(Error::Memory));
        let mut before=[0;100]; copy_from_user(&tables,address,&mut before).unwrap(); assert_eq!(before,[0;100]);
        let mut copy=Vec::new();
        assert_eq!(camera.metadata(owner,cap,&caps,0).unwrap(),(sequence,length));
        for offset in (0..jpeg.len()).step_by(MAX_READ_BYTES) {
            // A new physical frame can arrive at every scheduler yield. The
            // selected snapshot must remain stable until its last page lands.
            let mut next=jpeg.clone();next[2]^=1;
            assert_eq!(camera.publish(&next,&caps,0).unwrap(),(sequence,length));
            let count=(jpeg.len()-offset).min(MAX_READ_BYTES);
            assert_eq!(camera.read_to_user(&read,owner,&caps,0,sequence,offset,address,count,&mut tables),Ok(count));
            let mut chunk=vec![0;count]; copy_from_user(&tables,address,&mut chunk).unwrap(); copy.extend(chunk);
        }
        assert_eq!(copy,jpeg);
        // A polling reader must not immediately pin its consumed snapshot again.
        for _ in 0..64 {
            assert_eq!(camera.metadata(owner,cap,&caps,0),Err(Error::Unavailable));
            assert_eq!(camera.read_to_user(&read,owner,&caps,0,sequence,0,address,1,&mut tables),Err(Error::Stale));
        }
        let (next_sequence,_)=camera.publish(&jpeg,&caps,0).unwrap();
        assert!(next_sequence>sequence);
        let supervisor_address=address+8192;
        tables.map_range(&mut memory,supervisor_address,page.physical_address,4096,PagePermissions {user:false,..rw}).unwrap();
        assert_eq!(camera.read_to_user(&read,owner,&caps,0,next_sequence,0,supervisor_address,1,&mut tables),Err(Error::Memory));
        camera.publish(&jpeg,&caps,1).unwrap();
        assert_eq!(camera.read_to_user(&read,owner,&caps,1,sequence,0,address,1,&mut tables),Err(Error::Stale));
        camera.stop(&request(OperationId::CameraCaptureStop,owner),owner).unwrap();
        assert_eq!(camera.authorized(&caps,1),Err(Error::Unavailable));
        camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,1).unwrap();
        camera.publish(&jpeg,&caps,1).unwrap();
        assert_eq!(camera.authorized(&caps,5),Err(Error::Expired));
        camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,1).unwrap();
        camera.publish(&jpeg,&caps,1).unwrap();
        camera.metadata(owner,cap,&caps,1).unwrap();
        caps.revoke(cap).unwrap();
        assert_eq!(camera.authorized(&caps,2),Err(Error::Denied));
        assert_eq!(camera.metadata(owner,cap,&caps,2),Err(Error::Unavailable));
        let extended=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(30),0).unwrap();
        let start=IopMessage::request(OperationId::CameraCaptureStart,1,owner,extended,15,0,&[]).unwrap();
        camera.start(&start,owner,&caps,0).unwrap();
        camera.publish(&jpeg,&caps,6).unwrap();
        assert_eq!(camera.metadata(owner,extended,&caps,6).unwrap().1,jpeg.len());
        assert_eq!(camera.authorized(&caps,15),Err(Error::Expired));
    }
    assert!(storage.iter().all(|byte|*byte==0));
    transport_contract(&jpeg,&mut tables,address,owner);
    uvc_shutdown_recovery(&mut tables,address,owner);
    tables.release(&mut memory).unwrap(); memory.release(page).unwrap();
    unsafe {dealloc(arena,layout)};
}

// ------------------------=
// FUNC: uvc_shutdown_recovery
// DESC: Exercises the production UVC adapter through failed native shutdowns, bounded recovery and a new authorized frame.
// ------------------=
fn uvc_shutdown_recovery(tables:&mut impl runtime::native_memory::PageTable,address:u64,owner:runtime::execution::SecurityIdentity) {
    use runtime::{camera::{*,transport::*},camera_uvc::Uvc,capability::*,iop::*,native_memory::copy_from_user};
    use std::cell::RefCell;
    #[derive(Default)]
    struct Usb {starts:usize,stops:usize,frames:u8,stop_errors:usize}
    thread_local! {static USB:RefCell<Usb>=RefCell::new(Usb::default());}
    // ------------------------=
    // FUNC: exchange
    // DESC: Models the native USB operation contract with valid negotiated parameters and eight failed shutdown acknowledgments.
    // ------------------=
    unsafe extern "efiapi" fn exchange(operation:u64,bytes:*mut u8,length:*mut u64)->i64 {
        USB.with(|usb| {
            let mut usb=usb.borrow_mut();
            match operation {
                0=>{usb.starts+=1;0},
                1|3=>{assert_eq!(unsafe{*length},26);0},
                2=>{
                    let probe=unsafe{std::slice::from_raw_parts_mut(bytes,26)};
                    probe[18..22].copy_from_slice(&6u32.to_le_bytes());
                    probe[22..26].copy_from_slice(&65536u32.to_le_bytes());
                    0
                },
                4=>{
                    usb.frames+=1;
                    let payload=unsafe{std::slice::from_raw_parts_mut(bytes,8)};
                    payload.copy_from_slice(&[2,2,0xff,0xd8,usb.frames,0,0xff,0xd9]);
                    unsafe{*length=8;}0
                },
                5=>{
                    usb.stops+=1;
                    if usb.stop_errors>0 {usb.stop_errors-=1;-1}else{0}
                },
                _=>panic!("invalid native USB operation"),
            }
        })
    }
    USB.with(|usb|*usb.borrow_mut()=Usb{stop_errors:8,..Usb::default()});
    let mut caps=CapabilityManager::new();
    let cap=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(10),0).unwrap();
    let request=|operation|IopMessage::request(operation,1,owner,cap,5,0,&[]).unwrap();
    let start=request(OperationId::CameraCaptureStart);
    let read=request(OperationId::CameraFrameRead);
    let mut storage=vec![0;MAX_FRAME_BYTES];
    let mut frame=vec![0;MAX_FRAME_BYTES];
    let mut scratch=vec![0;MAX_FRAME_BYTES];
    let mut packet=Box::new([0;65536]);
    let mut transport=Transport::new(&mut storage,Uvc::new(exchange,&mut packet,&mut frame)).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    for now in 0..5 {transport.tick(&caps,0,now,&mut scratch);}
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Ok(16));
    transport.cancel();
    for attempt in 0..8 {
        transport.tick(&caps,0,5+attempt*50,&mut scratch);
        assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Busy));
    }
    assert_eq!(transport.state(),State::Faulted);
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Err(Error::Unavailable));
    for now in 356..605 {
        transport.tick(&caps,0,now,&mut scratch);
        assert!(!transport.retry_shutdown(now));
    }
    USB.with(|usb| {let usb=usb.borrow();assert_eq!((usb.starts,usb.stops,usb.frames),(1,8,1));});
    assert!(transport.retry_shutdown(605));
    assert!(!transport.retry_shutdown(606));
    transport.tick(&caps,0,606,&mut scratch);
    assert_eq!(transport.state(),State::Idle);
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    for now in 607..612 {transport.tick(&caps,0,now,&mut scratch);}
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Ok(16));
    let mut metadata=[0;16];copy_from_user(tables,address,&mut metadata).unwrap();
    let sequence=u64::from_le_bytes(metadata[..8].try_into().unwrap());
    assert_eq!(u64::from_le_bytes(metadata[8..].try_into().unwrap()),6);
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[sequence,0,address,6,0,0],tables),Ok(6));
    let mut bytes=[0;6];copy_from_user(tables,address,&mut bytes).unwrap();
    assert_eq!(bytes,[0xff,0xd8,2,0,0xff,0xd9]);
    USB.with(|usb| {let usb=usb.borrow();assert_eq!((usb.starts,usb.stops,usb.frames),(2,9,2));});
    transport.cancel();transport.tick(&caps,0,612,&mut scratch);drop(transport);
    assert!(storage.iter().chain(frame.iter()).chain(packet.iter()).chain(scratch.iter()).all(|byte|*byte==0));
}

// ------------------------=
// FUNC: transport_contract
// DESC: Tests lifecycle and authenticated dispatch with a scripted backend, not hardware capture evidence.
// ------------------=
fn transport_contract(jpeg:&[u8],tables:&mut impl runtime::native_memory::PageTable,address:u64,owner:runtime::execution::SecurityIdentity) {
    use runtime::{camera::{*,transport::*},capability::*,iop::*,native_memory::copy_from_user};
    use std::{rc::Rc,cell::RefCell};
    #[derive(Default)]
    struct Counters {starts:usize,stops:usize,polls:usize,disconnected:bool,stop_pending:bool,stop_errors:usize}
    struct Backend {c:Rc<RefCell<Counters>>,jpeg:Vec<u8>}
    impl Driver for Backend {
        // ------------------------=
        // FUNC: start
        // DESC: Records a scripted successful discovery, never accessing real hardware.
        // ------------------=
        fn start(&mut self)->Result<Attach,()> {self.c.borrow_mut().starts+=1;Ok(Attach::Ready)}
        // ------------------------=
        // FUNC: poll
        // DESC: Supplies fixture bytes or a disconnect event for lifecycle testing only.
        // ------------------=
        fn poll(&mut self,scratch:&mut [u8])->Transfer {
            let mut c=self.c.borrow_mut();c.polls+=1;
            if c.disconnected {Transfer::Disconnected} else {scratch[..self.jpeg.len()].copy_from_slice(&self.jpeg);Transfer::Frame(self.jpeg.len())}
        }
        // ------------------------=
        // FUNC: stop
        // DESC: Models acknowledged or stuck shutdown for retry-bound assertions.
        // ------------------=
        fn stop(&mut self)->Result<bool,()> {
            let mut c=self.c.borrow_mut();c.stops+=1;
            if c.stop_errors>0 {c.stop_errors-=1;return Err(());}
            Ok(!c.stop_pending)
        }
    }
    let mut caps=CapabilityManager::new();
    let cap=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(10),0).unwrap();
    let request=|op|IopMessage::request(op,1,owner,cap,5,0,&[]).unwrap();
    let c=Rc::new(RefCell::new(Counters::default()));
    let mut storage=vec![0;MAX_FRAME_BYTES];let mut scratch=vec![0;MAX_FRAME_BYTES];
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&request(OperationId::CameraCaptureStart),owner,&caps,0,[0;6],tables).unwrap();
    transport.tick(&caps,0,0,&mut scratch);assert_eq!(transport.state(),State::Streaming);
    transport.tick(&caps,0,1,&mut scratch);assert!(scratch.iter().all(|b|*b==0));
    let read=request(OperationId::CameraFrameRead);
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Ok(16));
    let mut meta=[0;16];copy_from_user(tables,address,&mut meta).unwrap();
    assert_eq!(u64::from_le_bytes(meta[8..].try_into().unwrap()),jpeg.len() as u64);
    let seq=u64::from_le_bytes(meta[..8].try_into().unwrap());
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[seq,0,address,4,0,0],tables),Ok(4));
    let mut bytes=[0;4];copy_from_user(tables,address,&mut bytes).unwrap();assert_eq!(bytes,jpeg[..4]);
    for i in 2..102 {transport.tick(&caps,0,i,&mut scratch);}
    assert_eq!(c.borrow().polls,1,"unread/pinned snapshots must not trigger USB polling");
    for offset in (0..jpeg.len()).step_by(MAX_READ_BYTES) {
        let count=(jpeg.len()-offset).min(MAX_READ_BYTES);
        transport.dispatch(&read,owner,&caps,0,[seq,offset as u64,address,count as u64,0,0],tables).unwrap();
    }
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Err(Error::Unavailable));
    c.borrow_mut().disconnected=true;
    transport.tick(&caps,0,2,&mut scratch);assert_eq!(transport.state(),State::Stopping);
    assert_eq!(transport.dispatch(&read,owner,&caps,0,[0,0,0,0,address,0],tables),Err(Error::Unavailable));
    transport.tick(&caps,0,3,&mut scratch);assert_eq!(c.borrow().stops,1);
    transport.tick(&caps,0,252,&mut scratch);assert_eq!(c.borrow().starts,1);
    transport.tick(&caps,0,253,&mut scratch);assert_eq!(c.borrow().starts,2);
    caps.revoke(cap).unwrap();
    transport.tick(&caps,0,254,&mut scratch);assert_eq!(transport.state(),State::Idle);
    assert_eq!(c.borrow().polls,2);assert_eq!(c.borrow().stops,2);
    drop(transport);
    assert!(storage.iter().all(|b|*b==0));

    let cap=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(10),0).unwrap();
    let start=IopMessage::request(OperationId::CameraCaptureStart,1,owner,cap,5,0,&[]).unwrap();
    let c=Rc::new(RefCell::new(Counters{stop_errors:1,..Counters::default()}));
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    transport.tick(&caps,0,0,&mut scratch);
    transport.cancel();
    transport.tick(&caps,0,1,&mut scratch);
    assert_eq!(transport.state(),State::Stopping);
    assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Busy));
    for now in 2..51 {transport.tick(&caps,0,now,&mut scratch);}
    assert_eq!(c.borrow().stops,1);
    transport.tick(&caps,0,51,&mut scratch);
    assert_eq!(transport.state(),State::Idle);
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    transport.tick(&caps,0,52,&mut scratch);
    assert_eq!(transport.state(),State::Streaming);
    assert_eq!(c.borrow().starts,2);
    drop(transport);

    let c=Rc::new(RefCell::new(Counters{stop_pending:true,..Counters::default()}));
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();transport.tick(&caps,0,0,&mut scratch);
    transport.cancel();
    for i in 0..8 {transport.tick(&caps,0,i*50,&mut scratch);}
    assert_eq!(transport.state(),State::Faulted);assert_eq!(c.borrow().stops,8);assert_eq!(c.borrow().polls,0);
    assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Busy));
    assert!(!transport.retry_shutdown(599));
    assert!(transport.retry_shutdown(600));
    assert!(!transport.retry_shutdown(601));
    assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Busy));
    transport.tick(&caps,0,601,&mut scratch);
    for now in 602..651 {
        assert!(!transport.retry_shutdown(now));
        transport.tick(&caps,0,now,&mut scratch);
    }
    assert_eq!(c.borrow().stops,9);
    c.borrow_mut().stop_pending=false;
    transport.tick(&caps,0,651,&mut scratch);
    assert_eq!(transport.state(),State::Idle);
    caps.revoke(cap).unwrap();
    assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Denied));
    drop(transport);
    let cap=caps.grant(CapabilityType::CameraCapture,0,1,0,owner,owner,Some(10),0).unwrap();
    let start=IopMessage::request(OperationId::CameraCaptureStart,1,owner,cap,5,0,&[]).unwrap();
    let c=Rc::new(RefCell::new(Counters{disconnected:true,..Counters::default()}));
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    for i in 0..40 {transport.tick(&caps,0,i*250,&mut scratch);}
    assert_eq!(c.borrow().starts,3);assert_eq!(transport.state(),State::Idle);
}
