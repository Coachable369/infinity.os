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
const CHUNK_READY:u8=6;
const CHUNK_READING:u8=7;
struct Data {url:[u8;8192],length:usize,request_headers:[u8;8192],request_head:usize,
    method:[u8;8],method_length:usize,request_body:[u8;65536],request_length:usize,
    status:u32,headers:[u8;8192],head:usize,body:[u8;https::BODY],size:usize}
struct Slot {state:AtomicU8,id:AtomicU64,cancelled:AtomicBool,data:UnsafeCell<Data>}
// Engine writes only after claiming FREE. BSP writes only after acquiring
// PENDING; CHUNK_READY publishes immutable bytes until the engine's next poll
// acknowledges CHUNK_READING. Terminal storage is released by cancellation.
unsafe impl Sync for Slot {}
impl Slot {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded native response storage in BSS rather than on the desktop stack.
    // ------------------=
    const fn new()->Self {Self {state:AtomicU8::new(FREE),id:AtomicU64::new(0),cancelled:AtomicBool::new(false),
        data:UnsafeCell::new(Data {url:[0;8192],length:0,request_headers:[0;8192],request_head:0,
            method:[0;8],method_length:0,request_body:[0;65536],request_length:0,
            status:0,headers:[0;8192],head:0,body:[0;https::BODY],size:0})}}
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
// FUNC: transport_ready
// DESC: Requires DHCP or static configuration to have published a usable IPv4 source, resolver and default route before consuming a queued navigation.
// ------------------=
fn transport_ready()->bool {
    crate::runtime::with_runtime(|runtime| runtime.network.browser_transport_ready())
        .unwrap_or(false)
}

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
pub unsafe fn begin(url:&[u8],headers:&[u8])->u64 {
    begin_request(url,headers,b"GET",&[])
}
// ------------------------=
// FUNC: begin_request
// DESC: Claims bounded storage only after validating request method, framing ownership, and body limits.
// ------------------=
pub unsafe fn begin_request(url:&[u8],headers:&[u8],method:&[u8],body:&[u8])->u64 {
    if url.is_empty() || url.len()>8192 || crate::http_transport::request::validate_headers(headers).is_err() {return 0;}
    let Ok(name)=core::str::from_utf8(method) else {return 0;};
    if crate::http_transport::request::validate_method(name).is_err() || method.len()>8 || body.len()>65536
        || (!body.is_empty() && matches!(name,"GET"|"HEAD")) {return 0;}
    for slot in &SLOTS {
        if slot.state.compare_exchange(FREE,WRITING,Ordering::Acquire,Ordering::Relaxed).is_err() {continue;}
        let id=match NEXT.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|n|n.checked_add(1)) {
            Ok(id)=>id,Err(_)=>{slot.state.store(FREE,Ordering::Release);return 0;}
        };
        let data=&mut *slot.data.get();data.url[..url.len()].copy_from_slice(url);data.length=url.len();
        data.request_headers.fill(0);data.request_headers[..headers.len()].copy_from_slice(headers);data.request_head=headers.len();
        data.method[..method.len()].copy_from_slice(method);data.method_length=method.len();
        data.request_body[..body.len()].copy_from_slice(body);data.request_length=body.len();
        slot.id.store(id,Ordering::Relaxed);slot.cancelled.store(false,Ordering::Relaxed);
        slot.state.store(PENDING,Ordering::Release);return id;
    }
    0
}
// ------------------------=
// FUNC: poll
// DESC: Lends one immutable response chunk; the next poll acknowledges and releases the prior chunk before producer reuse.
// ------------------=
/// Engine owner only. Returned pointers remain valid until the next poll or
/// cancel(id); callers must finish reading before either call and never retain them.
pub unsafe fn poll(id:u64,out:&mut abi::Response)->u32 {
    let Some(slot)=SLOTS.iter().find(|slot|slot.id.load(Ordering::Relaxed)==id && id!=0) else{return 2;};
    poll_slot(slot,out)
}
// ------------------------=
// FUNC: poll_slot
// DESC: Applies the immutable-chunk loan and acknowledgement transitions to one owned slot.
// ------------------=
unsafe fn poll_slot(slot:&Slot,out:&mut abi::Response)->u32 {
    let state=slot.state.load(Ordering::Acquire);
    if slot.cancelled.load(Ordering::Acquire) {return 2;}
    match state {
        PENDING|ACTIVE|WRITING=>0,
        CHUNK_READING=>{slot.state.store(ACTIVE,Ordering::Release);0},
        CHUNK_READY=>{
            let data=&*slot.data.get();
            *out=abi::Response{status:data.status,headers:data.headers.as_ptr(),headers_length:data.head,
                body:data.body.as_ptr(),body_length:data.size};
            slot.state.store(CHUNK_READING,Ordering::Relaxed);3
        },
        READY=>4,
        _=>2,
    }
}

