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
        for offset in (0..jpeg.len()).step_by(MAX_READ_BYTES) {
            let count=(jpeg.len()-offset).min(MAX_READ_BYTES);
            assert_eq!(camera.read_to_user(&read,owner,&caps,0,sequence,offset,address,count,&mut tables),Ok(count));
            let mut chunk=vec![0;count]; copy_from_user(&tables,address,&mut chunk).unwrap(); copy.extend(chunk);
        }
        assert_eq!(copy,jpeg);
        let supervisor_address=address+8192;
        tables.map_range(&mut memory,supervisor_address,page.physical_address,4096,PagePermissions {user:false,..rw}).unwrap();
        assert_eq!(camera.read_to_user(&read,owner,&caps,0,sequence,0,supervisor_address,1,&mut tables),Err(Error::Memory));
        camera.publish(&jpeg,&caps,1).unwrap();
        assert_eq!(camera.read_to_user(&read,owner,&caps,1,sequence,0,address,1,&mut tables),Err(Error::Stale));
        camera.stop(&request(OperationId::CameraCaptureStop,owner),owner).unwrap();
        assert_eq!(camera.authorized(&caps,1),Err(Error::Unavailable));
        camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,1).unwrap();
        camera.publish(&jpeg,&caps,1).unwrap();
        assert_eq!(camera.authorized(&caps,5),Err(Error::Expired));
        camera.start(&request(OperationId::CameraCaptureStart,owner),owner,&caps,1).unwrap();
        camera.publish(&jpeg,&caps,1).unwrap();
        caps.revoke(cap).unwrap();
        assert_eq!(camera.authorized(&caps,2),Err(Error::Denied));
    }
    assert!(storage.iter().all(|byte|*byte==0));
    transport_contract(&jpeg,&mut tables,address,owner);
    tables.release(&mut memory).unwrap(); memory.release(page).unwrap();
    unsafe {dealloc(arena,layout)};
}

// ------------------------=
// FUNC: transport_contract
// DESC: Tests lifecycle and authenticated dispatch with a scripted backend, not hardware capture evidence.
// ------------------=
fn transport_contract(jpeg:&[u8],tables:&mut impl runtime::native_memory::PageTable,address:u64,owner:runtime::execution::SecurityIdentity) {
    use runtime::{camera::{*,transport::*},capability::*,iop::*,native_memory::copy_from_user};
    use std::{rc::Rc,cell::RefCell};
    #[derive(Default)]
    struct Counters {starts:usize,stops:usize,polls:usize,disconnected:bool,stop_pending:bool}
    struct Backend {c:Rc<RefCell<Counters>>,jpeg:Vec<u8>}
    impl Driver for Backend {
        // ------------------------=
        // FUNC: start
        // DESC: Records a scripted successful discovery, never accessing real hardware.
        // ------------------=
        fn start(&mut self)->Result<bool,()> {self.c.borrow_mut().starts+=1;Ok(true)}
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
        fn stop(&mut self)->Result<bool,()> {let mut c=self.c.borrow_mut();c.stops+=1;Ok(!c.stop_pending)}
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
    let c=Rc::new(RefCell::new(Counters{stop_pending:true,..Counters::default()}));
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();transport.tick(&caps,0,0,&mut scratch);
    transport.cancel();
    for i in 0..20 {transport.tick(&caps,0,i,&mut scratch);}
    assert_eq!(transport.state(),State::Faulted);assert_eq!(c.borrow().stops,8);assert_eq!(c.borrow().polls,0);
    assert_eq!(transport.dispatch(&start,owner,&caps,0,[0;6],tables),Err(Error::Busy));
    drop(transport);
    let c=Rc::new(RefCell::new(Counters{disconnected:true,..Counters::default()}));
    let mut transport=Transport::new(&mut storage,Backend{c:c.clone(),jpeg:jpeg.to_vec()}).unwrap();
    transport.dispatch(&start,owner,&caps,0,[0;6],tables).unwrap();
    for i in 0..40 {transport.tick(&caps,0,i*250,&mut scratch);}
    assert_eq!(c.borrow().starts,3);assert_eq!(transport.state(),State::Idle);
}
