mod identity {#[derive(Clone,Copy,PartialEq,Eq)] pub struct StableId(pub [u8;16]);}
#[path="../kernel/runtime/face_login.rs"] mod face_login;
// ------------------------=
// FUNC: proof_binding_expiry_and_greeting
// DESC: Checks one-use proof boundaries and the restricted greeting parser without granting real authentication authority.
// ------------------=
#[test]
fn proof_binding_expiry_and_greeting() {
    use identity::StableId;
    assert!(face_login::Proof::new(StableId([1;16]),10).consume(StableId([1;16]),11));
    assert!(!face_login::Proof::new(StableId([1;16]),10).consume(StableId([2;16]),11));
    assert!(!face_login::Proof::new(StableId([1;16]),10).consume(StableId([1;16]),5010));
    assert!(!face_login::Proof::new(StableId([1;16]),10).consume(StableId([1;16]),9));
    for phrase in [b"Hello Infinity".as_slice(),b"Infinity hello.",b" HELLO  INFINITY! "] {assert!(face_login::greeting(phrase));}
    for phrase in [b"Infinity open files".as_slice(),b"Hello Infinity delete files",b"hello",b"Infinity hello Infinity"] {assert!(!face_login::greeting(phrase));}
}