// ------------------------=
// FUNC: publish_progress
// DESC: Publishes one authenticated chunk only after the engine releases the previous loan, applying bounded backpressure.
// ------------------=
pub(crate) fn publish_progress(head:Option<&[u8]>,bytes:&[u8])->bool { unsafe {
    let Some((index,_))=CURRENT else {return false;};
    publish_slot(&SLOTS[index],head,bytes)
}}
// ------------------------=
// FUNC: publish_slot
// DESC: Writes only an acknowledged slot and publishes validated headers or payload with release ordering.
// ------------------=
unsafe fn publish_slot(slot:&Slot,head:Option<&[u8]>,bytes:&[u8])->bool {
    if slot.cancelled.load(Ordering::Acquire) {return false;}
    if slot.state.load(Ordering::Acquire)!=ACTIVE {return false;}
    let data=&mut *slot.data.get();
    data.head=0;data.status=0;
    if let Some(head)=head {
        let Ok(Some(parsed))=crate::http_transport::response::parse(head,false) else {return false;};
        let Ok(headers)=crate::http_transport::response::Headers::parse(head) else {return false;};
        data.status=parsed.status as u32;
        for (name,value) in headers.iter() {
            if name.eq_ignore_ascii_case("transfer-encoding") || name.eq_ignore_ascii_case("connection") {continue;}
            for part in [name.as_bytes(),b": ",value,b"\r\n"] {
                if part.len()>data.headers.len()-data.head {return false;}
                data.headers[data.head..data.head+part.len()].copy_from_slice(part);data.head+=part.len();
            }
        }
    }
    if bytes.len()>data.body.len() {return false;}
    data.body[..bytes.len()].copy_from_slice(bytes);data.size=bytes.len();
    slot.state.store(CHUNK_READY,Ordering::Release);true
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
    // FUNC: streaming_handoff_preserves_loans_and_framing
    // DESC: Verifies early headers, lossless chunks, backpressure, acknowledgements, cancellation, and terminal state.
    // ------------------=
    #[test]
    fn streaming_handoff_preserves_loans_and_framing() { unsafe {
        static SLOT:Slot=Slot::new();
        let slot=&SLOT;
        slot.state.store(ACTIVE,Ordering::Release);
        let mut out=abi::Response{status:0,headers:core::ptr::null(),headers_length:0,body:core::ptr::null(),body_length:0};
        let head=b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        assert!(publish_slot(slot,Some(head),&[]));
        assert!(!publish_slot(slot,None,b"overwritten"));
        assert_eq!(poll_slot(slot,&mut out),3);assert_eq!(out.status,200);
        assert_eq!(core::slice::from_raw_parts(out.headers,out.headers_length),b"Content-Type: image/png\r\n");
        assert!(!publish_slot(slot,None,b"still borrowed"));
        assert_eq!(poll_slot(slot,&mut out),0);
        let bytes=[0,128,255,9];
        assert!(publish_slot(slot,None,&bytes));
        assert_eq!(poll_slot(slot,&mut out),3);assert_eq!(out.status,0);
        assert_eq!(core::slice::from_raw_parts(out.body,out.body_length),bytes);
        assert_eq!(poll_slot(slot,&mut out),0);
        slot.state.store(READY,Ordering::Release);assert_eq!(poll_slot(slot,&mut out),4);
        slot.cancelled.store(true,Ordering::Release);assert_eq!(poll_slot(slot,&mut out),2);
        assert!(!publish_slot(slot,None,b"late"));
    }}
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
        else if matches!(slot.state.load(Ordering::Acquire),CHUNK_READY|CHUNK_READING) {return;}
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
                            // Servo owns redirect URLs and destination-specific cookies.
                            let data=&mut *slot.data.get();data.status=response.status as u32;
                            data.head=0;data.size=0;true
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
    if !transport_ready() {return;}
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
        match https::get_browser_streaming(owner,caps[0],caps[1],caps[2],caps[3],options.host,443,options.target,
            &data.request_headers[..data.request_head],core::str::from_utf8(&data.method[..data.method_length]).unwrap(),
            &data.request_body[..data.request_length],publish_progress) {
            Ok(ticket)=>{INFINITY_BROWSER_NETWORK_FAILURE.store(0,Ordering::Release);
                slot.state.store(ACTIVE,Ordering::Release);CURRENT=Some((index,ticket));},
            Err(https::Failure::Busy)=>{},
            Err(error)=>{INFINITY_BROWSER_NETWORK_FAILURE.store(error as u32+1,Ordering::Release);
                slot.state.store(FAILED,Ordering::Release);},
        }
        break;
    }
}
