//! Behavioral checks for the combined login proof and encrypted speaker vault.
mod identity {#[derive(Clone,Copy,PartialEq,Eq)] pub struct StableId(pub [u8;16]);}
#[path="../kernel/runtime/face_login.rs"] mod face_login;
#[path="../kernel/runtime/biometric_autostart.rs"] mod biometric_autostart;
#[path="../kernel/runtime/voice_identity_login.rs"] mod voice_identity_login;
#[path="../kernel/runtime/crypto.rs"] mod crypto;
#[path="../kernel/runtime/voice_identity_vault.rs"] mod voice_identity_vault;
// ------------------------=
// FUNC: main
// DESC: Selects this bounded authentication harness through the build kit.
// ------------------=
fn main() {}
// ------------------------=
// FUNC: sequential_account_bound_authorization
// DESC: Requires fresh voice before fresh face; rejects account substitution, old/future audio, repeated factors and expired attempts.
// ------------------=
#[test]
fn sequential_account_bound_authorization(){
 use identity::StableId;use voice_identity_login::Pending;
 let a=StableId([1;16]);let b=StableId([2;16]);
 // Face alone cannot complete a newly started attempt.
 assert!(Pending::begin(a,100).complete(a,face_login::Proof::new(a,102),103).is_none());
 for (account,capture,verified) in [(b,101,102),(a,99,102),(a,100,102),(a,103,102),(a,101,90100)] {
  let mut p=Pending::begin(a,100);assert!(!p.accept_voice(account,capture,verified));
  assert!(p.complete(a,face_login::Proof::new(a,104),105).is_none());
 }
 for (account,face_time,complete) in [(b,104,105),(a,101,105),(a,103,105),(a,106,105),(a,104,5104),(a,90100,90101)] {
  let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Hello Infinity"));assert!(p.accept_voice(a,101,103));
  assert!(p.complete(account,face_login::Proof::new(account,face_time),complete).is_none());
 }
 let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Hello Infinity"));assert!(p.accept_voice(a,101,103));
 assert!(!p.accept_voice(a,104,105));
 assert!(p.complete(a,face_login::Proof::new(a,106),107).unwrap().consume(a,108));
 let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Hello Infinity"));assert!(p.accept_voice(a,101,103));
 assert!(!p.complete(a,face_login::Proof::new(a,104),105).unwrap().consume(b,106));
 let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Hello Infinity"));assert!(p.accept_voice(a,101,103));
 assert!(!p.complete(a,face_login::Proof::new(a,104),105).unwrap().consume(a,5105));
 // A face result for a different account cannot complete the selected account.
 let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Hello Infinity"));assert!(p.accept_voice(a,101,103));
 assert!(p.complete(a,face_login::Proof::new(b,104),105).is_none());
}
// ------------------------=
// FUNC: encrypted_voice_owner_model_and_integrity
// DESC: Verifies encrypted roundtrip and authenticated rejection of changed keys, owners, models, header and ciphertext.
// ------------------=
#[test]
fn encrypted_voice_owner_model_and_integrity(){
 use identity::StableId;use voice_identity_vault as v;
 let a=StableId([1;16]);let key=[2;32];let model=[3;32];let payload=[17;v::PAYLOAD];
 let record=v::seal(&key,a,&model,[4;12],&payload).unwrap();
 assert_ne!(&record[v::HEADER..v::BYTES-16],&payload);
 assert_eq!(*v::open(&key,a,&model,&record).unwrap(),payload);
 assert!(v::open(&[9;32],a,&model,&record).is_none());
 assert!(v::open(&key,StableId([9;16]),&model,&record).is_none());
 assert!(v::open(&key,a,&[9;32],&record).is_none());
 for i in [0,8,24,56,68,72,v::HEADER,v::BYTES-1]{
  let mut changed=record.clone();changed[i]^=1;assert!(v::open(&key,a,&model,&changed).is_none());
 }
 assert!(v::seal(&key,a,&model,[0;12],&payload).is_none());
}

// ------------------------=
// FUNC: greeting_and_speaker_must_share_fresh_capture
// DESC: Rejects phrase-only authorization, unrelated speech and substitution of a different speaker recording.
// ------------------=
#[test]
fn greeting_and_speaker_must_share_fresh_capture(){
 use identity::StableId;use voice_identity_login::Pending;
 let a=StableId([1;16]);let b=StableId([2;16]);
 for phrase in [b"Infinity Hello".as_slice(),b"Hello Infinity",b"hello INFINITY!"]{
  let mut p=Pending::begin(a,100);
  assert!(!p.accept_voice(a,101,102));
  assert!(!p.accept_greeting(b,101,102,phrase));
  assert!(!p.accept_greeting(a,100,102,phrase));
  assert!(p.accept_greeting(a,101,102,phrase));
  assert!(!p.accept_voice(a,102,103));
  assert!(p.accept_voice(a,101,103));
  assert!(p.complete(a,face_login::Proof::new(a,104),105).unwrap().consume(a,106));
 }
 for phrase in [b"hello".as_slice(),b"Infinity",b"Hello Infinity open files",b"random speech",b""]{
  let mut p=Pending::begin(a,100);
  assert!(!p.accept_greeting(a,101,102,phrase));assert!(!p.accept_voice(a,101,103));
 }
 let mut p=Pending::begin(a,100);assert!(p.accept_greeting(a,101,102,b"Infinity Hello"));
 assert!(p.complete(a,face_login::Proof::new(a,104),105).is_none());
}

// ------------------------=
// FUNC: failed_face_attempt_restarts_with_fresh_factors
// DESC: Exercises failed and expired voice-first attempts through the real retry gate, requiring new voice and face evidence after each cooldown.
// ------------------=
#[test]
fn failed_face_attempt_restarts_with_fresh_factors(){
 use identity::StableId;use voice_identity_login::Pending;
 let a=StableId([1;16]);let other=StableId([2;16]);
 let mut gate=biometric_autostart::Gate::new();
 assert!(!gate.poll(Some(a),true,false,0));
 assert!(gate.poll(Some(a),true,false,750));
 let mut start=750;
 for expired in [false,true,false,true]{
  let mut pending=Pending::begin(a,start);
  assert!(pending.accept_greeting(a,start+1,start+2,b"Hello Infinity"));
  assert!(pending.accept_voice(a,start+1,start+3));
  let end=if expired{start+90_000}else{start+5};
  assert!(pending.complete(a,face_login::Proof::new(other,end),end).is_none());
  gate.retry(end);
  for tick in [end+1,end+500,end+1999]{
   gate.retry(tick);
   assert!(!gate.poll(Some(a),true,false,tick));
  }
  start=end+2000;
  assert!(gate.poll(Some(a),true,false,start));
 }
 let mut fresh=Pending::begin(a,start);
 assert!(!fresh.accept_voice(a,start-1,start+1));
 assert!(fresh.accept_greeting(a,start+1,start+2,b"Infinity Hello"));
 assert!(fresh.accept_voice(a,start+1,start+3));
 assert!(fresh.complete(a,face_login::Proof::new(a,start+4),start+5).unwrap().consume(a,start+6));
}
