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
    generic_creation_opponent();
    host_packets();
    typed_actions();
    approvals();
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
// FUNC: generic_creation_opponent
// DESC: Routes unknown game names by an explicit creation-and-opponent suffix while preserving bounded direct-instruction authority.
// ------------------=
fn generic_creation_opponent() {
    use editor_compile_intent::{application_kind,application_style,create_application,requests_opponent,ApplicationKind,ApplicationStyle};
    for input in [b"create Othello and play me".as_slice(),b"create Reversi and play against me",
        b"Please BUILD a new hex-grid strategy and play against me.",
        b"generate NebulaTiles and play me",b"write a terminal OrbitRace and play me"] {
        assert_eq!(application_kind(input),Some(ApplicationKind::Game));
        assert!(create_application(input));assert!(requests_opponent(input));
    }
    assert_eq!(application_style(b"create Othello and play me"),Some(ApplicationStyle::Ui));
    assert_eq!(application_style(b"create terminal Othello and play me"),Some(ApplicationStyle::Terminal));
    for input in [b"create and play me".as_slice(),b"create and play against me",
        b"create Othello",b"create Othello and play someone",b"create Reversi and play me later",
        b"explain how to create Othello and play me",b"do not create Othello and play me",
        b"create Othello but do not play me",b"create Reversi without play against me",
        b"create Othello and never play me",b"create Othello and don't play me",
        b"\"create Othello and play me\"",b"create \"Othello and play me\"",
        b"create Othello; and play me"] {
        assert_eq!(application_kind(input),None);assert!(!requests_opponent(input));
    }
    let mut oversized=vec![b'x';257];oversized.splice(..7,b"create ".iter().copied());
    oversized.extend_from_slice(b" and play me");
    assert_eq!(application_kind(&oversized),None);assert!(!requests_opponent(&oversized));
    assert!(!requests_opponent(b"create a Reversi game"));
    assert!(requests_opponent(b"play against me"));
    assert_eq!(application_kind(b"create a calendar app"),Some(ApplicationKind::General));
}

// ------------------------=
// FUNC: approvals
// DESC: Verifies review quarantine, exact-token approval, expiry, changed state and revocation without dispatching unintended actions.
// ------------------=
fn approvals() {
    let mut registry=Box::new(Registry::new());
    let id=registry.register([1;16],[2;16]).unwrap();
    publish(&mut registry,id,1,Risk::Sensitive);registry.grant(id,[1;16],[2;16],true).unwrap();
    let state=registry.observe(id,[1;16],[2;16]).unwrap();
    assert_eq!(application_decision::parse_for_state(b"CHOICE:3:board.placeMark",&state).unwrap().id,3);
    assert!(application_decision::parse_for_state(b"CHOICE:3:document.clear",&state).is_none());
    assert!(application_decision::parse_for_state(b"CHOICE:3:board.placeMark\nCHOICE:4",&state).is_none());
    assert!(application_decision::parse_for_state(b"CHOICE:4:board.placeMark",&state).is_none());
    let review=registry.prepare_review(id,[1;16],[2;16],1,3,None,&[],10).unwrap();
    assert_eq!(registry.poll(id,[1;16],[2;16],11),Ok(None));
    assert_eq!(registry.approve_review(id,[3;16],[2;16],review.token,12),Err(Error::Denied));
    assert_eq!(registry.approve_review(id,[1;16],[3;16],review.token,12),Err(Error::Denied));
    assert_eq!(registry.approve_review(id,[1;16],[2;16],review.token+1,12),Err(Error::Denied));
    let approved=registry.approve_review(id,[1;16],[2;16],review.token,12).unwrap();
    assert_eq!(approved.token,review.token);
    assert_eq!(registry.approve_review(id,[1;16],[2;16],review.token,13),Err(Error::Missing));
    assert_eq!(registry.poll(id,[1;16],[2;16],13),Ok(Some(approved)));
    registry.finish(id,[1;16],[2;16],approved.token,Outcome::Rejected,14).unwrap();
    let expired=registry.prepare_review(id,[1;16],[2;16],1,3,None,&[],20).unwrap();
    assert_eq!(registry.approve_review(id,[1;16],[2;16],expired.token,expired.deadline),Err(Error::Expired));
    assert_eq!(registry.poll(id,[1;16],[2;16],expired.deadline),Ok(None));
    let stale=registry.prepare_review(id,[1;16],[2;16],1,3,None,&[],30).unwrap();
    publish(&mut registry,id,2,Risk::Sensitive);
    assert_eq!(registry.approve_review(id,[1;16],[2;16],stale.token,31),Err(Error::Missing));
    let revoked=registry.prepare_review(id,[1;16],[2;16],2,3,None,&[],40).unwrap();
    registry.grant(id,[1;16],[2;16],false).unwrap();
    assert!(registry.approve_review(id,[1;16],[2;16],revoked.token,41).is_err());
    registry.grant(id,[1;16],[2;16],true).unwrap();
    assert_eq!(registry.poll(id,[1;16],[2;16],42),Ok(None));
}

