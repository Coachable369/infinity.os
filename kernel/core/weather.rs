//! Desktop-thread forecast actor. Visibility opts in; the network profile and session govern every transfer.
use super::*;
use crate::{drivers::https,runtime::execution::SecurityIdentity};
#[path="weather_place.rs"] mod place;
use place::{Text,Place};
#[derive(Clone,Copy,PartialEq,Eq)]
enum Request{City,Detect,Forecast}
static mut PENDING:Option<(SecurityIdentity,https::Ticket,Request)>=None;
static mut SESSION:Option<SecurityIdentity>=None;
static mut NEXT:u64=0;
static mut REPORT:Option<(i16,u8,u64,bool)>=None;
static mut PLACE:Option<Place>=None;
static mut QUERY:Text<54>=Text::EMPTY;
static mut SETUP:bool=true;
static mut FOCUS:bool=false;
static mut LOOKUP:Option<Request>=None;
static mut MESSAGE:&[u8]=b"Enter your city";

// ------------------------=
// FUNC: view
// DESC: Copies the visible setup state without exposing mutable references to the painter.
// ------------------=
pub(crate) fn view()->(Option<(i16,u8,u64,bool)>,Text<64>,bool,Text<54>,bool,&'static [u8]){
    unsafe{(REPORT,PLACE.map(|p|p.city).unwrap_or(Text::EMPTY),SETUP,QUERY,FOCUS,MESSAGE)}
}
// ------------------------=
// FUNC: configure
// DESC: Opens the city field from the widget context menu.
// ------------------=
pub(super) fn configure(){unsafe{if LOOKUP.is_none(){SETUP=true;FOCUS=true;QUERY=Text::EMPTY;MESSAGE=b"Enter your city";crate::ui::widget_keyframes::invalidate(2);}}}
// ------------------------=
// FUNC: blur
// DESC: Releases the city field before another control receives a pointer press.
// ------------------=
pub(super) fn blur(){unsafe{FOCUS=false}}
// ------------------------=
// FUNC: click
// DESC: Routes weather field, submit and explicit approximate-location actions.
// ------------------=
pub(super) fn click(x:i32,y:i32)->bool{unsafe{
    if LOOKUP.is_some(){return false}
    if !SETUP && y<32 && x>=65{SETUP=true;MESSAGE=b"Enter your city";QUERY=Text::EMPTY;FOCUS=true;}
    else if SETUP && (33..59).contains(&y) && (5..95).contains(&x){FOCUS=true;}
    else if SETUP && (66..92).contains(&y) && ((5..45).contains(&x)||(50..95).contains(&x)){if x>=50{LOOKUP=Some(Request::Detect);}else if QUERY.len>0{LOOKUP=Some(Request::City);}NEXT=0;}
    else{return false}
    crate::ui::widget_keyframes::invalidate(2);true
}}
// ------------------------=
// FUNC: key
// DESC: Edits the focused field and submits a lookup only after explicit confirmation.
// ------------------=
pub(super) fn key(k:ConsoleKey)->bool{unsafe{
    if !FOCUS || !SETUP || LOOKUP.is_some() || crate::ui::desktop_widgets::current().visible&4==0{return false}
    match k{ConsoleKey::Character(c) if (32..=126).contains(&c)=>{let mut q=QUERY;let _=q.add(&[c]);QUERY=q;},
    ConsoleKey::Backspace=>QUERY.len=QUERY.len.saturating_sub(1),
    ConsoleKey::Escape=>{FOCUS=false;SETUP=PLACE.is_none();},
    ConsoleKey::Enter if QUERY.len>0=>{LOOKUP=Some(Request::City);NEXT=0;FOCUS=false;},_=>return false}
    crate::ui::widget_keyframes::invalidate(2);true
}}
// ------------------------=
// FUNC: parse
// DESC: Extracts finite, bounded readings from the provider's current observation object, excluding units and forecasts.
// ------------------=
fn parse(body:&[u8])->Option<(i16,u8,bool)>{
    let text=core::str::from_utf8(body).ok()?;
    let fields=text.split_once("\"current\"")?.1.trim_start().strip_prefix(':')?.trim_start().strip_prefix('{')?.split_once('}')?.0;
    let number=|key:&str|fields.split(',').find_map(|field|{
        let (name,value)=field.split_once(':')?;
        (name.trim().trim_matches('"')==key).then_some(value.trim())
    });
    let temp=number("temperature_2m")?.parse::<f32>().ok()?;
    let code=number("weather_code")?.parse::<u8>().ok()?;
    if !(-150.0..=160.0).contains(&temp)||code>99{return None}
    Some((libm::roundf(temp) as i16,code,match number("is_day")? {"1"=>true,"0"=>false,_=>return None}))
}
// ------------------------=
// FUNC: tick
// DESC: Refreshes a visible forecast every fifteen minutes and releases its exact ticket when hidden or signed out.
// ------------------=
pub(super) fn tick(console:&ConsoleRuntime){
    let owner=SecurityIdentity(console.current_session.0);
    let enabled=matches!(console.mode,ConsoleMode::Desktop|ConsoleMode::SystemMenu|ConsoleMode::Settings|ConsoleMode::AppLauncher)
        && crate::ui::desktop_widgets::current().visible&4!=0;
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000_000;
    unsafe{
        let active=enabled.then_some(owner);
        if SESSION!=active{
            if let Some((a,t,_))=PENDING{let _=https::cancel_browser(a,t);}
            PENDING=None;SESSION=active;REPORT=None;PLACE=None;QUERY=Text::EMPTY;SETUP=true;FOCUS=false;LOOKUP=None;NEXT=0;MESSAGE=b"Enter your city";
            if enabled{let mut saved=[0;120];if let Ok(n)=crate::storage::weather_location(console.current_user.0,owner.0,None,&mut saved){PLACE=Place::decode(&saved[..n]);SETUP=PLACE.is_none();}}
            crate::ui::widget_keyframes::invalidate(2);
        }
        if let Some((actor,ticket,kind))=PENDING{
            match https::take_browser(actor,ticket){
                Ok(None)|Err(https::Failure::Busy)=>return,
                result if enabled && actor==owner=>{
                    if kind==Request::Forecast{
                        if let Ok(Some(Ok(r)))=result{if r.status==200{if let Some((t,c,day))=parse(&r.bytes[..r.length]){REPORT=Some((t,c,now,day));NEXT=now+900;}}}
                    }else{
                        let found=if let Ok(Some(Ok(r)))=result{if r.status==200{place::resolve(&r.bytes[..r.length],kind==Request::Detect)}else{None}}else{None};
                        LOOKUP=None;
                        if let Some(p)=found{let mut scratch=[0;120];if crate::storage::weather_location(console.current_user.0,owner.0,Some(p.encode().value()),&mut scratch).is_ok(){
                            PLACE=Some(p);REPORT=None;SETUP=false;FOCUS=false;NEXT=0;
                        }else{MESSAGE=b"Could not save city";}}
                        else{MESSAGE=b"City not found";}
                    }
                    crate::ui::widget_keyframes::invalidate(2);
                }
                _=>{}
            }
            PENDING=None;
        }
        if !enabled||now<NEXT{return}
        let kind=if let Some(k)=LOOKUP{k}else if PLACE.is_some()&&!SETUP{Request::Forecast}else{return};
        NEXT=now+60;
        let ids=crate::runtime::with_runtime(|r|{
            let active=(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|r.identity.session_nth(i))
                .any(|s|s.id==console.current_session && s.user==console.current_user && s.state==crate::runtime::identity::SessionState::Active);
            r.network.browser_authority(&mut r.capabilities,owner,active,now)
        }).flatten();
        let query=QUERY;
        let city=place::search(query.value());let forecast=PLACE.and_then(Place::path);
        let (host,path)=match kind{Request::Detect=>("ipwho.is","/?fields=success,city,latitude,longitude"),
            Request::City=>("geocoding-api.open-meteo.com",city.as_ref().and_then(|p|core::str::from_utf8(p.value()).ok()).unwrap_or("")),
            Request::Forecast=>("api.open-meteo.com",forecast.as_ref().and_then(|p|core::str::from_utf8(p.value()).ok()).unwrap_or(""))};
        if let Some([c,s,r,d])=ids{if !path.is_empty(){match https::get_browser(owner,c,s,r,d,host,443,path){
            Ok(t)=>{PENDING=Some((owner,t,kind));if kind!=Request::Forecast{MESSAGE=b"Finding location...";crate::ui::widget_keyframes::invalidate(2);}return},
            Err(https::Failure::Busy)=>{NEXT=now+2;return},_=>{}
        }}}
        if kind!=Request::Forecast{LOOKUP=None;MESSAGE=b"Network unavailable";crate::ui::widget_keyframes::invalidate(2);}
    }
}
