//! Authenticated BSP-side launch and viewport synchronization for the native browser.
use super::*;
use infinity_browser_core::{layout::Layout,worker as abi};
use crate::runtime::{capability::CapabilityType,execution::SecurityIdentity};
struct Launch {owner:SecurityIdentity,url:[u8;2048],length:usize,stage:u8,size:(u32,u32)}
static mut LAUNCH:Option<Launch>=None;
static mut INPUT:infinity_browser_core::input_queue::Queue<64>=infinity_browser_core::input_queue::Queue::new();
static mut POINTER:infinity_browser_core::pointer::Pointer=infinity_browser_core::pointer::Pointer::new();
static mut DOWNLOAD:Option<crate::runtime::browser::Download>=None;
static mut CONSENT:Option<Launch>=None;

// ------------------------=
// FUNC: request_access
// DESC: Opens native browser consent without starting Servo or granting network authority.
// ------------------=
pub(super) fn request_access(console:&mut ConsoleRuntime,url:&[u8]) {
    if url.len()>2048 {return;}
    let mut consent=Launch{owner:SecurityIdentity(console.current_session.0),url:[0;2048],length:url.len(),stage:0,size:(0,0)};
    consent.url[..url.len()].copy_from_slice(url);
    unsafe {CONSENT=Some(consent);}
    if console.mode!=ConsoleMode::Desktop {console.enter_desktop();}
    console.store_active_app_window();console.desktop_app=DesktopAppKind::Browser;
    console.browser_window.visible=true;console.load_active_app_window();
    console.ai_chat_focus=0;console.shell_menu=0;
    crate::runtime::browser::permission_presentation(1);
}

// ------------------------=
// FUNC: approve_access
// DESC: Grants only the existing privileged, time-bounded network lease after a native user approval.
// ------------------=
fn approve_access(console:&mut ConsoleRuntime) {unsafe {
    let Some(consent)=(&*(&raw const CONSENT)).as_ref() else {return;};
    if consent.owner!=SecurityIdentity(console.current_session.0) {CONSENT=None;crate::runtime::browser::permission_presentation(0);return;}
    if !geturl::authorize_for(console,true,true) {crate::runtime::browser::permission_presentation(2);return;}
    let mut command=[0u8;2056];command[..8].copy_from_slice(b"browser ");
    command[8..8+consent.length].copy_from_slice(&consent.url[..consent.length]);let length=8+consent.length;
    CONSENT=None;crate::runtime::browser::permission_presentation(0);
    execute(console,&command[..length]);
}}

struct DownloadPolicy {owner:SecurityIdentity,capability:u64}
impl crate::storage::object::ObjectCapabilityPolicy for DownloadPolicy {
    // ------------------------=
    // FUNC: authorize
    // DESC: Revalidates only the ephemeral user-approved download creation authority at each object-service boundary.
    // ------------------=
    fn authorize(&self,operation:crate::storage::object::ObjectOperation,_:Option<crate::storage::object::ObjectRef>)->bool {
        use crate::storage::object::ObjectOperation as O;
        if !matches!(operation,O::Create|O::NamespaceAttach|O::RelationshipAttach) {return false;}
        let Some(now)=crate::runtime::node_client::clock() else {return false;};
        crate::runtime::with_runtime(|runtime|
            runtime.capabilities.validate(self.capability,self.owner,CapabilityType::ObjectWrite,0,1,0,now).is_ok()).unwrap_or(false)
    }
}

// ------------------------=
// FUNC: save_download
// DESC: Requires an active session and explicit native Save click before granting a one-operation object-write lease.
// ------------------=
fn save_download(console:&ConsoleRuntime) {unsafe {
    let Some(download)=(&*(&raw const DOWNLOAD)).as_ref() else {return;};
    let owner=SecurityIdentity(console.current_session.0);
    if !(&*(&raw const LAUNCH)).as_ref().is_some_and(|launch|launch.owner==owner && launch.stage==2) {return;}
    let now=crate::runtime::node_client::clock();
    let capability=crate::runtime::with_runtime(|runtime| {
        let active=(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|runtime.identity.session_nth(i))
            .any(|s|s.id==console.current_session&&s.user==console.current_user&&s.state==crate::runtime::identity::SessionState::Active);
        if !active {return None;}
        let now=now?;
        runtime.capabilities.grant(CapabilityType::ObjectWrite,0,1,0,owner,owner,Some(now.saturating_add(5)),0).ok()
    }).flatten();
    let Some(capability)=capability else {crate::runtime::browser::download_presentation(&download.name[..download.name_length],3);return;};
    let policy=DownloadPolicy{owner,capability};
    let prefix=b"/home/default/downloads/";
    let mut path=[0u8;95];path[..prefix.len()].copy_from_slice(prefix);
    path[prefix.len()..prefix.len()+download.name_length].copy_from_slice(&download.name[..download.name_length]);
    let saved=crate::storage::object_save_download(crate::storage::object::DownloadCreateRequest {
        name:&download.name[..download.name_length],content:&download.bytes[..download.length],
        path:&path[..prefix.len()+download.name_length],media_type:&download.media_type[..download.type_length],
        owner:crate::storage::object::ObjectId(console.current_user.0),
    },&policy).is_ok();
    crate::runtime::with_runtime(|runtime|{let _=runtime.capabilities.retire_leaf(capability,owner);});
    crate::runtime::browser::download_presentation(&download.name[..download.name_length],if saved {2}else{3});
    if saved {DOWNLOAD=None;}
}}

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
    CONSENT=None;crate::runtime::browser::permission_presentation(0);
    DOWNLOAD=None;crate::runtime::browser::download_presentation(&[],0);
    (&mut *(&raw mut INPUT)).clear();
    POINTER=infinity_browser_core::pointer::Pointer::new();
    if let Some(launch)=(&mut *(&raw mut LAUNCH)).as_mut() {launch.stage=3;}
}}