// ------------------------=
// FUNC: typed_actions
// DESC: Exercises typed document edits, bounds, pinned references and atomic ABI delivery with observable content changes.
// ------------------=
fn typed_actions() {
    use application_decision::parse_proposal;
    let context=application_host::attach([1;16],[2;16]).unwrap();
    let mut packet=[0u8;2072];packet[..4].copy_from_slice(&1u32.to_le_bytes());
    packet[4..8].copy_from_slice(&1u32.to_le_bytes());packet[8..16].copy_from_slice(&1u64.to_le_bytes());
    packet[16..20].copy_from_slice(&5u32.to_le_bytes());packet[24..29].copy_from_slice(b"hello");
    let mut output=[0u8;592];
    application_host::syscall(0x1150,[2;16],&packet,&mut output,1).unwrap();
    let mut action=[0u8;416];action[..4].copy_from_slice(&1u32.to_le_bytes());
    action[4..8].copy_from_slice(&9u32.to_le_bytes());action[8..12].copy_from_slice(&1u32.to_le_bytes());
    action[16..31].copy_from_slice(b"document.insert");action[64..75].copy_from_slice(b"insert text");
    action[160..168].copy_from_slice(&42i64.to_le_bytes());
    for (index,(name,kind,minimum,maximum)) in [
        (b"document".as_slice(),3u32,42i64,42i64), (b"position",1,0,5), (b"select",2,0,1), (b"reserved",0,0,0)
    ].iter().enumerate() {
        let at=192+index*48;action[at..at+name.len()].copy_from_slice(name);
        action[at+24..at+28].copy_from_slice(&kind.to_le_bytes());
        action[at+32..at+40].copy_from_slice(&minimum.to_le_bytes());action[at+40..at+48].copy_from_slice(&maximum.to_le_bytes());
    }
    action[384..391].copy_from_slice(b"content");action[408..412].copy_from_slice(&32u32.to_le_bytes());
    application_host::syscall(0x1155,[2;16],&action,&mut output,1).unwrap();
    application_host::syscall(0x1152,[2;16],&packet[..8],&mut output,1).unwrap();
    let diagnostic=unsafe{application_host::INFINITY_APP_INTERACTION_SNAPSHOT};
    assert_eq!(&diagnostic[8..11],&[1,1,1]);
    let exported=unsafe{application_host::INFINITY_APP_INTERACTION_CHOICES};
    assert_eq!(u32::from_le_bytes(exported[12..16].try_into().unwrap()),1);
    assert_eq!(&exported[192..416],&action[192..416]);
    application_host::with_registry(|r|r.grant(context.instance,[1;16],[2;16],true)).unwrap();
    for (arguments,text) in [
        ([43,5,0,0],b"world".as_slice()), ([42,6,0,0],b"world"), ([42,5,2,0],b"world"),
        ([42,5,0,1],b"world"), ([42,5,0,0],&[0xff]), ([42,5,0,0],b"bad\0text"),
        ([42,5,0,0],&[b'a';33]),
    ] {
        assert_eq!(application_host::with_registry(|r|r.propose_payload(context.instance,[1;16],[2;16],1,9,Some(arguments),text,false,2,100)),Err(Error::Invalid));
    }
    assert_eq!(application_host::with_registry(|r|r.propose(context.instance,[1;16],[2;16],1,9,false,2,100)),Err(Error::Invalid));
    let proposal=parse_proposal("ACTION:9 42 5 1 0\nTEXT:7: 世界".as_bytes()).unwrap();
    let request=application_host::with_registry(|r|r.propose_payload(context.instance,[1;16],[2;16],1,proposal.id,proposal.arguments,proposal.text,false,2,100)).unwrap();
    assert_eq!(application_host::syscall(0x1153,[2;16],&packet[..8],&mut output,3),Err(Error::Invalid));
    assert_eq!(application_host::syscall(0x1156,[2;16],&packet[..8],&mut output[..591],3),Err(Error::Invalid));
    assert_eq!(application_host::syscall(0x1156,[2;16],&packet[..8],&mut output,3),Ok(592));
    assert_eq!(u64::from_le_bytes(output[8..16].try_into().unwrap()),request.token);
    let position=i64::from_le_bytes(output[48..56].try_into().unwrap()) as usize;
    let length=u32::from_le_bytes(output[72..76].try_into().unwrap()) as usize;
    let mut document=String::from("hello");
    document.insert_str(position,core::str::from_utf8(&output[80..80+length]).unwrap());
    assert_eq!(document,"hello 世界");
    assert_eq!(application_host::syscall(0x1156,[2;16],&packet[..8],&mut output,4),Ok(0));
    application_host::with_registry(|r|{
        r.begin(context.instance,[1;16],[2;16],2,document.as_bytes(),false,false).unwrap();
        r.commit(context.instance,[1;16],[2;16]).unwrap();
        assert_eq!(r.finish(context.instance,[1;16],[2;16],request.token,Outcome::Applied,5).unwrap().revision,2);
    });
    for input in [b"ACTION:9 42 5 0\nTEXT:1:x".as_slice(),b"ACTION:9 42 5 0 0 0\nTEXT:1:x",b"ACTION:0 42 5 0 0\nTEXT:1:x",b"ACTION:9 42 9223372036854775808 0 0\nTEXT:1:x",b"ACTION:9 42 5 0 0\nother:x",b"ACTION:9 42 5 0 0\nTEXT:2:x",b"ACTION:9 42 5 0 0\nTEXT:1:xTrailing",b"ACTION:9 42 5 0 0\nTEXT:0:x"] {
        assert!(parse_proposal(input).is_none());
    }
    // Command-looking content is literal data, not a second execution proposal.
    let literal=parse_proposal(b"ACTION:9 42 0 0 0\nTEXT:17:CHOICE:3\nACTION:4").unwrap();
    assert_eq!(literal.text,b"CHOICE:3\nACTION:4");
    application_host::detach();
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
    let diagnostic=unsafe{application_host::INFINITY_APP_INTERACTION_SNAPSHOT};
    assert_eq!(diagnostic[2],diagnostic[17]);assert_eq!(diagnostic[2]%2,0);
    assert_eq!(&diagnostic[3..11],&[context.instance,1,1,0,5,1,0,1]);
    let exported=unsafe{application_host::INFINITY_APP_INTERACTION_CHOICES};
    assert_eq!(&exported[..192],&offer);
    assert_eq!(unsafe{application_host::INFINITY_APP_INTERACTION_STATE}[..5],packet[24..29]);
    application_host::with_registry(|r|r.grant(context.instance,[1;16],[2;16],true)).unwrap();
    let request=application_host::with_registry(|r|r.propose(context.instance,[1;16],[2;16],1,3,false,2,100)).unwrap();
    assert_eq!(application_host::syscall(0x1153,[2;16],&packet[..8],&mut output,3),Ok(72));
    assert_eq!(u64::from_le_bytes(output[8..16].try_into().unwrap()),request.token);
    assert_eq!(i64::from_le_bytes(output[40..48].try_into().unwrap()),2);
    assert_eq!(application_host::syscall(0x1153,[2;16],&packet[..8],&mut output,4),Ok(0));
    let diagnostic=unsafe{application_host::INFINITY_APP_INTERACTION_SNAPSHOT};
    assert_eq!(diagnostic[11],1);assert_eq!(diagnostic[14],3);assert_eq!(diagnostic[15],request.token);
    packet[8..16].copy_from_slice(&2u64.to_le_bytes());
    application_host::syscall(0x1150,[2;16],&packet,&mut output,5).unwrap();
    application_host::syscall(0x1152,[2;16],&packet[..8],&mut output,5).unwrap();
    let mut receipt=[0;16];receipt[..4].copy_from_slice(&1u32.to_le_bytes());receipt[8..16].copy_from_slice(&request.token.to_le_bytes());
    application_host::syscall(0x1154,[2;16],&receipt,&mut output,6).unwrap();
    let diagnostic=unsafe{application_host::INFINITY_APP_INTERACTION_SNAPSHOT};
    assert_eq!(diagnostic[4],2);assert_eq!(&diagnostic[10..14],&[2,1,1,0]);assert_eq!(diagnostic[16],1);
    application_host::detach();
    assert!(application_host::context().is_none());
    assert_eq!(unsafe{application_host::INFINITY_APP_INTERACTION_SNAPSHOT}[3],0);
    assert!(unsafe{application_host::INFINITY_APP_INTERACTION_STATE}.iter().all(|value|*value==0));
    assert!(unsafe{application_host::INFINITY_APP_INTERACTION_CHOICES}.iter().all(|value|*value==0));
}
