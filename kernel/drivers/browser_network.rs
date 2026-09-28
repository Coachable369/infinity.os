//! Single engine-owner CPU producer; BSP exclusively runs the governed HTTPS
//! actor. Response storage remains pinned until the engine releases its handle.
use core::{cell::UnsafeCell,sync::atomic::{AtomicBool,AtomicU8,AtomicU32,AtomicU64,Ordering}};
use crate::runtime::{execution::SecurityIdentity,capability::CapabilityId};
use super::https;
#[path="../../sdk/infinity-browser-core/worker.rs"]
pub mod abi;
const COUNT:usize=16;
const FREE:u8=0;
const WRITING:u8=1;
const PENDING:u8=2;
const ACTIVE:u8=3;
const READY:u8=4;
const FAILED:u8=5;
struct Data {url:[u8;2048],length:usize,status:u32,headers:[u8;8192],head:usize,body:[u8;https::BODY],size:usize}
struct Slot {state:AtomicU8,id:AtomicU64,cancelled:AtomicBool,data:UnsafeCell<Data>}
// Engine writes only after claiming FREE. BSP writes only after acquiring
// PENDING; READY release-publishes immutable bytes until engine cancellation.
unsafe impl Sync for Slot {}
impl Slot {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded native response storage in BSS rather than on the desktop stack.
    // ------------------=
    const fn new()->Self {Self {state:AtomicU8::new(FREE),id:AtomicU64::new(0),cancelled:AtomicBool::new(false),
        data:UnsafeCell::new(Data {url:[0;2048],length:0,status:0,headers:[0;8192],head:0,body:[0;https::BODY],size:0})}}
}
static SLOTS:[Slot;COUNT]=[const {Slot::new()};COUNT];
static NEXT:AtomicU64=AtomicU64::new(1);
// Transaction-edge diagnostics only; never consulted for authorization.
#[no_mangle]
pub static INFINITY_BROWSER_NETWORK_FAILURE:AtomicU32=AtomicU32::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_NETWORK_STATUS:AtomicU32=AtomicU32::new(0);
#[no_mangle]
pub static INFINITY_BROWSER_NETWORK_COMPLETED:AtomicU64=AtomicU64::new(0);
static mut AUTHORITY:Option<(SecurityIdentity,[CapabilityId;4])>=None;
static mut CURRENT:Option<(usize,https::Ticket)>=None;

