//! Native engine component. No boot addresses or device drivers belong here.
#![no_std]
extern crate std;
mod resources;
mod session;
#[path = "../infinity-browser-core/worker.rs"]
mod abi;
use core::{mem::MaybeUninit, sync::atomic::{AtomicPtr, Ordering}};
use std::{string::ToString, time::Duration};
use infinity_servo_runtime_primitives::native::{Hooks, Runtime};
static HOST: AtomicPtr<abi::Host> = AtomicPtr::new(core::ptr::null_mut());
static mut RUNTIME: MaybeUninit<Runtime> = MaybeUninit::uninit();
#[no_mangle]
static __dso_handle:usize=0;
// ------------------------=
// FUNC: infinity_browser_tls_trace
// DESC: Records test-only TLS destructor boundaries to identify a blocked native teardown.
// ------------------=
#[cfg(infinity_component_trace)]
#[no_mangle]
pub extern "C" fn infinity_browser_tls_trace(thread:u64,callback:usize,done:u32) {
    if trace::ACTIVE.load(Ordering::Relaxed) {
        static EVENTS:core::sync::atomic::AtomicUsize=core::sync::atomic::AtomicUsize::new(0);
        if done>=5 && EVENTS.fetch_add(1,Ordering::Relaxed)>=512 {return;}
        let name=unsafe{Runtime::diagnostic_thread_name(thread)};
        let end=name.iter().position(|byte|*byte==0).unwrap_or(name.len());
        event(abi::EVENT_DIAGNOSTIC,503,&std::format!("TLS {thread} {callback:#x} {done} {}\n",
            core::str::from_utf8(&name[..end]).unwrap_or("?")));
    }
}
#[cfg(infinity_component_trace)]
mod trace {
    use core::sync::atomic::{AtomicBool,Ordering};
    pub static ACTIVE:AtomicBool=AtomicBool::new(false);
    struct Logger;
    static LOGGER:Logger=Logger;
    impl log::Log for Logger {
        // ------------------------=
        // FUNC: enabled
        // DESC: Restricts test diagnostics to shutdown, never normal page rendering.
        // ------------------=
        fn enabled(&self,metadata:&log::Metadata)->bool {
            ACTIVE.load(Ordering::Relaxed) && (metadata.target().contains("constellation")
                || metadata.target().contains("script_thread") || metadata.target().contains("background_hang"))
        }
        // ------------------------=
        // FUNC: log
        // DESC: Sends test-only shutdown progress to the supervising guest fixture.
        // ------------------=
        fn log(&self,record:&log::Record) {
            if self.enabled(record.metadata()) {
                super::event(super::abi::EVENT_DIAGNOSTIC,502,&std::format!("{}\n",record.args()));
            }
        }
        // ------------------------=
        // FUNC: flush
        // DESC: Has no buffering or filesystem output to flush.
        // ------------------=
        fn flush(&self) {}
    }
    // ------------------------=
    // FUNC: install
    // DESC: Installs tracing only in explicitly instrumented component fixtures.
    // ------------------=
    pub fn install() {if log::set_logger(&LOGGER).is_ok(){log::set_max_level(log::LevelFilter::Debug);}}
}

// ------------------------=
// FUNC: host
// DESC: Accesses the permanently granted callback table after entry validation.
// ------------------=
fn host() -> &'static abi::Host { unsafe { &*HOST.load(Ordering::Acquire) } }
// ------------------------=
// FUNC: cpu
// DESC: Obtains owner identity without architecture-specific application logic.
// ------------------=
fn cpu() -> u64 { let h=host(); unsafe { (h.cpu)(h.context) } }
// ------------------------=
// FUNC: clock
// DESC: Uses the native monotonic service for executor and resource deadlines.
// ------------------=
fn clock() -> u64 { let h=host(); unsafe { (h.monotonic)(h.context) } }
// ------------------------=
// FUNC: utc
// DESC: Refuses fabricated wall time when the native clock is unavailable.
// ------------------=
fn utc() -> Option<(u64,u32)> {
    let h=host(); let mut seconds=0; let mut nanos=0;
    if unsafe { (h.utc)(h.context,&mut seconds,&mut nanos) } == 1 && nanos<1_000_000_000 {
        Some((seconds,nanos))
    } else { None }
}
// ------------------------=
// FUNC: entropy
// DESC: Obtains explicit native entropy without a deterministic fallback.
// ------------------=
fn entropy(bytes: &mut [u8]) -> bool { let h=host(); unsafe { (h.entropy)(h.context,bytes.as_mut_ptr(),bytes.len()) == 1 } }
// ------------------------=
// FUNC: idle
// DESC: Gives the host a nonblocking worker service opportunity.
// ------------------=
fn idle() { let h=host(); unsafe { (h.idle)(h.context) } }
// ------------------------=
// FUNC: event
// DESC: Transfers bounded metadata to the native shell without lending ownership.
// ------------------=
fn event(kind:u32,value:u32,text:&str) {
    let h=host(); let mut length=text.len().min(2048);
    while !text.is_char_boundary(length) { length-=1; }
    unsafe { (h.event)(h.context,kind,value,text.as_ptr(),length) }
}

