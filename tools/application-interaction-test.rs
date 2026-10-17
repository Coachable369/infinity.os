#[path = "../kernel/runtime/ai/application.rs"]
mod application;
#[path = "../kernel/runtime/ai/application_host.rs"]
mod application_host;
#[path = "../kernel/runtime/ai/application_decision.rs"]
mod application_decision;
#[path = "../kernel/ui/editor_compile_intent.rs"]
mod editor_compile_intent;
use application::*;
// ------------------------=
// FUNC: choice
// DESC: Creates a semantic fixture action with typed coordinates and no model authority.
// ------------------=
fn choice(id: u32, risk: Risk) -> Choice {
    let mut action = [0; 48]; action[..15].copy_from_slice(b"board.placeMark");
    let mut label = [0; 96]; label[..11].copy_from_slice(b"upper right");
    Choice { id, action, label, arguments: [0, 2, 1, 0], risk }
}
// ------------------------=
// FUNC: publish
// DESC: Publishes a complete fixture state and legal choices through the public API.
// ------------------=
fn publish(registry: &mut Registry, id: u64, revision: u64, risk: Risk) {
    registry.begin(id, [1;16], [2;16], revision, b"board empty; agent X", true, false).unwrap();
    registry.offer(id, [1;16], [2;16], choice(3, risk)).unwrap();
    registry.commit(id, [1;16], [2;16]).unwrap();
}
// ------------------------=
// FUNC: main
// DESC: Exercises instance isolation, atomic observations, verified results, approvals, stale moves, and bounded recovery.
// ------------------=
fn main() {
    host_packets();
    assert!(editor_compile_intent::requests_opponent(b"create a chess game and play me"));
    assert!(editor_compile_intent::requests_opponent(b"play against me"));
    assert!(!editor_compile_intent::requests_opponent(b"create a chess game"));
    assert!(!editor_compile_intent::requests_opponent(b"explain how you play chess"));
    assert!(!editor_compile_intent::requests_opponent(b"do not play me"));
    assert!(!editor_compile_intent::requests_opponent(b"create chess but do not play me"));
    assert_eq!(application_decision::parse_choice(b"CHOICE:3\n"),Some(3));
    for invalid in [b"CHOICE:0".as_slice(),b"CHOICE:3\nCHOICE:4",b"CHOICE:-1",b"CHOICE:4294967296",b"move(3)",b"done"] {
        assert_eq!(application_decision::parse_choice(invalid),None);
    }
    let state=Snapshot::new();
    assert!(application_decision::decision_prompt(b"play",&state,&mut [0;32]).is_none());
    assert!(application_decision::decision_prompt(b"play",&state,&mut [0;1024]).is_some());
    let mut registry = Box::new(Registry::new());
    let id = registry.register([1;16], [2;16]).unwrap();
    let other = registry.register([1;16], [2;16]).unwrap();
    publish(&mut registry, id, 1, Risk::Reversible);
    assert!(matches!(registry.observe(id,[1;16],[2;16]),Err(Error::Denied)));
    registry.grant(id,[1;16],[2;16],true).unwrap();
    assert!(matches!(registry.observe(id,[3;16],[2;16]),Err(Error::Denied)));
    assert!(matches!(registry.observe(id,[1;16],[3;16]),Err(Error::Denied)));
    assert!(matches!(registry.observe(other,[1;16],[2;16]),Err(Error::Denied)));
    registry.begin(id,[1;16],[2;16],2,b"staged",true,false).unwrap();
    assert_eq!(registry.observe(id,[1;16],[2;16]).unwrap().revision,1);
    let request = registry.propose(id,[1;16],[2;16],1,3,false,10,100).unwrap();
    assert_eq!(registry.propose(id,[1;16],[2;16],1,3,false,10,100),Err(Error::Busy));
    assert_eq!(registry.poll(id,[1;16],[2;16],11),Ok(Some(request)));
    assert_eq!(registry.poll(id,[1;16],[2;16],12),Ok(None));
    assert_eq!(registry.finish(id,[1;16],[2;16],request.token,Outcome::Applied,13),Err(Error::Stale));
    registry.commit(id,[1;16],[2;16]).unwrap();
    let receipt = registry.finish(id,[1;16],[2;16],request.token,Outcome::Applied,14).unwrap();
    assert_eq!(receipt.revision,2);
    assert_eq!(registry.finish(id,[1;16],[2;16],request.token,Outcome::Applied,15),Err(Error::Missing));
    publish(&mut registry,id,3,Risk::Sensitive);
    assert_eq!(registry.propose(id,[1;16],[2;16],2,3,true,20,100),Err(Error::Stale));
    assert_eq!(registry.propose(id,[1;16],[2;16],3,9,true,20,100),Err(Error::Invalid));
    assert_eq!(registry.propose(id,[1;16],[2;16],3,3,false,20,100),Err(Error::Approval));
    registry.propose(id,[1;16],[2;16],3,3,true,20,100).unwrap();
    registry.grant(id,[1;16],[2;16],false).unwrap();
    assert_eq!(registry.result(id,[1;16],[2;16],21).unwrap().unwrap().outcome,Outcome::Cancelled);
    registry.grant(id,[1;16],[2;16],true).unwrap();
    registry.propose(id,[1;16],[2;16],3,3,true,20,100).unwrap();
    registry.poll(id,[1;16],[2;16],21).unwrap();
    assert_eq!(registry.result(id,[1;16],[2;16],101).unwrap().unwrap().outcome,Outcome::Unknown);
    registry.close(id,[1;16],[2;16]).unwrap();
    let relaunched=registry.register([1;16],[2;16]).unwrap();
    assert_ne!(id,relaunched);
    assert!(matches!(registry.observe(id,[1;16],[2;16]),Err(Error::Missing)));
}

