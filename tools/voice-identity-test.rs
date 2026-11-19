//! Behavioral checks for the combined login proof and encrypted speaker vault.
mod identity {#[derive(Clone,Copy,PartialEq,Eq)] pub struct StableId(pub [u8;16]);}
#[path="../kernel/runtime/face_login.rs"] mod face_login;
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
  let mut p=Pending::begin(a,100);assert!(p.accept_voice(a,101,103));
  assert!(p.complete(account,face_login::Proof::new(account,face_time),complete).is_none());
 }
 let mut p=Pending::begin(a,100);assert!(p.accept_voice(a,101,103));
 assert!(!p.accept_voice(a,104,105));
 assert!(p.complete(a,face_login::Proof::new(a,106),107).unwrap().consume(a,108));
 let mut p=Pending::begin(a,100);assert!(p.accept_voice(a,101,103));
 assert!(!p.complete(a,face_login::Proof::new(a,104),105).unwrap().consume(b,106));
 let mut p=Pending::begin(a,100);assert!(p.accept_voice(a,101,103));
 assert!(!p.complete(a,face_login::Proof::new(a,104),105).unwrap().consume(a,5105));
 // A face result for a different account cannot complete the selected account.
 let mut p=Pending::begin(a,100);assert!(p.accept_voice(a,101,103));
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