struct Network;
impl resources::Provider for Network {
    // ------------------------=
    // FUNC: download
    // DESC: Copies an attachment into the native consent mailbox without granting file or namespace access to Servo.
    // ------------------=
    fn download(&mut self, metadata:&resources::download::Metadata, body:&[u8])->Result<(),()> {
        let h=host();
        if unsafe {(h.download)(h.context,metadata.name.as_ptr(),metadata.name_length,
            metadata.media_type.as_ptr(),metadata.type_length,body.as_ptr(),body.len())}==1 {Ok(())} else {Err(())}
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Requests authority through the host mailbox instead of opening sockets.
    // ------------------=
    fn begin(&mut self,url:&str)->Result<u64,()> {
        self.begin_with_headers(url, &[])
    }
    // ------------------------=
    // FUNC: begin_with_headers
    // DESC: Copies bounded engine-selected fields into the versioned native network mailbox.
    // ------------------=
    fn begin_with_headers(&mut self,url:&str,headers:&[u8])->Result<u64,()> {
        let h=host(); let id=unsafe { (h.begin)(h.context,url.as_ptr(),url.len(),headers.as_ptr(),headers.len()) };
        if id==0 { Err(()) } else { Ok(id) }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Copies a bounded native response while its mailbox handle remains owned.
    // ------------------=
    fn poll(&mut self,id:u64)->Result<Option<resources::Response>,()> {
        let h=host(); let mut response=abi::Response {status:0,headers:core::ptr::null(),headers_length:0,
            body:core::ptr::null(),body_length:0};
        match unsafe { (h.poll)(h.context,id,&mut response) } { 0=>return Ok(None),1=>{},_=>return Err(()) }
        if !(100..=599).contains(&response.status) || response.headers_length>16*1024
            || response.body_length>resources::MAX_BODY
            || (response.headers_length>0 && response.headers.is_null())
            || (response.body_length>0 && response.body.is_null()) { return Err(()); }
        let raw=if response.headers_length==0 { &[][..] } else { unsafe {core::slice::from_raw_parts(response.headers,response.headers_length)} };
        let mut headers=std::vec::Vec::new();
        for line in core::str::from_utf8(raw).map_err(|_|())?.split("\r\n").filter(|line|!line.is_empty()) {
            let (name,value)=line.split_once(':').ok_or(())?;
            if headers.len()==32 { return Err(()); }
            headers.push((name.to_string(),value.trim().to_string()));
        }
        let body=if response.body_length==0 { std::vec::Vec::new() } else {
            unsafe { core::slice::from_raw_parts(response.body,response.body_length) }.to_vec()
        };
        Ok(Some(resources::Response {status:response.status as u16,headers,body}))
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Releases the host operation on all success, failure and close paths.
    // ------------------=
    fn cancel(&mut self,id:u64) { let h=host(); unsafe { (h.cancel)(h.context,id) } }
}

// ------------------------=
// FUNC: run
// DESC: Owns the engine and one reopenable window; processes bounded batches off the UI CPU.
// ------------------=
fn run() {
    extern "C" { static browser_init_start:usize; static browser_init_end:usize; fn infinity_sqlite_initialize()->i32; }
    unsafe {
        let mut entry=core::ptr::addr_of!(browser_init_start);
        while entry<core::ptr::addr_of!(browser_init_end) {
            let initialize:extern "C" fn()=core::mem::transmute(entry.read()); initialize();entry=entry.add(1);
        }
        if infinity_sqlite_initialize()!=0 { fatal(2); }
    }
    let engine=servo::ServoBuilder::default().build();
    #[cfg(infinity_component_trace)]
    verify_closed_channel_retirement();
    let mut session:Option<session::TabSessions<Network>>=None;
    let mut last_address=None;
    let mut last_title=None;
    let mut last_complete=false;
    let mut last_history=None;
    let mut last_failed=false;
    loop {
        for _ in 0..16 {
            let h=host(); let mut command=abi::Command::empty();
            if unsafe { (h.command)(h.context,&mut command) } != 1 { break; }
            if command.kind==abi::SHUTDOWN {
                #[cfg(infinity_component_trace)]
                trace::ACTIVE.store(true,Ordering::Relaxed);
                event(abi::EVENT_MEMORY,unsafe{Runtime::peak_allocated()} as u32,"");
                drop(session);
                let deadline=clock().saturating_add(100_000_000);
                while clock()<deadline {engine.spin_event_loop();std::thread::sleep(Duration::from_millis(1));}
                event(abi::EVENT_DIAGNOSTIC,500,"engine shutdown");
                #[cfg(infinity_component_trace)]
                trace::ACTIVE.store(true,Ordering::Relaxed);
                drop(engine);
                event(abi::EVENT_DIAGNOSTIC,501,"engine stopped");
                return;
            }
            if command.kind==abi::CLOSE {
                session=None; last_address=None; last_title=None; last_complete=false; last_history=None;last_failed=false;
                event(abi::EVENT_MEMORY,unsafe{Runtime::peak_allocated()} as u32,"");
                event(abi::EVENT_CLOSED,0,""); continue;
            }
            if command.kind==abi::OPEN {
                if session.is_none() {
                    session=session::TabSessions::new(&engine,Network,clock,command.a,command.b).ok();
                    if let Some(group)=session.as_mut() {
                        if let Ok(id)=group.create(&engine,Network,true) {
                            event(abi::EVENT_TAB_CREATED,id,"");event(abi::EVENT_TAB_SELECTED,id,"");
                        } else {session=None;}
                    }
                    event(if session.is_some(){abi::EVENT_OPEN}else{abi::EVENT_ERROR},0,"");
                }
                continue;
            }
            let Some(group)=session.as_mut() else { continue; };
            if matches!(command.kind,abi::TAB_CREATE|abi::TAB_SELECT|abi::TAB_CLOSE) {
                let previous=group.active();
                let result=match command.kind {
                    abi::TAB_CREATE=>group.create(&engine,Network,true).map(|id|event(abi::EVENT_TAB_CREATED,id,"")),
                    abi::TAB_SELECT=>group.select(command.a),
                    _=>group.close(command.a).map(|()|event(abi::EVENT_TAB_CLOSED,command.a,"")),
                };
                if result.is_ok() && group.active()!=previous {
                    event(abi::EVENT_TAB_SELECTED,group.active(),"");
                    last_address=None;last_title=None;last_complete=false;last_history=None;last_failed=false;
                }
                if result.is_err() {event(abi::EVENT_ERROR,1,"");}
                continue;
            }
            if command.kind==abi::RESIZE {
                if group.resize(command.a,command.b).is_err() {event(abi::EVENT_ERROR,1,"");}
                continue;
            }
            let Some(view)=group.current() else {continue;};
            if dispatch(view,&command).is_err() { event(abi::EVENT_ERROR,1,""); }
            else if matches!(command.kind,abi::NAVIGATE|abi::RELOAD|abi::BACK|abi::FORWARD) && !view.complete() {
                last_complete=false;event(abi::EVENT_LOAD,0,"");
            }
        }
        if let Some(group)=session.as_mut() {
            if group.pump(&engine,|w,h,bytes| {
                let host=host(); unsafe { (host.frame)(host.context,w,h,bytes.as_ptr(),bytes.len()) }
            }).is_err() { event(abi::EVENT_ERROR,2,""); session=None; continue; }
            let Some(view)=group.current() else {std::thread::sleep(Duration::from_millis(1));continue;};
            if view.failed() && !last_failed {event(abi::EVENT_ERROR,3,"");}
            last_failed=view.failed();
            let address=view.address(); let title=view.title(); let complete=view.complete();
            if address!=last_address { event(abi::EVENT_ADDRESS,0,address.as_deref().unwrap_or("")); last_address=address; }
            if title!=last_title { event(abi::EVENT_TITLE,0,title.as_deref().unwrap_or("")); last_title=title; }
            if complete!=last_complete { event(abi::EVENT_LOAD,complete as u32,"");last_complete=complete; }
            let history=view.history_available();
            if last_history!=Some(history) {event(abi::EVENT_HISTORY,history,"");last_history=Some(history);}
        } else { engine.spin_event_loop(); }
        std::thread::sleep(Duration::from_millis(1));
    }
}

// ------------------------=
// FUNC: verify_closed_channel_retirement
// DESC: Requires closed channels to be reported once, stable live IDs and native blocking after selector reconstruction.
// ------------------=
#[cfg(infinity_component_trace)]
fn verify_closed_channel_retirement() {
    use servo_base::generic_channel::{self,GenericReceiverSet,GenericSelectionResult};
    let (closed_tx,closed_rx)=generic_channel::channel::<u32>().unwrap();
    let (live_tx,live_rx)=generic_channel::channel::<u32>().unwrap();
    let mut set=GenericReceiverSet::new();
    let closed_id=set.add(closed_rx);
    let live_id=set.add(live_rx);
    drop(closed_tx);
    let mut selector=set.selector();
    assert_eq!(selector.select().as_slice(),&[GenericSelectionResult::ChannelClosed(closed_id)]);
    let sender=live_tx.clone();
    let worker=std::thread::spawn(move||{sender.send(17).unwrap();});
    assert_eq!(selector.select().as_slice(),&[GenericSelectionResult::MessageReceived(live_id,17)]);
    worker.join().unwrap();
    drop(selector);
    let worker=std::thread::spawn(move||{live_tx.send(29).unwrap();});
    let mut selector=set.selector();
    assert_eq!(selector.select().as_slice(),&[GenericSelectionResult::MessageReceived(live_id,29)]);
    worker.join().unwrap();
    assert_eq!(selector.select().as_slice(),&[GenericSelectionResult::ChannelClosed(live_id)]);
}

// ------------------------=
// FUNC: dispatch
// DESC: Converts integer-only native commands to Servo input, refusing malformed text.
// ------------------=
fn dispatch(view:&mut session::Session<Network>,command:&abi::Command)->Result<(),()> {
    let point=servo::WebViewPoint::Device((command.x as f32,command.y as f32).into());
    match command.kind {
        abi::FIND=>{
            let text=core::str::from_utf8(command.text.get(..command.length as usize).ok_or(())?).map_err(|_|())?;
            let token=command.b;
            view.find(text,command.a,move |value|event(abi::EVENT_FIND,value,&token.to_string()));
        },
        abi::NAVIGATE=>view.navigate(core::str::from_utf8(command.text.get(..command.length as usize).ok_or(())?).map_err(|_|())?)?,
        abi::RESIZE=>view.resize(command.a,command.b)?,
        abi::BACK=>view.history(false), abi::FORWARD=>view.history(true), abi::RELOAD=>view.reload(),
        abi::POINTER=>view.input(servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(point))),
        abi::BUTTON=>view.input(servo::InputEvent::MouseButton(servo::MouseButtonEvent::new(
            if command.flags&1!=0 {servo::MouseButtonAction::Down}else{servo::MouseButtonAction::Up},
            match command.a {0=>servo::MouseButton::Primary,1=>servo::MouseButton::Secondary,2=>servo::MouseButton::Auxiliary,_=>return Err(())},point))),
        abi::KEY=>{
            if command.flags & !(abi::KEY_DOWN|abi::KEY_NAMED|abi::KEY_REPEAT)!=0
                || command.b & !(abi::MOD_SHIFT|abi::MOD_CONTROL|abi::MOD_ALT|abi::MOD_META)!=0 {return Err(());}
            let key=if command.flags&abi::KEY_NAMED!=0 { servo::Key::Named(match command.a {
                1=>servo::NamedKey::Enter,2=>servo::NamedKey::Tab,3=>servo::NamedKey::Backspace,4=>servo::NamedKey::Delete,
                5=>servo::NamedKey::ArrowLeft,6=>servo::NamedKey::ArrowRight,7=>servo::NamedKey::ArrowUp,8=>servo::NamedKey::ArrowDown,
                9=>servo::NamedKey::Home,10=>servo::NamedKey::End,11=>servo::NamedKey::Escape,_=>return Err(())
            })} else { servo::Key::Character(char::from_u32(command.a).ok_or(())?.to_string()) };
            let mut event=servo::KeyboardEvent::from_state_and_key(
                if command.flags&abi::KEY_DOWN!=0 {servo::KeyState::Down}else{servo::KeyState::Up},key);
            for (mask,modifier) in [(abi::MOD_SHIFT,servo::Modifiers::SHIFT),(abi::MOD_CONTROL,servo::Modifiers::CONTROL),
                (abi::MOD_ALT,servo::Modifiers::ALT),(abi::MOD_META,servo::Modifiers::META)] {
                if command.b&mask!=0 {event.event.modifiers|=modifier;}
            }
            event.event.repeat=command.flags&abi::KEY_REPEAT!=0;
            view.input(servo::InputEvent::Keyboard(event));
        },
        abi::SCROLL=>view.scroll(servo::Scroll::Delta(servo::WebViewVector::Device((command.a as i32 as f32,command.b as i32 as f32).into())),point),
        _=>return Err(()),
    }
    Ok(())
}

// ------------------------=
// FUNC: fatal
// DESC: Transfers unrecoverable engine failure to the native worker supervisor.
// ------------------=
fn fatal(code:u32)->! { let h=host();unsafe { (h.fatal)(h.context,code) } }
// ------------------------=
// FUNC: infinity_browser_run
// DESC: Installs one explicitly granted runtime on its dedicated owner CPU.
// ------------------=
#[no_mangle]
pub unsafe extern "C" fn infinity_browser_run(host:*mut abi::Host)->u32 {
    if host.is_null() || (*host).version!=abi::VERSION || (*host).size as usize!=core::mem::size_of::<abi::Host>()
        || (*host).heap.is_null() || (*host).heap_length<128*1024*1024
        || (*host).heap_length>512*1024*1024
        || ((*host).heap as usize).checked_add((*host).heap_length).is_none() { return 1; }
    if HOST.compare_exchange(core::ptr::null_mut(),host,Ordering::AcqRel,Ordering::Acquire).is_err() { return 2; }
    let runtime=core::ptr::addr_of_mut!(RUNTIME).cast::<Runtime>();
    let Some(value)=Runtime::new(core::slice::from_raw_parts_mut((*host).heap,(*host).heap_length),
        Hooks {cpu,monotonic:clock,utc,entropy,pump:idle}) else {
            HOST.store(core::ptr::null_mut(),Ordering::Release);return 3;
        };
    runtime.write(value);
    if !Runtime::install(runtime) { return 4; }
    #[cfg(infinity_component_trace)]
    trace::install();
    std::panic::set_hook(std::boxed::Box::new(|info| {
        let line=info.location().map_or(0,|location|location.line());
        if let Some(location)=info.location() {event(abi::EVENT_DIAGNOSTIC,line,location.file());}
        if let Some(message)=info.payload().downcast_ref::<&str>() {event(abi::EVENT_DIAGNOSTIC,0,message);}
        if let Some(message)=info.payload().downcast_ref::<std::string::String>() {event(abi::EVENT_DIAGNOSTIC,0,message);}
        fatal(3)
    }));
    match std::thread::Builder::new().stack_size(4*1024*1024).spawn(run) {
        Ok(thread)=>{if thread.join().is_ok(){0}else{5}}, Err(_)=>6,
    }
}
// ------------------------=
// FUNC: infinity_browser_abort
// DESC: Forwards C abort without bringing Unix process termination into the kernel.
// ------------------=
#[no_mangle]
pub extern "C" fn infinity_browser_abort()->! {
    event(abi::EVENT_ALLOCATION_FAILURE,unsafe{Runtime::failed_request().min(u32::MAX as usize)} as u32,"");
    event(abi::EVENT_MEMORY,unsafe{Runtime::peak_allocated()} as u32,"");fatal(4)
}
