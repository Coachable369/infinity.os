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
    tables.release(&mut memory).unwrap(); memory.release(page).unwrap();
    unsafe {dealloc(arena,layout)};
}
