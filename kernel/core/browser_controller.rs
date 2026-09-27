//! Authenticated BSP-side launch and viewport synchronization for the native browser.
use super::*;
use infinity_browser_core::{layout::Layout,worker as abi};
use crate::runtime::{capability::CapabilityType,execution::SecurityIdentity};
struct Launch {owner:SecurityIdentity,url:[u8;2048],length:usize,stage:u8,size:(u32,u32)}
static mut LAUNCH:Option<Launch>=None;
static mut INPUT:infinity_browser_core::input_queue::Queue<64>=infinity_browser_core::input_queue::Queue::new();
static mut POINTER:infinity_browser_core::pointer::Pointer=infinity_browser_core::pointer::Pointer::new();

// ------------------------=
// FUNC: viewport
// DESC: Uses the same physical content bounds and scale as the browser painter.
// ------------------=
fn viewport(console:&ConsoleRuntime)->Option<(u32,u32)> {
    let state=console.browser_window_state();
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let layout=Layout::new(bounds.width,bounds.height,scale as u32)?;
    Some((layout.content.width,layout.content.height))
}

// ------------------------=
// FUNC: execute
// DESC: Opens a native browser only for an active session already holding the required network capabilities.
// ------------------=
pub(super) fn execute(console:&mut ConsoleRuntime,command:&[u8])->bool {
    if command==b"browser authorize" || command==b"browser authorize confirm=true" {
        geturl::authorize_for(console,command==b"browser authorize confirm=true",true);return true;
    }
    let Some(url)=command.strip_prefix(b"browser ") else {return false;};
    if url.len()>2048 || !(url.starts_with(b"https://") || url.starts_with(b"http://")) || core::str::from_utf8(url).is_err() {
        console.output.write_line(b"Usage: browser https://example.com/");return true;
    }
    let owner=SecurityIdentity(console.current_session.0);
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000_000;
    let caps=crate::runtime::with_runtime(|runtime| {
        if !(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|runtime.identity.session_nth(i))
            .any(|s|s.id==console.current_session && s.user==console.current_user
                && s.state==crate::runtime::identity::SessionState::Active) {return None;}
        let mut ids=[0;4];
        for (i,kind) in [CapabilityType::NetworkConnect,CapabilityType::NetworkSend,
            CapabilityType::NetworkReceive,CapabilityType::NetworkResolve].into_iter().enumerate() {
            ids[i]=(0..runtime.capabilities.count()).filter_map(|i|runtime.capabilities.nth(i))
                .find(|c|runtime.capabilities.validate(c.id,owner,kind,0,1,0,now).is_ok())?.id;
        }
        Some(ids)
    }).flatten();
    let Some(caps)=caps else {
        console.output.write_line(b"Browser requires network permission. See https authorize.");return true;
    };
    let Some(size)=viewport(console) else {console.output.write_line(b"Browser window is too small.");return true;};
    if size.0>2048 || size.1>2048 || !unsafe {crate::runtime::browser::start(owner,caps)} {
        console.output.write_line(b"Native browser worker is unavailable.");return true;
    }
    let mut launch=Launch{owner,url:[0;2048],length:url.len(),stage:0,size};
    launch.url[..url.len()].copy_from_slice(url);
    unsafe {
        if let Some(previous)=(&*(&raw const LAUNCH)).as_ref() {
            if previous.owner==owner && previous.stage!=0 {
                // OPEN was already accepted. Preserve its last sent viewport so
                // a new navigation cannot accidentally suppress a pending resize.
                launch.stage=1;launch.size=previous.size;
            }
        }
        if launch.stage==0 {(&mut *(&raw mut INPUT)).clear();POINTER=infinity_browser_core::pointer::Pointer::new();}
        LAUNCH=Some(launch);
    }
    if console.mode!=ConsoleMode::Desktop {console.enter_desktop();}
    console.store_active_app_window();console.desktop_app=DesktopAppKind::Browser;
    console.browser_window.visible=true;console.load_active_app_window();
    console.ai_chat_focus=0;console.shell_menu=0;
    console.app_window_dragging=false;console.app_window_resizing=None;
    poll(console);true
}

