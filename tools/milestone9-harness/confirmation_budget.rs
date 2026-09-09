//! HOST adversarial transport reproduction, not evidence of the installed packet loss.
use super::*;
use crate::node::{types::NodeError,wire_trust::WireState};
// ------------------------=
// FUNC: control_pressure_exhausts_confirmation_retries_before_deadline
// DESC: Drives actual wire transmission and native policy/receive admission while invalid signed packets consume the receiver's bounded control budget before legitimate confirmation retries.
// ------------------=
#[test]
fn control_pressure_exhausts_confirmation_retries_before_deadline(){
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(case).unwrap().join().unwrap();
}
// ------------------------=
// FUNC: case
// DESC: Reproduces twelve real confirmation emissions followed by silence with remaining ceremony time, without changing runtime budgets or trusting forged consent.
// ------------------=
fn case(){
    let mut a=fixture::configured([2,0,0,0,0,1],[0xb1;32]);
    let mut b=fixture::configured([2,0,0,0,0,2],[0xb2;32]);let mut now=0;
    advance(&mut a,&mut b,&mut now,8);
    let aid=a.nodes.local_id().unwrap();let bid=b.nodes.local_id().unwrap();
    let link=a.transport.inspect(a.connection,a.owner).unwrap();
    let tx=a.transport.trust.begin(&mut a.nodes,link,0,false,now).unwrap();
    advance(&mut a,&mut b,&mut now,16);
    let av=a.transport.trust.verification(aid,bid).unwrap();
    let bv=b.transport.trust.verification(bid,aid).unwrap();
    a.transport.trust.confirm(&mut a.nodes,tx,av.code,true,now).unwrap();
    advance(&mut a,&mut b,&mut now,6);
    assert_eq!(b.transport.trust.lifecycle(aid).unwrap().1,WireState::RemotelyConfirmed);
    b.transport.trust.confirm(&mut b.nodes,tx,bv.code,true,now).unwrap();
    assert_eq!(b.transport.trust.lifecycle(aid).unwrap().1,WireState::Confirmed);
    let initial=now;let mut emitted=0;let mut blocked=0;
    for _ in 0..40 {
        for _ in 0..1000 {
            // At most four real Ethernet frames per endpoint per NIC opportunity.
            for _ in 0..4 {
                let Some(frame)=b.network.wire.peek_transmit().copied()else{break};
                b.network.wire.complete_transmit();
                if frame.length<51||&frame.bytes[42..50]!=b"IN9A0001"{continue}
                a.network.wire.ingest(&frame.bytes[..frame.length],now).unwrap();
                while let Some(packet)=a.network.wire.receive_datagram(){
                    if packet.length>8&&packet.bytes[8]==5 {
                        emitted+=1;
                        let mut forged=packet;forged.bytes[forged.length-1]^=1;
                        a.network.connections.deliver_datagram(forged,&mut a.network.policy,&a.capabilities,now).unwrap();
                        a.transport.poll(&mut a.nodes,&mut a.network,&a.capabilities,now);
                        assert_eq!(a.transport.last_error,Some(NodeError::SignatureInvalid));
                        a.network.connections.deliver_datagram(packet,&mut a.network.policy,&a.capabilities,now).unwrap();
                        a.transport.poll(&mut a.nodes,&mut a.network,&a.capabilities,now);
                        assert_eq!(a.transport.last_error,Some(NodeError::ResourceLimit));blocked+=1;
                    }else{a.network.connections.deliver_datagram(packet,&mut a.network.policy,&a.capabilities,now).unwrap();}
                }
            }
            for _ in 0..4 {
                let Some(frame)=a.network.wire.peek_transmit().copied()else{break};
                a.network.wire.complete_transmit();
                if frame.length<51||&frame.bytes[42..50]!=b"IN9A0001"{continue}
                b.network.wire.ingest(&frame.bytes[..frame.length],now).unwrap();
                while let Some(packet)=b.network.wire.receive_datagram(){b.network.connections.deliver_datagram(packet,&mut b.network.policy,&b.capabilities,now).unwrap();}
            }
            a.transport.poll(&mut a.nodes,&mut a.network,&a.capabilities,now);
            b.transport.poll(&mut b.nodes,&mut b.network,&b.capabilities,now);
        }
        now+=1;
    }
    assert_eq!((emitted,blocked),(12,12));assert_eq!(now-initial,40);
    assert!(now+30<av.expires.min(bv.expires));
    assert_eq!(a.transport.trust.lifecycle(bid).unwrap().1,WireState::LocallyConfirmed);
    assert_ne!(a.nodes.discovered_nodes()[0].unwrap().trust,TrustState::Trusted);
    assert_eq!(b.nodes.discovered_nodes()[0].unwrap().trust,TrustState::Trusted);
    a.transport.poll(&mut a.nodes,&mut a.network,&a.capabilities,av.expires);
    assert_eq!(a.transport.trust.lifecycle(bid).unwrap().1,WireState::Expired);
}
