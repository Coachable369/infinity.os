//! Desktop-thread forecast actor. Visibility opts in; the network profile and session govern every transfer.
use super::*;
use crate::{drivers::https,runtime::execution::SecurityIdentity};
const PATH:&str="/v1/forecast?latitude=32.52&longitude=-93.73&current=temperature_2m,weather_code&temperature_unit=fahrenheit";
static mut PENDING:Option<(SecurityIdentity,https::Ticket)>=None;
static mut NEXT:u64=0;
static mut REPORT:Option<(i16,u8,u64)>=None;

// ------------------------=
// FUNC: reading
// DESC: Copies the last successful public forecast, retaining its observation age while offline.
// ------------------=
pub(crate) fn reading()->Option<(i16,u8,u64)>{unsafe{REPORT}}
// ------------------------=
// FUNC: parse
// DESC: Extracts finite, bounded readings from the provider's current observation object, excluding units and forecasts.
// ------------------=
fn parse(body:&[u8])->Option<(i16,u8)>{
    let text=core::str::from_utf8(body).ok()?;
    let fields=text.split_once("\"current\"")?.1.trim_start().strip_prefix(':')?.trim_start().strip_prefix('{')?.split_once('}')?.0;
    let number=|key:&str|fields.split(',').find_map(|field|{
        let (name,value)=field.split_once(':')?;
        (name.trim().trim_matches('"')==key).then_some(value.trim())
    });
    let temp=number("temperature_2m")?.parse::<f32>().ok()?;
    let code=number("weather_code")?.parse::<u8>().ok()?;
    if !(-150.0..=160.0).contains(&temp)||code>99{return None}
    Some((libm::roundf(temp) as i16,code))
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
        if let Some((actor,ticket))=PENDING{
            if !enabled || actor!=owner{let _=https::cancel_browser(actor,ticket);}
            match https::take_browser(actor,ticket){
                Ok(None)|Err(https::Failure::Busy)=>return,
                Ok(Some(Ok(r))) if enabled && actor==owner && r.status==200=>{
                    if let Some((t,c))=parse(&r.bytes[..r.length]){REPORT=Some((t,c,now));crate::ui::widget_keyframes::invalidate(2);NEXT=now+900;}
                }
                _=>{}
            }
            PENDING=None;
        }
        if !enabled||now<NEXT{return}
        NEXT=now+60;
        let ids=crate::runtime::with_runtime(|r|{
            let active=(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|r.identity.session_nth(i))
                .any(|s|s.id==console.current_session && s.user==console.current_user && s.state==crate::runtime::identity::SessionState::Active);
            r.network.browser_authority(&mut r.capabilities,owner,active,now)
        }).flatten();
        if let Some([c,s,r,d])=ids{
            match https::get_browser(owner,c,s,r,d,"api.open-meteo.com",443,PATH){
                Ok(ticket)=>PENDING=Some((owner,ticket)),
                Err(https::Failure::Busy)=>NEXT=now+8,
                _=>{}
            }
        }
    }
}
