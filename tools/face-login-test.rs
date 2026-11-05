mod identity {#[derive(Clone,Copy,PartialEq,Eq)] pub struct StableId(pub [u8;16]);}
#[path="../kernel/runtime/face_login.rs"] mod face_login;
// ------------------------=
// FUNC: sign_out_selects_identity_not_previous_page
// DESC: Checks single and multiple-account transitions, removal fallback and an empty picker without carrying over a navigation index.
// ------------------=
#[test]
fn sign_out_selects_identity_not_previous_page() {
    use identity::StableId;
    let users=[StableId([1;16]),StableId([2;16])];
    for old_page in [0,1,7,8,9,20] {
        let mut selected=old_page;
        assert_eq!(selected,old_page);
        selected=face_login::account_selection(users[1],users.len(),|i|users.get(i).copied());
        assert_eq!(selected,1);
    }
    assert_eq!(face_login::account_selection(users[0],1,|i|users.get(i).copied()),0);
    assert_eq!(face_login::account_selection(StableId([3;16]),2,|i|users.get(i).copied()),0);
    assert_eq!(face_login::account_selection(users[0],0,|_|None),0);
}
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
    for phrase in [b"Hello Infinity".as_slice(),b"Infinity hello.",b" HELLO  INFINITY! ",b"Hello, Infinity.",b"Infinity, hello!"] {assert!(face_login::greeting(phrase));}
    for phrase in [b"Infinity open files".as_slice(),b"Hello Infinity delete files",b"hello",b"Infinity hello Infinity"] {assert!(!face_login::greeting(phrase));}
}