// ------------------------=
// FUNC: close
// DESC: Retains a close request until the worker mailbox accepts it, superseding pending navigation.
// ------------------=
pub(super) fn close() {unsafe {
    (&mut *(&raw mut INPUT)).clear();
    POINTER=infinity_browser_core::pointer::Pointer::new();
    if let Some(launch)=(&mut *(&raw mut LAUNCH)).as_mut() {launch.stage=3;}
}}

// ------------------------=
// FUNC: key
// DESC: Admits native text and editing keys as complete press/release pairs for focused web content.
// ------------------=
pub(super) fn key(console:&ConsoleRuntime,key:ConsoleKey) {
    if crate::runtime::browser::presentation().address_focused {
        if matches!(key,ConsoleKey::Enter) {navigate_address(console);}
        else if matches!(key,ConsoleKey::Escape|ConsoleKey::Tab(_)) {crate::runtime::browser::focus_address(false);}
        else if let Some(key)=text_edit_key(key) {crate::runtime::browser::edit_address(key);}
        return;
    }
    let mut down=abi::Command::empty();down.kind=abi::KEY;down.flags=abi::KEY_DOWN|abi::KEY_NAMED;
    down.a=match key {
        ConsoleKey::Character(c)=>{down.flags=abi::KEY_DOWN;u32::from(c)},
        ConsoleKey::Enter=>1,ConsoleKey::Tab(shift)=>{if shift {down.b=abi::MOD_SHIFT;}2},
        ConsoleKey::Backspace=>3,ConsoleKey::Delete=>4,ConsoleKey::Left=>5,ConsoleKey::Right=>6,
        ConsoleKey::Up=>7,ConsoleKey::Down=>8,ConsoleKey::Home=>9,ConsoleKey::End=>10,ConsoleKey::Escape=>11,
        _=>return,
    };
    let mut up=down;up.flags&=!abi::KEY_DOWN;
    unsafe {
        let Some(launch)=(&*(&raw const LAUNCH)).as_ref() else{return;};
        if launch.stage!=2 || launch.owner!=SecurityIdentity(console.current_session.0) {return;}
        let accepted=(&mut *(&raw mut INPUT)).push_reserved(&[down,up],3);
        crate::runtime::browser::input_pressure(!accepted);
    }
    poll(console);
}

// ------------------------=
// FUNC: enqueue
// DESC: Admits a native chrome operation only into its authenticated active engine lifetime.
// ------------------=
fn enqueue(console:&ConsoleRuntime,command:abi::Command)->bool {unsafe {
    let Some(launch)=(&*(&raw const LAUNCH)).as_ref() else{return false;};
    if launch.stage!=2 || launch.owner!=SecurityIdentity(console.current_session.0) {return false;}
    let accepted=(&mut *(&raw mut INPUT)).push_reserved(&[command],3);
    crate::runtime::browser::input_pressure(!accepted);
    accepted
}}

// ------------------------=
// FUNC: navigate_address
// DESC: Routes the current native address field through the same governed engine navigation queue.
// ------------------=
fn navigate_address(console:&ConsoleRuntime) {
    let view=crate::runtime::browser::presentation();
    let (bytes,length)=if view.address_focused {(&view.edit,view.edit_length)}else{(&view.address,view.address_length)};
    let mut command=abi::Command::empty();command.kind=abi::NAVIGATE;command.length=length as u32;
    command.text[..length].copy_from_slice(&bytes[..length]);
    if enqueue(console,command) {crate::runtime::browser::focus_address(false);poll(console);}
}

// ------------------------=
// FUNC: chrome_pointer
// DESC: Hit-tests native chrome with the exact painter layout and keeps those clicks out of web content.
// ------------------=
pub(super) fn chrome_pointer(console:&mut ConsoleRuntime)->bool {
    use infinity_browser_core::layout::Control;
    let state=console.browser_window_state();
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    let Some(control)=layout.hit(x,y) else{return false;};
    let mut command=abi::Command::empty();
    let view=crate::runtime::browser::presentation();
    match control {
        Control::Address=>crate::runtime::browser::focus_address(true),
        Control::Go=>navigate_address(console),
        Control::Back=>{if view.history&1!=0 {command.kind=abi::BACK;}},
        Control::Forward=>{if view.history&2!=0 {command.kind=abi::FORWARD;}},
        Control::Reload=>command.kind=abi::RELOAD,
        Control::Minimize=>console.minimize_desktop_app(),
        Control::Maximize=>console.toggle_window_maximized(5),
        Control::Close=>console.close_desktop_app(),
        Control::Content=>{crate::runtime::browser::focus_address(false);return false;},
        _=>return false,
    }
    if command.kind!=0 && enqueue(console,command) {
        crate::runtime::browser::focus_address(false);poll(console);
    }
    true
}