// ------------------------=
// FUNC: key
// DESC: Admits native text and editing keys as complete press/release pairs for focused web content.
// ------------------=
pub(super) fn key(console:&mut ConsoleRuntime,key:ConsoleKey) {
    if let ConsoleKey::Shortcut(value)=key {
        let mut command=abi::Command::empty();
        match value {
            b't'|b'T'=>command.kind=abi::TAB_CREATE,
            b'w'|b'W'=>{
                let view=crate::runtime::browser::presentation();
                if view.tab_count<=1 {console.close_desktop_app();return;}
                command.kind=abi::TAB_CLOSE;command.a=view.active_tab;
            },
            b'l'|b'L'=>{crate::runtime::browser::select_address();return;},
            b'a'|b'A' if crate::runtime::browser::presentation().address_focused=>{
                crate::runtime::browser::select_address();return;
            },
            _=>return,
        }
        if enqueue(console,command) {poll(console);}
        return;
    }
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
    let mut command=abi::Command::empty();command.kind=abi::NAVIGATE;
    let Ok(input)=core::str::from_utf8(&bytes[..length]) else{return;};
    let Ok((_,length))=infinity_browser_core::omnibox::resolve(input,
        "https://www.google.com/search?q=",&mut command.text) else{return;};
    command.length=length as u32;
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
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32).map(|layout|layout.with_tab_count(crate::runtime::browser::presentation().tab_count)) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    if crate::runtime::browser::presentation().permission!=0 {
        if layout.download_save.local(x,y).is_some() {approve_access(console);}
        else if layout.download_discard.local(x,y).is_some() {console.close_desktop_app();}
        else if layout.close.local(x,y).is_some() {console.close_desktop_app();}
        return true;
    }
    if crate::runtime::browser::presentation().download_state!=0 && layout.download_card.local(x,y).is_some() {
        if layout.download_save.local(x,y).is_some() {save_download(console);}
        if layout.download_discard.local(x,y).is_some() {unsafe {DOWNLOAD=None;}crate::runtime::browser::download_presentation(&[],0);}
        return true;
    }
    let mut command=abi::Command::empty();
    let view=crate::runtime::browser::presentation();
    for index in 0..view.tab_count {
        let Some((tab,close))=layout.tab(index,view.tab_count) else {continue;};
        if tab.local(x,y).is_some_and(|(x,y)|infinity_browser_core::tab_style::contains(tab.width,tab.height,x,y)) {
            if close.local(x,y).is_some() && view.tab_count==1 {console.close_desktop_app();return true;}
            command.kind=if close.local(x,y).is_some(){abi::TAB_CLOSE}else{abi::TAB_SELECT};
            command.a=view.tabs[index].id;
            if enqueue(console,command) {poll(console);}
            return true;
        }
    }
    let Some(control)=layout.hit(x,y) else{return false;};
    match control {
        Control::Address=>crate::runtime::browser::focus_address(true),
        Control::Go=>navigate_address(console),
        Control::Back=>{if view.history&1!=0 {command.kind=abi::BACK;}},
        Control::Forward=>{if view.history&2!=0 {command.kind=abi::FORWARD;}},
        Control::Reload=>command.kind=abi::RELOAD,
        Control::NewTab=>command.kind=abi::TAB_CREATE,
        Control::Minimize=>console.minimize_desktop_app(),
        Control::Maximize=>console.toggle_window_maximized(5),
        Control::Close=>console.close_desktop_app(),
        Control::Content=>{crate::runtime::browser::focus_address(false);return false;},
        Control::Menu=>{
            let url=&view.address[..view.address_length];
            request_access(console,if url.starts_with(b"https://") {url}else{b"https://example.com/"});
        },
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
    if crate::runtime::browser::presentation().permission!=0 {return false;}
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
    let view=crate::runtime::browser::presentation();
    let mut hover=(0,false);
    if !captured {for index in 0..view.tab_count {
        let Some((tab,close))=layout.tab(index,view.tab_count) else {continue;};
        if tab.local(x,y).is_some_and(|(x,y)|infinity_browser_core::tab_style::contains(tab.width,tab.height,x,y)) {
            hover=(view.tabs[index].id,close.local(x,y).is_some());break;
        }
    }}
    crate::runtime::browser::hover_tab(hover.0,hover.1);
    if !captured && layout.content.local(x,y).is_none() {return false;}
    if !captured && crate::runtime::browser::presentation().download_state!=0 && layout.download_card.local(x,y).is_some() {return false;}
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
        if launch.stage==2 && DOWNLOAD.is_none() && launch.owner==SecurityIdentity(console.current_session.0) {
            if let Some(value)=crate::runtime::browser::take_download(launch.owner) {
                crate::runtime::browser::download_presentation(&value.name[..value.name_length],1);DOWNLOAD=Some(value);
            }
        }
        if launch.owner!=SecurityIdentity(console.current_session.0) {
            DOWNLOAD=None;crate::runtime::browser::download_presentation(&[],0);launch.stage=3;
        }
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
