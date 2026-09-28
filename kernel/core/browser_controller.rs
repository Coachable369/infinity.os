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
static mut NEW_TABS:u8=0;
static mut FAVORITES:infinity_browser_core::favorites::Favorites=infinity_browser_core::favorites::Favorites::new();
static mut FAVORITES_OWNER:Option<[u8;16]>=None;
static mut FAVORITES_OFFSET:usize=0;
static mut FAVORITES_ERROR:u8=0;
static mut SETTINGS_OWNER:Option<[u8;16]>=None;

// ------------------------=
// FUNC: settings_sync
// DESC: Loads preferences on authenticated profile transitions without sharing state across users.
// ------------------=
fn settings_sync(console:&ConsoleRuntime,active:bool) {unsafe {
    let owner=if active {Some(console.current_user.0)}else{None};
    if SETTINGS_OWNER==owner {return;}SETTINGS_OWNER=owner;
    let mut settings=infinity_browser_core::settings::Settings::new();let mut notice=0;
    if active {let mut bytes=[0u8;6];
        match crate::storage::browser_settings(console.current_user.0,console.current_session.0,None,&mut bytes) {
            Ok(n)=>match infinity_browser_core::settings::Settings::decode(&bytes[..n]) {Some(s)=>settings=s,None=>notice=2},
            Err(crate::storage::object::ObjectError::NamespaceNotFound)=>{},Err(_)=>notice=2,
        }
    }
    crate::runtime::browser::settings_presentation(settings,false,notice,false);
}}
// ------------------------=
// FUNC: settings_action
// DESC: Applies native settings only after durable commit and requires confirmation before clearing favorites.
// ------------------=
fn settings_action(console:&ConsoleRuntime,index:usize) {
    if !favorites_sync(console) {return;}
    let v=crate::runtime::browser::presentation();let mut next=v.settings;
    if index==6 {crate::runtime::browser::settings_presentation(next,false,0,false);return;}
    if index==4 {
        if !v.settings_confirm {crate::runtime::browser::settings_presentation(next,true,0,true);return;}
        let mut empty=infinity_browser_core::favorites::Favorites::new();
        let _=empty.set_preferences(v.settings.search,v.settings.favorites);
        let ok=crate::storage::browser_favorites_save(console.current_user.0,console.current_session.0,empty.bytes()).is_ok();
        if ok {unsafe {FAVORITES=empty;FAVORITES_OFFSET=0;FAVORITES_ERROR=0;}favorites_sync(console);}
        crate::runtime::browser::settings_presentation(next,true,if ok {1}else{2},false);return;
    }
    match index {0..=2=>next.search=index as u8,3=>next.favorites=!next.favorites,5=>next=infinity_browser_core::settings::Settings::new(),_=>return}
    let mut previous=[0u8;6];
    let ok=crate::storage::browser_settings(console.current_user.0,console.current_session.0,Some(&next.bytes()),&mut previous).is_ok();
    if ok {unsafe {FAVORITES_OWNER=None;}favorites_sync(console);}
    crate::runtime::browser::settings_presentation(if ok {next}else{v.settings},true,if ok {1}else{2},false);
}
// ------------------------=
// FUNC: settings_dimensions
// DESC: Computes settings scale and visible height from the same window geometry as painting.
// ------------------=
fn settings_dimensions(console:&ConsoleRuntime)->(u32,u32) {
    let s=console.browser_window_state();let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(s.x,s.y,s.width,s.height,s.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1)) as u32;
    (viewport(console).map(|(_,h)|h).unwrap_or(320*scale),scale)
}

