use super::*;
use crate::node::{types::NodeError,wire_trust::WireState};
// ------------------------=
// FUNC: cold_simultaneous_reconnect
// DESC: Exercises independent persisted identities and simultaneous authenticated session opens through actual bounded Ethernet transport.
// ------------------=
#[test]
fn cold_simultaneous_reconnect(){std::thread::Builder::new().stack_size(32*1024*1024).spawn(case).unwrap().join().unwrap();}
// ------------------------=
// FUNC: case
// DESC: Creates real bilateral pairing receipts, cold restores without traffic keys, then checks authenticated collision resolution in both offer orders.
// ------------------=
fn case(){
    for reverse in [false,true] {
    let mut a=fixture::configured([2,0,0,0,0,1],[0x31;32]);let mut b=fixture::configured([2,0,0,0,0,2],[0x32;32]);let mut now=0;advance(&mut a,&mut b,&mut now,8);
    let aid=a.nodes.local_id().unwrap();let bid=b.nodes.local_id().unwrap();let link=a.transport.inspect(a.connection,a.owner).unwrap();
    let tx=a.transport.trust.begin(&mut a.nodes,link,0,false,now).unwrap();advance(&mut a,&mut b,&mut now,16);
    let av=a.transport.trust.verification(aid,bid).unwrap();let bv=b.transport.trust.verification(bid,aid).unwrap();
    a.transport.trust.confirm(&mut a.nodes,tx,av.code,true,now).unwrap();b.transport.trust.confirm(&mut b.nodes,tx,bv.code,true,now).unwrap();advance(&mut a,&mut b,&mut now,8);
    let ast=a.nodes.encode_state().unwrap();let bst=b.nodes.encode_state().unwrap();
    a=fixture::configured([2,0,0,0,0,1],[0x41;32]);b=fixture::configured([2,0,0,0,0,2],[0x42;32]);a.nodes.restore_state(&ast).unwrap();b.nodes.restore_state(&bst).unwrap();now=0;advance(&mut a,&mut b,&mut now,8);
    assert_eq!(a.nodes.paired_digest(bid),b.nodes.paired_digest(aid));
    let al=a.transport.inspect(a.connection,a.owner).unwrap();let bl=b.transport.inspect(b.connection,b.owner).unwrap();
    let at=a.transport.trust.begin(&mut a.nodes,al,0,true,now).unwrap();let bt=b.transport.trust.begin(&mut b.nodes,bl,0,true,now).unwrap();
    let deadline=a.transport.trust.lifecycle(bid).unwrap().2.min(b.transport.trust.lifecycle(aid).unwrap().2);
    let ap=a.transport.trust.outgoing(a.connection,now).unwrap();let bp=b.transport.trust.outgoing(b.connection,now).unwrap();
    let mut forged=bp;forged.bytes[forged.length as usize-1]^=1;
    let before=a.transport.trust.lifecycle(bid).unwrap();assert_eq!(a.transport.trust.ingest(&mut a.nodes,al,&forged.bytes[..forged.length as usize],now),Err(NodeError::SignatureInvalid));assert_eq!(a.transport.trust.lifecycle(bid).unwrap(),before);
    if reverse {b.transport.trust.ingest(&mut b.nodes,bl,&ap.bytes[..ap.length as usize],now).unwrap();a.transport.trust.ingest(&mut a.nodes,al,&bp.bytes[..bp.length as usize],now).unwrap();}
    else{a.transport.trust.ingest(&mut a.nodes,al,&bp.bytes[..bp.length as usize],now).unwrap();b.transport.trust.ingest(&mut b.nodes,bl,&ap.bytes[..ap.length as usize],now).unwrap();}
    let winning=if aid.0<bid.0{at}else{bt};
    assert_eq!(a.transport.trust.lifecycle(bid).unwrap().0,winning);assert_eq!(b.transport.trust.lifecycle(aid).unwrap().0,winning);
    if aid.0<bid.0 {assert_eq!(a.transport.trust.ingest(&mut a.nodes,al,&bp.bytes[..bp.length as usize],now),Err(NodeError::ReplayDetected));b.transport.trust.ingest(&mut b.nodes,bl,&ap.bytes[..ap.length as usize],now).unwrap();}
    else{assert_eq!(b.transport.trust.ingest(&mut b.nodes,bl,&ap.bytes[..ap.length as usize],now),Err(NodeError::ReplayDetected));a.transport.trust.ingest(&mut a.nodes,al,&bp.bytes[..bp.length as usize],now).unwrap();}
    advance(&mut a,&mut b,&mut now,16);assert!(now<deadline);
    let ah=a.transport.trust.session(bid).unwrap();let bh=b.transport.trust.session(aid).unwrap();
    assert_eq!(a.transport.trust.lifecycle(bid).unwrap().1,WireState::Established);assert_eq!(b.transport.trust.lifecycle(aid).unwrap().1,WireState::Established);
    let ar=a.nodes.sessions().iter().flatten().find(|s|s.id==ah).unwrap().protocol_reference;let br=b.nodes.sessions().iter().flatten().find(|s|s.id==bh).unwrap().protocol_reference;assert_eq!(ar,br);
    if aid.0<bid.0 {assert_eq!(a.transport.trust.ingest(&mut a.nodes,al,&bp.bytes[..bp.length as usize],now),Err(NodeError::ReplayDetected));}else{assert_eq!(b.transport.trust.ingest(&mut b.nodes,bl,&ap.bytes[..ap.length as usize],now),Err(NodeError::ReplayDetected));}
    assert_eq!(a.transport.trust.session(bid),Some(ah));assert_eq!(b.transport.trust.session(aid),Some(bh));
    a.transport.trust.send_data(&mut a.nodes,bid,&[1,7,9],false,now).unwrap();advance(&mut a,&mut b,&mut now,4);assert_eq!(&b.transport.trust.receive_data().unwrap().bytes[..3],&[1,7,9]);
    }
}
