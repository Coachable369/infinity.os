//! Host-only measurement of the unchanged production transport, not a throughput claim for installed storage.
#[path="../wire-trust-fixture.rs"]
mod fixture;
use fixture::Fixture;
// ------------------------=
// FUNC: step
// DESC: Executes one millisecond opportunity with four-frame NIC bounds and one production poll per node.
// ------------------=
fn step(a:&mut Fixture,b:&mut Fixture,ms:&mut u64){
    let now=*ms/1000;
    carry(a,b,now);carry(b,a,now);
    for peer in [&mut *a,&mut *b]{for _ in 0..4{let Some(p)=peer.network.wire.receive_datagram()else{break};peer.network.connections.deliver_datagram(p,&mut peer.network.policy,&peer.capabilities,now).unwrap();}peer.transport.poll(&mut peer.nodes,&mut peer.network,&peer.capabilities,now);}
    *ms+=1;
}
// ------------------------=
// FUNC: carry
// DESC: Moves no more than four real Ethernet frames in one directed fixture opportunity.
// ------------------=
fn carry(from:&mut Fixture,to:&mut Fixture,now:u64){for _ in 0..4{let Some(f)=from.network.wire.peek_transmit().copied()else{break};from.network.wire.complete_transmit();let _=to.network.wire.ingest(&f.bytes[..f.length],now);}}
// ------------------------=
// FUNC: advance
// DESC: Advances only the fixture clock, retaining actual production second-resolution transport gates.
// ------------------=
fn advance(a:&mut Fixture,b:&mut Fixture,ms:&mut u64,seconds:u64){for _ in 0..seconds*1000{step(a,b,ms)}}
// ------------------------=
// FUNC: measure_established_data_cadence
// DESC: Measures successful authenticated payload delivery with a 1ms caller without modifying runtime budgets.
// ------------------=
#[test]
fn measure_established_data_cadence(){std::thread::Builder::new().stack_size(32*1024*1024).spawn(measure_case).unwrap().join().unwrap();}
// ------------------------=
// FUNC: measure_case
// DESC: Establishes real signed trust/session state, then counts exact ordered encrypted payloads over ten virtual seconds.
// ------------------=
fn measure_case(){
    let mut a=fixture::configured([2,0,0,0,0,1],[0x91;32]);let mut b=fixture::configured([2,0,0,0,0,2],[0x92;32]);let mut ms=0;
    advance(&mut a,&mut b,&mut ms,8);let aid=a.nodes.local_id().unwrap();let bid=b.nodes.local_id().unwrap();let link=a.transport.inspect(a.connection,a.owner).unwrap();
    let tx=a.transport.trust.begin(&mut a.nodes,link,0,false,ms/1000).unwrap();advance(&mut a,&mut b,&mut ms,16);
    let code=a.transport.trust.verification(aid,bid).unwrap().code;a.transport.trust.confirm(&mut a.nodes,tx,code,true,ms/1000).unwrap();b.transport.trust.confirm(&mut b.nodes,tx,code,true,ms/1000).unwrap();advance(&mut a,&mut b,&mut ms,6);
    a.transport.trust.begin(&mut a.nodes,link,0,true,ms/1000).unwrap();advance(&mut a,&mut b,&mut ms,16);assert!(a.transport.trust.session(bid).is_some());
    let start=ms;let mut admitted=0u64;let mut received=0u64;
    while ms<start+10_000{
        let mut payload=[0;64];payload[..8].copy_from_slice(&admitted.to_le_bytes());
        if a.transport.trust.send_data(&mut a.nodes,bid,&payload,false,ms/1000).is_ok(){admitted+=1;}
        step(&mut a,&mut b,&mut ms);
        if let Some(data)=b.transport.trust.receive_data(){assert_eq!(&data.bytes[..8],&received.to_le_bytes());received+=1;}
    }
    assert!(received>0);assert!(received<=10);
    println!("MEASURE production transport: polls=10000 virtual_ms=10000 admitted={admitted} delivered={received} payload_bytes={}",received*64);
}