// ------------------------=
// FUNC: favorites_sync
// DESC: Loads only the active profile's durable favorites and preserves corrupt or unavailable objects untouched.
// ------------------=
fn favorites_sync(console:&ConsoleRuntime)->bool {unsafe {
    let active=crate::runtime::with_runtime(|runtime|(0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|i|runtime.identity.session_nth(i)).any(|s|s.id==console.current_session&&s.user==console.current_user
        &&s.state==crate::runtime::identity::SessionState::Active)).unwrap_or(false);
    if !active {FAVORITES_OWNER=None;FAVORITES=infinity_browser_core::favorites::Favorites::new();FAVORITES_OFFSET=0;FAVORITES_ERROR=0;}
    else if FAVORITES_OWNER!=Some(console.current_user.0) {
        FAVORITES=infinity_browser_core::favorites::Favorites::new();FAVORITES_OFFSET=0;FAVORITES_ERROR=0;
        let mut bytes=[0u8;infinity_browser_core::favorites::BYTES];
        match crate::storage::browser_favorites_load(console.current_user.0,console.current_session.0,&mut bytes) {
            Ok(length)=>match infinity_browser_core::favorites::Favorites::decode(&bytes[..length]) {
                Ok(value)=>FAVORITES=value,Err(_)=>FAVORITES_ERROR=2,
            },
            Err(crate::storage::object::ObjectError::NamespaceNotFound)=>{},
            Err(_)=>FAVORITES_ERROR=3,
        }
        FAVORITES_OWNER=Some(console.current_user.0);
    }
    settings_sync(console,active);
    crate::runtime::browser::favorites_presentation(&*(&raw const FAVORITES),FAVORITES_OFFSET,FAVORITES_ERROR);active
}}
// ------------------------=
// FUNC: toggle_favorite
// DESC: Saves or removes the current destination only after an authenticated durable object commit succeeds.
// ------------------=
fn toggle_favorite(console:&ConsoleRuntime) {unsafe {
    if FAVORITES_ERROR==3 {FAVORITES_OWNER=None;}
    if !favorites_sync(console) || matches!(FAVORITES_ERROR,2|3) {return;}
    let view=crate::runtime::browser::presentation();let url=&view.address[..view.address_length];
    let mut next=FAVORITES;
    let result=if let Some(index)=next.find(url) {next.remove(index)} else {
        let title=if view.title_length>0 && !view.loading && view.error==0 {&view.title[..view.title_length]}else{url};
        let mut end=title.len().min(96);while core::str::from_utf8(&title[..end]).is_err() && end>0 {end-=1;}
        next.add(url,&title[..end])
    };
    match result {
        Ok(())=>if crate::storage::browser_favorites_save(console.current_user.0,console.current_session.0,next.bytes()).is_ok() {
            FAVORITES=next;FAVORITES_ERROR=0;FAVORITES_OFFSET=FAVORITES_OFFSET.min(next.count().saturating_sub(1));
        }else{FAVORITES_ERROR=1;},
        Err(infinity_browser_core::favorites::Error::Full)=>FAVORITES_ERROR=4,
        Err(_)=>FAVORITES_ERROR=5,
    }
    crate::runtime::browser::favorites_presentation(&*(&raw const FAVORITES),FAVORITES_OFFSET,FAVORITES_ERROR);
}}

// ------------------------=
// FUNC: request_access
// DESC: Opens the browser immediately under the signed-in user's Network Settings policy.
// ------------------=
pub(super) fn request_access(console:&mut ConsoleRuntime,url:&[u8]) {
    crate::runtime::browser::chrome_menu_presentation(0,0);
    favorites_sync(console);
    let view=crate::runtime::browser::presentation();
    crate::runtime::browser::settings_presentation(view.settings,false,view.settings_notice,false);
    if url.len()>2048 {return;}
    let mut consent=Launch{owner:SecurityIdentity(console.current_session.0),url:[0;2048],length:url.len(),stage:0,size:(0,0)};
    consent.url[..url.len()].copy_from_slice(url);
    unsafe {CONSENT=Some(consent);}
    if console.mode!=ConsoleMode::Desktop {console.enter_desktop();}
    console.store_active_app_window();console.desktop_app=DesktopAppKind::Browser;
    console.browser_window.visible=true;console.load_active_app_window();
    console.ai_chat_focus=0;console.shell_menu=0;
    approve_access(console);
}

