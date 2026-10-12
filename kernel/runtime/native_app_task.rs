//! Hardware-isolated application execution with bounded cooperative desktop visits.
use super::{native_cpu as cpu,native_memory::*,native_c_image::Image,
    native_process::{Process,State},execution::SecurityIdentity};
pub struct Task { process:Process<cpu::BoundContext>, space:ProcessAddressSpace<cpu::Tables> }
impl Task {
    // ------------------------=
    // FUNC: start
    // DESC: Maps a validated position-independent executable into user-only pages and supplies no kernel callback pointers.
    // ------------------=
    pub unsafe fn start(bytes:&[u8],owner:SecurityIdentity,id:u32,memory:&mut NativeMemory,
        _kernel:KernelAccessMap,controller:cpu::Controller,deadline:u64)->Result<Self,MemoryError> {
        let image=Image::parse(bytes,cpu::MACHINE).map_err(|_|MemoryError::InvalidAddress)?;
        let layout=CompilerVirtualLayout{image_base:0x100000000,heap_base:0x200000000,heap_pages:16,
            stack_guard_base:0x300000000,stack_pages:64};
        let mut space=ProcessAddressSpace::<cpu::Tables>::build(memory,id,&image,layout,cpu::private_kernel_map())?;
        let ready=Image::load_at(bytes,cpu::MACHINE,space.image_bytes_mut(),layout.image_base as usize)
            .map_err(|_|MemoryError::InvalidAddress).and_then(|_|cpu::prepare(&mut space,memory,controller));
        if let Err(error)=ready {space.release(memory)?;return Err(error);}
        let tls=if let Some(template)=image.tls {
            let storage=core::slice::from_raw_parts_mut(space.heap.physical_address as *mut u8,4096);
            match template.initialize_at(bytes,cpu::MACHINE,storage,layout.heap_base as usize) {
                Ok(tls)=>tls.thread_pointer() as u64,
                Err(_)=>{space.release(memory)?;return Err(MemoryError::InvalidAddress);}
            }
        }else{0};
        let context=cpu::Context::new(space.entry,space.stack_top,0,tls);
        let executor=cpu::BoundContext::new(context,space.page_tables.root_physical_address(),controller);
        Ok(Self{process:Process::new(executor,owner,1_000_000,Some(deadline)),space})
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Executes one preemptible slice; trusted services receive only the fixed owner and checked private page tables.
    // ------------------=
    pub fn poll(&mut self,now:u64,authorized:bool,mut service:impl FnMut(SecurityIdentity,&mut cpu::Tables,u64,[u64;6])->i64)->State {
        if !authorized {self.process.cancel();}
        let tables=&mut self.space.page_tables;
        self.process.step(now,|owner,op,args|service(owner,tables,op,args))
    }
    // ------------------------=
    // FUNC: release
    // DESC: Stops future entry and reclaims every application image, stack, heap and page-table frame.
    // ------------------=
    pub fn release(mut self,memory:&mut NativeMemory)->Result<(),MemoryError> {
        self.process.cancel();self.space.release(memory)
    }
}
