//! SNTP client. State is owned exclusively by the serialized native NIC pump.
use crate::runtime::{ai::clock,network::{NetworkRuntime,types::*,wire::Datagram}};
#[path="ntp_wire.rs"] mod wire;
const SERVER:[u8;4]=[162,159,200,1];
const SERVICE:u32=0x4e5450;
static mut NEXT:u64=0;
static mut PENDING:Option<(u64,[u8;8],u16,[u8;4])>=None;

// ------------------------=
// FUNC: poll
// DESC: Sends one correlated NTP probe after address acquisition, with bounded retry and explicit endpoint policy.
// ------------------=
pub(super) unsafe fn poll(net:&mut NetworkRuntime,address:[u8;4],ns:u64){
    if ns<NEXT{return}
    PENDING=None;
    NEXT=ns.saturating_add(16_000_000_000);
    if address==[0;4] || !net.profiles.active().is_some_and(|p|p.internet_allowed){return}
    let Some(gateway)=super::https::default_gateway(net) else{return};
    let mut token=[0;8];
    if !super::ssh_entropy::fill(&mut token){return}
    let port=49152+(u16::from_be_bytes([token[0],token[1]])&16383);
    if net.policy.evaluate(Subject::Service(SERVICE),Direction::Outbound,Some(2),
        Endpoint{address:IpAddress::V4(address),port},Endpoint{address:IpAddress::V4(SERVER),port:123},
        TransportProtocol::Datagram,ns/1_000_000_000).action!=PolicyAction::Allow{return}
    let mut bytes=[0;48];bytes[0]=0x23;bytes[40..48].copy_from_slice(&token);
    if net.wire.send(SERVER,gateway,port,123,&bytes,ns/1_000_000_000).is_ok(){PENDING=Some((ns,token,port,address));}
    else{NEXT=ns.saturating_add(2_000_000_000);}
}
// ------------------------=
// FUNC: receive
// DESC: Accepts only matching synchronized server replies and applies an era-aware correction without changing monotonic time.
// ------------------=
pub(super) unsafe fn receive(p:&Datagram,ns:u64)->bool{
    let Some((sent,token,port,local))=PENDING else{return false};
    if p.source!=SERVER || p.destination!=local || p.destination_port!=port || p.source_port!=123{return false}
    let b=&p.bytes[..p.length];
    if b.len()<48 || b[24..32]!=token || ns.saturating_sub(sent)>5_000_000_000{return true}
    PENDING=None;
    if b[1]==0 {NEXT=ns.saturating_add(3_600_000_000_000);return true}
    let Some(current)=clock::SYSTEM.utc_nanoseconds(ns) else{return true};
    let Some(delta)=wire::correction(b,current,ns.saturating_sub(sent)) else{return true};
    let mut settings=clock::SYSTEM.settings();
    settings.correction_seconds+=delta;
    if clock::SYSTEM.configure(settings){NEXT=ns.saturating_add(3_600_000_000_000);}
    true
}