// ------------------------=
// FUNC: approve_access
// DESC: Retries normal browser launch without overriding Network Settings restrictions.
// ------------------=
fn approve_access(console:&mut ConsoleRuntime) {unsafe {
    let Some(consent)=(&*(&raw const CONSENT)).as_ref() else {return;};
    if consent.owner!=SecurityIdentity(console.current_session.0) {CONSENT=None;crate::runtime::browser::permission_presentation(0);return;}
    let mut command=[0u8;2056];command[..8].copy_from_slice(b"browser ");
    command[8..8+consent.length].copy_from_slice(&consent.url[..consent.length]);let length=8+consent.length;
    crate::runtime::browser::permission_presentation(0);
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
    let layout=Layout::new(bounds.width,bounds.height,scale as u32)?.with_favorites(crate::runtime::browser::presentation().settings.favorites);
    Some((layout.content.width,layout.content.height))
}

// ------------------------=
// FUNC: execute
// DESC: Opens a native browser only for an active session already holding the required network capabilities.
// ------------------=
pub(super) fn execute(console:&mut ConsoleRuntime,command:&[u8])->bool {
    if command==b"browser status" {
        let (state,failure,generation,peak)=crate::runtime::browser::status();
        console.output.write_number(b"Browser worker state: ",state as u64);
        console.output.write_number(b"Browser failure: ",failure as u64);
        console.output.write_number(b"Browser generation: ",generation);
        console.output.write_number(b"Browser peak bytes: ",peak);
        console.output.write_number(b"Browser failed allocation bytes: ",crate::runtime::browser::INFINITY_BROWSER_FAILED_ALLOCATION.load(core::sync::atomic::Ordering::Acquire) as u64);
        console.output.write_number(b"Browser network failure: ",crate::drivers::browser_network::INFINITY_BROWSER_NETWORK_FAILURE.load(core::sync::atomic::Ordering::Acquire) as u64);
        console.output.write_number(b"Browser page error: ",crate::runtime::browser::presentation().error as u64);
        let (detail,length)=crate::runtime::browser::diagnostic();
        for line in detail[..length].split(|byte|*byte==b'\n').filter(|line|!line.is_empty()) {
            for part in line.chunks(LINE_CAPACITY) {console.output.write_line(part);}
        }
        return true;
    }
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
        runtime.network.browser_authority(&mut runtime.capabilities,owner,true,now)
    }).flatten();
    let Some(caps)=caps else {
        crate::runtime::browser::permission_presentation(2);
        console.output.write_line(b"Browser access unavailable. Check your session and Network Settings.");return true;
    };
    let size=viewport(console);
    let result=match size {
        Some((w,h)) if w<=2048 && h<=2048=>unsafe {crate::runtime::browser::start(owner,caps)},
        _=>Err(infinity_browser_core::startup::Error::Viewport),
    };
    crate::runtime::browser::launch_presentation(url,result.err());
    if let Err(error)=result {
        console.output.write_number(b"Native browser startup failure: ",error as u64);return true;
    }
    let size=size.unwrap();
    unsafe { CONSENT=None; }
    crate::runtime::browser::permission_presentation(0);
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
    crate::runtime::browser::chrome_menu_presentation(0,0);
    NEW_TABS=0;
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
    let settings=crate::runtime::browser::presentation();
    if settings.chrome_menu!=0 {
        let count=if settings.chrome_menu==1 {3}else{1};
        match key {
            ConsoleKey::Escape=>crate::runtime::browser::chrome_menu_presentation(0,0),
            ConsoleKey::Down|ConsoleKey::Tab(false)=>crate::runtime::browser::chrome_menu_presentation(settings.chrome_menu,(settings.menu_focus+1)%count),
            ConsoleKey::Up|ConsoleKey::Tab(true)=>crate::runtime::browser::chrome_menu_presentation(settings.chrome_menu,(settings.menu_focus+count-1)%count),
            ConsoleKey::Enter|ConsoleKey::Character(b' ')=>menu_action(console,settings.chrome_menu,settings.menu_focus),
            _=>{},
        }return;
    }
    if settings.settings_open {
        if matches!(key,ConsoleKey::Escape) {settings_action(console,6);}
        else if matches!(key,ConsoleKey::Enter|ConsoleKey::Character(b' ')) {settings_action(console,settings.settings_focus);}
        else if let ConsoleKey::Tab(reverse)=key {
            let focus=(settings.settings_focus+if reverse {6}else{1})%7;
            let (height,scale)=settings_dimensions(console);
            let r=infinity_browser_core::settings::control(infinity_browser_core::Viewport{x:0,y:0,width:760*scale,height},scale,focus);
            crate::runtime::browser::settings_position(focus,(r.y as u32+40*scale).saturating_sub(height));
        }
        return;
    }
    if let ConsoleKey::Shortcut(value)=key {
        let mut command=abi::Command::empty();
        match value {
            b'r'|b'R'=>{command.kind=abi::RELOAD;},
            b'd'|b'D'=>{toggle_favorite(console);return;},
            b't'|b'T'=>{new_tab(console);return;},
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
fn navigate_address(console:&mut ConsoleRuntime) {
    let view=crate::runtime::browser::presentation();
    let (bytes,length)=if view.address_focused {(&view.edit,view.edit_length)}else{(&view.address,view.address_length)};
    let mut command=abi::Command::empty();command.kind=abi::NAVIGATE;
    let Ok(input)=core::str::from_utf8(&bytes[..length]) else{return;};
    let Ok((_,length))=infinity_browser_core::omnibox::resolve(input,
        view.settings.search_prefix(),&mut command.text) else{return;};
    command.length=length as u32;
    let retained=unsafe {
        if let Some(launch)=(&mut *(&raw mut LAUNCH)).as_mut().filter(|launch|
            launch.owner==SecurityIdentity(console.current_session.0) && launch.stage<3) {
            launch.url[..length].copy_from_slice(&command.text[..length]);
            launch.length=length;
            // Preserve OPEN during startup; otherwise retain NAVIGATE until
            // the worker accepts it. Older admitted input drains first.
            launch.stage=if launch.stage==0 {0}else{1};
            true
        } else {false}
    };
    if retained {
        crate::runtime::browser::launch_presentation(&command.text[..length],None);
        crate::runtime::browser::focus_address(false);poll(console);
    } else {
        request_access(console,&command.text[..length]);
    }
}

// ------------------------=
// FUNC: new_tab
// DESC: Retains add-tab gestures across worker startup and mailbox pressure instead of silently discarding clicks.
// ------------------=
fn new_tab(console:&mut ConsoleRuntime) {
    let view=crate::runtime::browser::presentation();
    crate::runtime::browser::settings_presentation(view.settings,false,view.settings_notice,false);
    unsafe {
        if (&*(&raw const LAUNCH)).as_ref().is_some_and(|launch|
            launch.owner==SecurityIdentity(console.current_session.0) && launch.stage<3) {
            NEW_TABS=NEW_TABS.saturating_add(1).min(8);
            poll(console);
            return;
        }
    }
    request_access(console,b"https://example.com/");
}

// ------------------------=
// FUNC: menu_action
// DESC: Routes visible menu entries to existing browser commands and the same persistent settings page.
// ------------------=
fn menu_action(console:&mut ConsoleRuntime,menu:u8,index:usize) {
    crate::runtime::browser::chrome_menu_presentation(0,0);
    if menu==2 && index==0 {
        let v=crate::runtime::browser::presentation();crate::runtime::browser::settings_position(6,0);
        crate::runtime::browser::settings_presentation(v.settings,true,v.settings_notice,false);
    } else if menu==1 {
        let v=crate::runtime::browser::presentation();crate::runtime::browser::settings_presentation(v.settings,false,v.settings_notice,false);
        match index {0=>new_tab(console),1=>key(console,ConsoleKey::Shortcut(b'w')),2=>console.close_desktop_app(),_=>{}}
    }
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
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32).map(|layout|layout.with_tab_count(crate::runtime::browser::presentation().tab_count).with_favorites(crate::runtime::browser::presentation().settings.favorites)) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    let view=crate::runtime::browser::presentation();
    for (r,menu) in [(layout.file_menu,1),(layout.settings_menu,2)] {
        if r.local(x,y).is_some() {crate::runtime::browser::focus_address(false);
            crate::runtime::browser::chrome_menu_presentation(if view.chrome_menu==menu {0}else{menu},0);return true;}
    }
    if view.chrome_menu!=0 {
        for index in 0..3 {if layout.menu_item(view.chrome_menu,index).is_some_and(|r|r.local(x,y).is_some()) {
            menu_action(console,view.chrome_menu,index);return true;
        }}
        crate::runtime::browser::chrome_menu_presentation(0,0);return true;
    }
    if layout.menu.local(x,y).is_some() {
        crate::runtime::browser::settings_position(6,0);
        crate::runtime::browser::settings_presentation(view.settings,!view.settings_open,view.settings_notice,false);return true;
    }
    if !view.settings_open && view.permission!=0 {
        if layout.download_save.local(x,y).is_some() {approve_access(console);}
        else if layout.download_discard.local(x,y).is_some() {console.close_desktop_app();}
        else if layout.close.local(x,y).is_some() {console.close_desktop_app();}
        return true;
    }
    if !view.settings_open && view.download_state!=0 && layout.download_card.local(x,y).is_some() {
        if layout.download_save.local(x,y).is_some() {save_download(console);}
        if layout.download_discard.local(x,y).is_some() {unsafe {DOWNLOAD=None;}crate::runtime::browser::download_presentation(&[],0);}
        return true;
    }
    let mut command=abi::Command::empty();
    let view=crate::runtime::browser::presentation();
    if view.settings_open && layout.content.local(x,y).is_some() {
        let content=infinity_browser_core::Viewport{y:layout.content.y-view.settings_scroll as i32,..layout.content};
        for index in 0..7 {if infinity_browser_core::settings::control(content,scale as u32,index).local(x,y).is_some() {
            settings_action(console,index);poll(console);break;
        }}return true;
    }
    if layout.favorites.local(x,y).is_some() {
        for index in 0..layout.favorite_slots() {
            if layout.favorite_item(index).is_some_and(|r|r.local(x,y).is_some()) {
                if favorites_sync(console) {unsafe {
                    if let Some((url,_))=(&*(&raw const FAVORITES)).get(FAVORITES_OFFSET+index) {
                        let mut destination=[0u8;2048];let length=url.len();destination[..length].copy_from_slice(url);
                        request_access(console,&destination[..length]);
                    }
                }}return true;
            }
        }
    }
    for index in 0..view.tab_count {
        let Some((tab,close))=layout.tab(index,view.tab_count) else {continue;};
        if tab.local(x,y).is_some_and(|(x,y)|infinity_browser_core::tab_style::contains(tab.width,tab.height,x,y)) {
            crate::runtime::browser::settings_presentation(view.settings,false,view.settings_notice,false);
            if close.local(x,y).is_some() && view.tab_count==1 {console.close_desktop_app();return true;}
            command.kind=if close.local(x,y).is_some(){abi::TAB_CLOSE}else{abi::TAB_SELECT};
            command.a=view.tabs[index].id;
            if enqueue(console,command) {poll(console);}
            return true;
        }
    }
    let Some(control)=layout.hit(x,y) else{return false;};
    match control {
        Control::Favorite=>{toggle_favorite(console);return true;},
        Control::FavoritesPrevious|Control::FavoritesNext=>{unsafe {
            let slots=layout.favorite_slots();
            if control==Control::FavoritesPrevious {FAVORITES_OFFSET=FAVORITES_OFFSET.saturating_sub(slots);}
            else if FAVORITES_OFFSET+slots<view.favorite_count {FAVORITES_OFFSET+=slots;}
            favorites_sync(console);
        }return true;},
        Control::Address=>{crate::runtime::browser::settings_presentation(view.settings,false,view.settings_notice,false);crate::runtime::browser::focus_address(true);},
        Control::Go=>{crate::runtime::browser::settings_presentation(view.settings,false,view.settings_notice,false);navigate_address(console);},
        Control::Back=>{if view.settings_open {settings_action(console,6);return true;}if view.history&1!=0 {command.kind=abi::BACK;}},
        Control::Forward=>{if view.history&2!=0 {command.kind=abi::FORWARD;}},
        Control::Reload=>{
            if unsafe {LAUNCH.is_none()} {navigate_address(console);return true;}
            command.kind=abi::RELOAD;
        },
        Control::NewTab=>{new_tab(console);return true;},
        Control::Minimize=>console.minimize_desktop_app(),
        Control::Maximize=>console.toggle_window_maximized(5),
        Control::Close=>console.close_desktop_app(),
        Control::Content=>{crate::runtime::browser::focus_address(false);return false;},
        Control::Menu=>{
            crate::runtime::browser::settings_presentation(view.settings,!view.settings_open,0,false);
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
    let view=crate::runtime::browser::presentation();
    if view.chrome_menu!=0 {return true;}
    if view.settings_open {
        let (height,scale)=settings_dimensions(console);
        let limit=(320*scale).saturating_sub(height);
        let next=(view.settings_scroll as i32-vertical as i32*32*scale as i32).clamp(0,limit as i32) as u32;
        crate::runtime::browser::settings_position(view.settings_focus,next);return true;
    }
    if vertical==0 {return false;}
    let state=console.browser_window_state();
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32).map(|l|l.with_favorites(crate::runtime::browser::presentation().settings.favorites)) else{return false;};
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
    if crate::runtime::browser::presentation().settings_open {return false;}
    if crate::runtime::browser::presentation().permission!=0 {return false;}
    let captured=unsafe {(&*(&raw const POINTER)).captured()};
    if capture_only && !captured {return false;}
    let state=console.browser_window_state();
    if !state.visible {return false;}
    let system=SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height);
    let bounds=system.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized).window;
    let scale=system.scale().max(1).min((bounds.width as usize/760).max(1));
    let Some(layout)=Layout::new(bounds.width,bounds.height,scale as u32).map(|l|l.with_favorites(crate::runtime::browser::presentation().settings.favorites)) else{return false;};
    let x=(console.system.framebuffer_width as i64*i64::from(console.pointer_x)/1000) as i32-bounds.x;
    let y=(console.system.framebuffer_height as i64*i64::from(console.pointer_y)/1000) as i32-bounds.y;
    let view=crate::runtime::browser::presentation();
    let mut hover=(0,false);
    if view.chrome_menu!=0 {
        for index in 0..3 {if layout.menu_item(view.chrome_menu,index).is_some_and(|r|r.local(x,y).is_some()) {
            crate::runtime::browser::chrome_menu_presentation(view.chrome_menu,index);break;
        }}return false;
    }
    if !captured {for index in 0..view.tab_count {
        let Some((tab,close))=layout.tab(index,view.tab_count) else {continue;};
        if tab.local(x,y).is_some_and(|(x,y)|infinity_browser_core::tab_style::contains(tab.width,tab.height,x,y)) {
            hover=(view.tabs[index].id,close.local(x,y).is_some());break;
        }
    }}
    crate::runtime::browser::hover_tab(hover.0,hover.1);
    let favorite_hover=if !captured {(0..layout.favorite_slots()).find(|index|
        view.favorite_offset+index<view.favorite_count && layout.favorite_item(*index).is_some_and(|r|r.local(x,y).is_some()))}else{None};
    crate::runtime::browser::hover_favorite(favorite_hover.map(|i|view.favorite_offset+i).unwrap_or(usize::MAX));
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
    favorites_sync(console);
    unsafe {
        let slot=&mut *(&raw mut LAUNCH);
        let Some(launch)=slot.as_mut() else {return;};
        if launch.stage==2 && DOWNLOAD.is_none() && launch.owner==SecurityIdentity(console.current_session.0) {
            if let Some(value)=crate::runtime::browser::take_download(launch.owner) {
                crate::runtime::browser::download_presentation(&value.name[..value.name_length],1);DOWNLOAD=Some(value);
            }
        }
        if launch.owner!=SecurityIdentity(console.current_session.0) {
            NEW_TABS=0;
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
                    if NEW_TABS>0 {
                        command.kind=abi::TAB_CREATE;
                        if crate::runtime::browser::submit(launch.owner,command).is_ok() {NEW_TABS-=1;}
                        return;
                    }
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
