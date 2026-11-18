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
// DESC: Rejects stale face proof, account switching, pre-face audio, future audio and expired voice authorization.
// ------------------=
#[test]
fn sequential_account_bound_authorization(){
 use identity::StableId;use voice_identity_login::Pending;
 let a=StableId([1;16]);let b=StableId([2;16]);
 assert!(Pending::begin(a,face_login::Proof::new(a,100),5100).is_none());
 assert!(Pending::begin(b,face_login::Proof::new(a,100),101).is_none());
 for (account,capture,complete) in [(b,102,103),(a,100,103),(a,101,103),(a,104,103),(a,102,90101)] {
  let p=Pending::begin(a,face_login::Proof::new(a,100),101).unwrap();
  assert!(p.complete(account,capture,complete).is_none());
 }
 let p=Pending::begin(a,face_login::Proof::new(a,100),101).unwrap();
 assert!(p.complete(a,102,103).unwrap().consume(a,104));
 let p=Pending::begin(a,face_login::Proof::new(a,100),101).unwrap();
 assert!(!p.complete(a,102,103).unwrap().consume(b,104));
 let p=Pending::begin(a,face_login::Proof::new(a,100),101).unwrap();
 assert!(!p.complete(a,102,103).unwrap().consume(a,5103));
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