// ------------------------=
// FUNC: scroll
// DESC: Maps desktop wheel input to viewport-local Servo coordinates without entering the engine on the UI CPU.
// ------------------=
pub(super) fn scroll(console:&ConsoleRuntime,vertical:i8)->bool {
    if vertical==0 {return false;}
    let state=console.browser_window_state();
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    let Some((x,y))=layout.content.local(x,y) else{return false;};
    let mut command=abi::Command::empty();command.kind=abi::SCROLL;
    command.x=x as i32;command.y=y as i32;command.b=(i32::from(vertical)*48) as u32;
    unsafe {
        let Some(launch)=(&*(&raw const LAUNCH)).as_ref() else{return false;};
        if launch.stage!=2 || launch.owner!=SecurityIdentity(console.current_session.0) {return false;}
        crate::runtime::browser::input_pressure(!(&mut *(&raw mut INPUT)).push_reserved(&[command],3));
    }
    poll(console);true
}

// ------------------------=
// FUNC: pointer
// DESC: Translates desktop pointer packets to web coordinates and preserves captured releases outside the page.
// ------------------=
pub(super) fn pointer(console:&ConsoleRuntime,buttons:u8,capture_only:bool)->bool {
    let captured=unsafe {(&*(&raw const POINTER)).captured()};
    if capture_only && !captured {return false;}
    let state=console.browser_window_state();
    if !state.visible {return false;}
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    if !captured && layout.content.local(x,y).is_none() {return false;}
    unsafe {
        let Some(launch)=(&*(&raw const LAUNCH)).as_ref() else{return false;};
        if launch.stage!=2 || launch.owner!=SecurityIdentity(console.current_session.0) {return false;}
        let accepted=(&mut *(&raw mut POINTER)).update(&mut *(&raw mut INPUT),
            x-layout.content.x,y-layout.content.y,buttons);
        crate::runtime::browser::input_pressure(!accepted);
    }
    if buttons&7!=0 {crate::runtime::browser::focus_address(false);}
    poll(console);true
}

// ------------------------=
// FUNC: poll
// DESC: Sends bounded ordered launch/resize/close commands, retaining them unchanged under backpressure.
// ------------------=
pub(super) fn poll(console:&ConsoleRuntime) {
    unsafe {
        let slot=&mut *(&raw mut LAUNCH);
        let Some(launch)=slot.as_mut() else {return;};
        if launch.owner!=SecurityIdentity(console.current_session.0) {launch.stage=3;}
        if matches!(launch.stage,1|2) {
            (&mut *(&raw mut INPUT)).drain(16,|command|crate::runtime::browser::submit(launch.owner,command).is_ok());
            if launch.stage==1 && !(&*(&raw const INPUT)).is_empty() {return;}
        }
        for _ in 0..2 {
            let mut command=abi::Command::empty();
            match launch.stage {
                0=>{command.kind=abi::OPEN;command.a=launch.size.0;command.b=launch.size.1;},
                1=>{command.kind=abi::NAVIGATE;command.length=launch.length as u32;
                    command.text[..launch.length].copy_from_slice(&launch.url[..launch.length]);},
                2=>{
                    let Some(size)=viewport(console) else {return;};
                    if size==launch.size || size.0>2048 || size.1>2048 {return;}
                    command.kind=abi::RESIZE;command.a=size.0;command.b=size.1;
                },
                _=>command.kind=abi::CLOSE,
            }
            if crate::runtime::browser::submit(launch.owner,command).is_err() {return;}
            match launch.stage {
                0|1=>launch.stage+=1,
                2=>{launch.size=(command.a,command.b);return;},
                _=>{*slot=None;return;},
            }
        }
    }
}