// ------------------------=
// FUNC: host_packets
// DESC: Exercises native ABI decoding, session isolation, and request delivery through the actual host adapter.
// ------------------=
fn host_packets() {
    let context=application_host::attach([1;16],[2;16]).unwrap();
    let mut packet=[0u8;2072];
    packet[..4].copy_from_slice(&1u32.to_le_bytes());
    packet[4..8].copy_from_slice(&1u32.to_le_bytes());
    packet[8..16].copy_from_slice(&1u64.to_le_bytes());
    packet[16..20].copy_from_slice(&5u32.to_le_bytes());
    packet[24..29].copy_from_slice(b"ready");
    let mut output=[0;72];
    assert_eq!(application_host::syscall(0x1150,[3;16],&packet,&mut output,1),Err(Error::Denied));
    assert_eq!(application_host::syscall(0x1150,[2;16],&packet[..24],&mut output,1),Err(Error::Invalid));
    application_host::syscall(0x1150,[2;16],&packet,&mut output,1).unwrap();
    let mut offer=[0u8;192];
    offer[..4].copy_from_slice(&1u32.to_le_bytes());
    offer[4..8].copy_from_slice(&3u32.to_le_bytes());
    offer[8..12].copy_from_slice(&1u32.to_le_bytes());
    offer[16..31].copy_from_slice(b"board.placeMark");
    offer[64..75].copy_from_slice(b"upper right");
    offer[160..168].copy_from_slice(&2i64.to_le_bytes());
    application_host::syscall(0x1151,[2;16],&offer,&mut output,1).unwrap();
    application_host::syscall(0x1152,[2;16],&packet[..8],&mut output,1).unwrap();
    assert!(matches!(application_host::observe([1;16]),Err(Error::Denied)));
    application_host::with_registry(|r|r.grant(context.instance,[1;16],[2;16],true)).unwrap();
    let request=application_host::with_registry(|r|r.propose(context.instance,[1;16],[2;16],1,3,false,2,100)).unwrap();
    assert_eq!(application_host::syscall(0x1153,[2;16],&packet[..8],&mut output,3),Ok(72));
    assert_eq!(u64::from_le_bytes(output[8..16].try_into().unwrap()),request.token);
    assert_eq!(i64::from_le_bytes(output[40..48].try_into().unwrap()),2);
    assert_eq!(application_host::syscall(0x1153,[2;16],&packet[..8],&mut output,4),Ok(0));
    application_host::detach();
    assert!(application_host::context().is_none());
}
