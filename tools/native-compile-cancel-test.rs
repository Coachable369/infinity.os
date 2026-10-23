//! Exercises the production compile-job lifecycle; inert platform seams never compile or execute application source.
#[path="../kernel/runtime/native_compile_job.rs"] mod native_compile_job;
mod execution {
    #[derive(Clone,Copy,PartialEq,Eq)] pub struct SecurityIdentity(pub [u8;16]);
}
mod native_memory {
    #[derive(Clone,Copy)] pub struct KernelAccessMap;
    pub struct Memory;
    static mut MEMORY:Memory=Memory;
    // ------------------------=
    // FUNC: memory_mut
    // DESC: Supplies inert host memory state; the lifecycle test cancels before native preparation.
    // ------------------=
    pub unsafe fn memory_mut()->&'static mut Memory {&mut *(&raw mut MEMORY)}
}
mod native_cpu {
    #[derive(Clone,Copy)] pub struct Controller;
    pub const MACHINE:u16=183;
    // ------------------------=
    // FUNC: controller_from_handoff
    // DESC: Admits the single typed host fixture platform without emulating any CPU execution.
    // ------------------=
    pub fn controller_from_handoff(kind:u32,_reserved:u32,_base:u64,_auxiliary:u64)->Option<Controller> {
        (kind==1).then_some(Controller)
    }
}
mod native_compiler_package {
    pub const CLANG:&[u8]=&[];pub const LLD:&[u8]=&[];
    pub const CRT:&[u8]=&[];pub const LINK_SCRIPT:&[u8]=&[];
}
mod native_job_files {pub struct File<'a>{pub path:&'a[u8],pub bytes:&'a[u8]}}
mod native_process {#[derive(PartialEq,Eq)] pub enum State {Runnable,Exited(i32)}}
mod native_c_image {
    pub struct Image;
    impl Image {
        // ------------------------=
        // FUNC: parse
        // DESC: Rejects synthetic artifacts; this test cannot produce a false compiled executable.
        // ------------------=
        pub fn parse(_bytes:&[u8],_machine:u16)->Result<Self,()> {Err(())}
    }
}
mod native_compiler_task {
    use super::{execution::SecurityIdentity,native_cpu::Controller,native_job_files::File,
        native_memory::{KernelAccessMap,Memory},native_process::State};
    pub const COMPILE_ARGS:&[&[u8]]=&[];pub const LINK_ARGS:&[&[u8]]=&[];
    macro_rules! empty_headers {($($name:ident),*)=>{$(pub const $name:&[u8]=&[];)*};}
    empty_headers!(STDIO,STDDEF,STRING,STDLIB,STDINT,STDBOOL,STDARG,LIMITS,ERRNO,CTYPE,ASSERT,CONSOLE,CAMERA,SYSCALL,UIKIT,INTERACTION,APP,APP_API);
    pub struct Task;
    impl Task {
        // ------------------------=
        // FUNC: start
        // DESC: Rejects native preparation so this host seam cannot claim compiler execution.
        // ------------------=
        pub unsafe fn start(_bytes:&'static[u8],_args:&'static[&'static[u8]],_owner:SecurityIdentity,
            _id:u32,_session:u32,_memory:&mut Memory,_kernel:KernelAccessMap,_controller:Controller,
            _deadline:u64)->Result<Self,()> {Err(())}
        // ------------------------=
        // FUNC: release
        // DESC: Completes inert cleanup without creating or accessing native mappings.
        // ------------------=
        pub fn release(self,_memory:&mut Memory)->Result<(),()> {Ok(())}
        // ------------------------=
        // FUNC: poll
        // DESC: Cannot advance a native compiler; only production cancellation and terminal state are exercised.
        // ------------------=
        pub fn poll(&mut self,_now:u64,_authorized:bool,_memory:&mut Memory,_input:&[u8],
            _files:&[File<'_>],_output:&mut[u8],_diagnostics:&mut[u8])->State {State::Runnable}
        // ------------------------=
        // FUNC: result_lengths
        // DESC: Supplies no fabricated output artifact or compiler diagnostics.
        // ------------------=
        pub fn result_lengths(&self,_input:&[u8],_output:&mut[u8],_diagnostics:&mut[u8])->Option<(usize,usize,bool)> {None}
    }
}

// ------------------------=
// FUNC: main
// DESC: Proves explicit cancellation remains observable to the exact owner and permits a later build once the caller consumes its lease.
// ------------------=
fn main() {unsafe {
    use native_compile_job::{self as job,Status,Error};
    let owner=execution::SecurityIdentity([1;16]);let other=execution::SecurityIdentity([2;16]);
    job::initialize(native_memory::KernelAccessMap,1,0,0,0);
    assert_eq!(job::start(owner,b"int main(void){return 0;}",10),Ok(()));
    assert_eq!(job::status(owner),Status::Compile);
    assert!(!job::cancel(other));assert_eq!(job::status(owner),Status::Compile);
    assert!(job::cancel(owner));assert_eq!(job::status(owner),Status::Cancelled);
    assert!(!job::poll(other,true,11));assert_eq!(job::result(other,|_,_,_,_|()),None);
    assert!(job::poll(owner,true,11));
    assert_eq!(job::result(owner,|status,binary,_,_|(status,binary.len())),Some((Status::Cancelled,0)));
    assert!(job::poll(owner,true,12));
    assert_eq!(job::start(owner,b"int main(void){return 1;}",13),Ok(()));
    assert_eq!(job::status(owner),Status::Compile);
    assert_eq!(job::start(other,b"int main(void){return 2;}",14),Err(Error::Busy));
    assert!(job::cancel(owner));assert!(job::poll(owner,true,15));
    assert_eq!(job::start(other,b"int main(void){return 2;}",16),Ok(()));
    assert_eq!(job::status(owner),Status::Idle);assert_eq!(job::status(other),Status::Compile);
    assert!(job::cancel(other));assert!(!job::poll(owner,true,17));assert!(job::poll(other,true,17));
}}
