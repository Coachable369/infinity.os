//! Behavioral encryption and tamper tests, not camera/liveness acceptance.
#[path="../kernel/runtime/crypto.rs"] mod crypto;
#[path="../kernel/runtime/face_setup.rs"] mod face_setup;
#[path="../kernel/runtime/face_vault.rs"] mod face_vault;
use face_setup::wire;

// ------------------------=
// FUNC: main
// DESC: Keeps this bounded harness selectable independently through cargo test --bin without building unrelated harnesses.
// ------------------=
fn main() {}

// ------------------------=
// FUNC: encrypted_owner_bound_roundtrip
// DESC: Verifies authenticated portrait/template recovery and rejection of tampering, wrong keys, wrong accounts and wrong models.
// ------------------=
#[test]
fn encrypted_owner_bound_roundtrip() {
    let mut control=wire::Control::empty();control.model=[3;32];control.name_len=4;control.name[..4].copy_from_slice(b"Test");
    let mut payload=[130;wire::ENROLLMENT_BYTES];payload[..1536].fill(0);
    for view in 0..3 {payload[view*512..view*512+4].copy_from_slice(&1f32.to_le_bytes());}
    let record=face_vault::seal(&[5;32],[1;16],&control,[7;12],99,&payload).unwrap();
    assert_ne!(&record[face_vault::HEADER..face_vault::BYTES-16],&payload);
    let plain=face_vault::open(&[5;32],[1;16],[3;32],&*record).unwrap();
    assert_eq!(&plain[face_vault::HEADER..face_vault::BYTES-16],&payload);
    assert!(face_vault::open(&[6;32],[1;16],[3;32],&*record).is_none());
    assert!(face_vault::open(&[5;32],[2;16],[3;32],&*record).is_none());
    assert!(face_vault::open(&[5;32],[1;16],[4;32],&*record).is_none());
    for at in [56,128,140,face_vault::HEADER,face_vault::BYTES-1] {
        let mut corrupt=record.clone();corrupt[at]^=1;
        assert!(face_vault::open(&[5;32],[1;16],[3;32],&*corrupt).is_none());
    }
    payload[..1536].fill(0);
    assert!(face_vault::seal(&[5;32],[1;16],&control,[7;12],99,&payload).is_none());
}