// ------------------------=
// FUNC: configure
// DESC: Installs only caller-supplied authority between sessions; never mints or broadens capabilities.
// ------------------=
/// BSP only, with the engine producer stopped while changing session ownership.
pub unsafe fn configure(owner:SecurityIdentity,capabilities:[CapabilityId;4])->bool {
    if CURRENT.is_some() || SLOTS.iter().any(|slot|slot.state.load(Ordering::Acquire)!=FREE) {return false;}
    AUTHORITY=Some((owner,capabilities));true
}
// ------------------------=
// FUNC: renew
// DESC: Replaces future-request authority only for the existing owner; active HTTPS transactions retain their original lease checks.
// ------------------=
/// BSP only. The engine never reads AUTHORITY and no response slots are changed.
pub unsafe fn renew(owner:SecurityIdentity,capabilities:[CapabilityId;4])->bool {
    if !AUTHORITY.as_ref().is_some_and(|(current,_)|*current==owner) {return false;}
    AUTHORITY=Some((owner,capabilities));true
}
// ------------------------=
// FUNC: begin
// DESC: Queues a bounded URL without touching BSP services, sockets or runtime locks.
// ------------------=
/// Called only from the single native engine owner CPU.
pub unsafe fn begin(url:&[u8])->u64 {
    if url.is_empty() || url.len()>2048 {return 0;}
    for slot in &SLOTS {
        if slot.state.compare_exchange(FREE,WRITING,Ordering::Acquire,Ordering::Relaxed).is_err() {continue;}
        let id=match NEXT.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|n|n.checked_add(1)) {
            Ok(id)=>id,Err(_)=>{slot.state.store(FREE,Ordering::Release);return 0;}
        };
        let data=&mut *slot.data.get();data.url[..url.len()].copy_from_slice(url);data.length=url.len();
        slot.id.store(id,Ordering::Relaxed);slot.cancelled.store(false,Ordering::Relaxed);
        slot.state.store(PENDING,Ordering::Release);return id;
    }
    0
}
// ------------------------=
// FUNC: poll
// DESC: Lends completed native response bytes without blocking the engine or copying on every poll.
// ------------------=
/// Engine owner only. Returned pointers remain valid until cancel(id); callers
/// must finish reading before cancellation and must not retain them afterwards.
pub unsafe fn poll(id:u64,out:&mut abi::Response)->u32 {
    let Some(slot)=SLOTS.iter().find(|slot|slot.id.load(Ordering::Relaxed)==id && id!=0) else{return 2;};
    let state=slot.state.load(Ordering::Acquire);
    if slot.cancelled.load(Ordering::Acquire) {return 2;}
    match state {
        PENDING|ACTIVE|WRITING=>0,
        READY=>{let data=&*slot.data.get();*out=abi::Response{status:data.status,headers:data.headers.as_ptr(),
            headers_length:data.head,body:data.body.as_ptr(),body_length:data.size};1},
        _=>2,
    }
}
// ------------------------=
// FUNC: cancel
// DESC: Releases a handle asynchronously; a stale generation cannot cancel another operation.
// ------------------=
/// Engine owner only, after releasing every borrowed response pointer.
pub unsafe fn cancel(id:u64) {
    if let Some(slot)=SLOTS.iter().find(|slot|slot.id.load(Ordering::Relaxed)==id && id!=0) {
        slot.cancelled.store(true,Ordering::Release);
    }
}
// ------------------------=
// FUNC: cancel_all
// DESC: Revokes all pending engine handles on fatal termination without entering BSP services.
// ------------------=
/// Engine owner only, after permanently abandoning all borrowed response data.
pub unsafe fn cancel_all() {
    for slot in &SLOTS {
        if slot.state.load(Ordering::Acquire)!=FREE {slot.cancelled.store(true,Ordering::Release);}
    }
}
// ------------------------=
// FUNC: finish
// DESC: Copies one validated response and strips already-decoded hop-by-hop framing before publication.
// ------------------=
unsafe fn finish(slot:&Slot,response:https::Response)->bool {
    if response.header_length>response.headers.len() || response.length>response.bytes.len() {return false;}
    let Ok(headers)=crate::http_transport::response::Headers::parse(&response.headers[..response.header_length]) else{return false;};
    let data=&mut *slot.data.get();data.head=0;
    for (name,value) in headers.iter() {
        if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("connection") {continue;}
        let Some(end)=data.head.checked_add(name.len()+value.len()+4).filter(|end|*end<=data.headers.len()) else{return false;};
        for part in [name.as_bytes(),b": ",value,b"\r\n"] {
            data.headers[data.head..data.head+part.len()].copy_from_slice(part);data.head+=part.len();
        }
        debug_assert_eq!(data.head,end);
    }
    data.body[..response.length].copy_from_slice(&response.bytes[..response.length]);
    data.size=response.length;data.status=response.status as u32;true
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: lease_renewal_preserves_session_ownership
    // DESC: Checks that renewal updates only the current owner's future requests and rejects cross-session replacement.
    // ------------------=
    #[test]
    fn lease_renewal_preserves_session_ownership() { unsafe {
        let owner=SecurityIdentity([7;16]);
        AUTHORITY=Some((owner,[1,2,3,4]));
        assert!(!renew(SecurityIdentity([8;16]),[5,6,7,8]));
        assert_eq!(AUTHORITY.unwrap().1,[1,2,3,4]);
        assert!(renew(owner,[5,6,7,8]));
        assert_eq!(AUTHORITY.unwrap().1,[5,6,7,8]);
        AUTHORITY=None;
    }}
    // ------------------------=
    // FUNC: response_handoff_preserves_content_not_transfer_framing
    // DESC: Checks actual HTTP metadata transformation and exact decoded binary-body ownership.
    // ------------------=
    #[test]
    fn response_handoff_preserves_content_not_transfer_framing() {
        static SLOT:Slot=Slot::new();
        let slot=&SLOT;
        let mut response=https::Response{status:200,length:4,bytes:https::ResponseBody::claim().unwrap(),header_length:0,headers:[0;8192]};
        response.bytes[..4].copy_from_slice(&[0,128,255,9]);
        let head=b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        response.headers[..head.len()].copy_from_slice(head);response.header_length=head.len();
        assert!(unsafe {finish(&slot,response)});
        let data=unsafe {&*slot.data.get()};
        assert_eq!(data.status,200);assert_eq!(&data.body[..data.size],&[0,128,255,9]);
        assert_eq!(&data.headers[..data.head],b"Content-Type: image/png\r\n");
        let malformed=https::Response{status:200,length:https::BODY+1,bytes:https::ResponseBody::claim().unwrap(),header_length:0,headers:[0;8192]};
        assert!(!unsafe {finish(&slot,malformed)});
        let mut large=https::Response{status:200,length:512*1024,bytes:https::ResponseBody::claim().unwrap(),header_length:0,headers:[0;8192]};
        assert!(https::ResponseBody::claim().is_none());
        for (i,b) in large.bytes[..large.length].iter_mut().enumerate(){*b=(i%251) as u8;}
        large.headers[..head.len()].copy_from_slice(head);large.header_length=head.len();
        assert!(unsafe {finish(slot,large)});
        let data=unsafe {&*slot.data.get()};
        assert_eq!(data.size,512*1024);
        assert!(data.body[..data.size].iter().enumerate().all(|(i,b)|*b==(i%251) as u8));
        let released=https::ResponseBody::claim().unwrap();assert!(released.iter().all(|b|*b==0));
    }
}
// ------------------------=
// FUNC: pump
// DESC: Advances at most one governed request per BSP tick; engine callbacks never enter runtime services.
// ------------------=
/// BSP only, outside any outstanding Runtime or HTTPS actor borrow.
pub unsafe fn pump() {
    let Some((owner,mut caps))=AUTHORITY else {
        for slot in &SLOTS {
            let state=slot.state.load(Ordering::Acquire);
            if matches!(state,PENDING|READY|FAILED) {
                slot.state.store(if slot.cancelled.load(Ordering::Acquire){FREE}else{FAILED},Ordering::Release);
            }
        }
        return;
    };
    let allowed=crate::runtime::with_runtime(|runtime| {
        (0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|runtime.identity.session_nth(i))
            .any(|s|s.id.0==owner.0 && s.state==crate::runtime::identity::SessionState::Active)
            && runtime.network.profiles.active().is_some_and(|p|p.interfaces_enabled && p.internet_allowed && p.resolver_enabled)
    }).unwrap_or(false);
    if !allowed {
        if let Some((_,ticket))=CURRENT {let _=https::cancel_browser(owner,ticket);}
        for slot in &SLOTS {
            if slot.state.load(Ordering::Acquire)==PENDING {slot.state.store(FAILED,Ordering::Release);}
        }
    }
    if let Some((index,ticket))=CURRENT {
        let slot=&SLOTS[index];
        if slot.cancelled.load(Ordering::Acquire) {let _=https::cancel_browser(owner,ticket);}
        match https::take_browser(owner,ticket) {
            Ok(None)|Err(https::Failure::Busy)=>return,
            result=>{
                CURRENT=None;
                if slot.cancelled.load(Ordering::Acquire) {slot.state.store(FREE,Ordering::Release);}
                else {
                    let good=match result {
                        Ok(Some(Ok(response)))=>{
                            INFINITY_BROWSER_NETWORK_STATUS.store(response.status as u32,Ordering::Release);
                            INFINITY_BROWSER_NETWORK_COMPLETED.fetch_add(1,Ordering::Release);
                            finish(slot,response)
                        },
                        Ok(Some(Err(error)))|Err(error)=>{
                            INFINITY_BROWSER_NETWORK_FAILURE.store(error as u32+1,Ordering::Release);false
                        },
                        _=>false
                    };
                    slot.state.store(if good{READY}else{FAILED},Ordering::Release);
                }
            }
        }
    }
    // Releasing completed/cancelled handles never requires network permission.
    // Otherwise disabling Internet retains slots and prevents reconfiguration.
    for slot in &SLOTS {
        if matches!(slot.state.load(Ordering::Acquire),PENDING|READY|FAILED)
            && slot.cancelled.load(Ordering::Acquire) {
            slot.state.store(FREE,Ordering::Release);
        }
    }
    if !allowed {return;}
    if SLOTS.iter().any(|slot|slot.state.load(Ordering::Acquire)==PENDING) {
        let now=crate::runtime::node_client::clock();
        let refreshed=now.and_then(|now|crate::runtime::with_runtime(|runtime|
            runtime.network.browser_authority(&mut runtime.capabilities,owner,true,now)).flatten());
        let Some(refreshed)=refreshed else {return;};
        caps=refreshed;AUTHORITY=Some((owner,caps));
    }
    for (index,slot) in SLOTS.iter().enumerate() {
        let state=slot.state.load(Ordering::Acquire);
        if matches!(state,PENDING|READY|FAILED) && slot.cancelled.load(Ordering::Acquire) {
            slot.state.store(FREE,Ordering::Release);continue;
        }
        if state!=PENDING {continue;}
        let data=&*slot.data.get();
        let parsed=core::str::from_utf8(&data.url[..data.length]).ok().and_then(|url|
            crate::http_transport::geturl::parse(core::iter::once(url)).ok());
        let Some(crate::http_transport::geturl::Command::Get(options))=parsed else {
            slot.state.store(FAILED,Ordering::Release);continue;
        };
        match https::get_browser(owner,caps[0],caps[1],caps[2],caps[3],options.host,443,options.target) {
            Ok(ticket)=>{INFINITY_BROWSER_NETWORK_FAILURE.store(0,Ordering::Release);
                slot.state.store(ACTIVE,Ordering::Release);CURRENT=Some((index,ticket));},
            Err(https::Failure::Busy)=>{},
            Err(error)=>{INFINITY_BROWSER_NETWORK_FAILURE.store(error as u32+1,Ordering::Release);
                slot.state.store(FAILED,Ordering::Release);},
        }
        break;
    }
}
