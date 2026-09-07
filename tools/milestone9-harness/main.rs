#[path = "../../kernel/runtime/crypto.rs"]
mod crypto;
#[path = "../../kernel/runtime/node/mod.rs"]
mod node;

use node::types::{MeshRole, NodeError, TrustState};
use node::NodeRuntime;

// ------------------------=
// FUNC: paired_nodes
// DESC: Creates two independent nodes and completes explicit code-confirmed trust for the behavioral harness.
// ------------------=
fn paired_nodes() -> (NodeRuntime, NodeRuntime, node::types::NodeId, node::types::NodeId) {
    let mut a = NodeRuntime::new();
    let mut b = NodeRuntime::new();
    let a_id = a.initialize(&[0x11; 32], true).unwrap();
    let b_id = b.initialize(&[0x22; 32], true).unwrap();
    assert_ne!(a_id, b_id);
    a.discover(b.advertise(0x3, 1, 10).unwrap(), 10).unwrap();
    b.discover(a.advertise(0x5, 1, 10).unwrap(), 10).unwrap();
    assert_eq!(a.discovered_nodes()[0].unwrap().trust, TrustState::Untrusted);
    let pairing = a.begin_pairing(b_id, 11).unwrap();
    assert_eq!(a.confirm_pairing(pairing.id, pairing.verification_code + 1, 12, 7), Err(NodeError::VerificationMismatch));
    a.confirm_pairing(pairing.id, pairing.verification_code, 12, 7).unwrap();
    let pairing = b.begin_pairing(a_id, 11).unwrap();
    b.confirm_pairing(pairing.id, pairing.verification_code, 12, 7).unwrap();
    (a, b, a_id, b_id)
}

// ------------------------=
// FUNC: main
// DESC: Exercises node identity, discovery, trust, secure sessions, remote authority, membership, persistence, and bounds through behavior.
// ------------------=
fn main() {
    assert_eq!(NodeRuntime::new().initialize(&[0; 32], false), Err(NodeError::EntropyUnavailable));
    let (mut a, mut b, a_id, b_id) = paired_nodes();

    let secret_a = [0x31; 32];
    let secret_b = [0x42; 32];
    let (_, public_a) = crypto::NodeCrypto::agreement_keypair(&secret_a);
    let (_, public_b) = crypto::NodeCrypto::agreement_keypair(&secret_b);
    let session_a = a.open_session(b_id, &secret_a, &public_b, b"iop-v1/a-b", 20, 8).unwrap();
    let session_b = b.open_session(a_id, &secret_b, &public_a, b"iop-v1/a-b", 20, 8).unwrap();
    assert_eq!(session_a, session_b);
    let aad = b"typed-operation";
    let mut outgoing = *b"hello remote node";
    let (sequence, tag) = a.protect(session_a, aad, &mut outgoing, 21).unwrap();
    let ciphertext = outgoing;
    b.unprotect(session_b, sequence, aad, &mut outgoing, &tag, 21).unwrap();
    assert_eq!(&outgoing, b"hello remote node");
    let mut replay = ciphertext;
    assert_eq!(b.unprotect(session_b, sequence, aad, &mut replay, &tag, 21), Err(NodeError::ReplayDetected));

    let grant = a.grant_remote(b_id, 0x3002, 0x77, 1, 50, 22, 9).unwrap();
    assert!(a.authorize_remote(grant, b_id, 0x3002, 0x77, 1, 23).is_ok());
    assert_eq!(a.authorize_remote(grant, b_id, 0x3002, 0x78, 1, 23), Err(NodeError::CapabilityDenied));
    a.revoke_remote(grant, 24, 9).unwrap();
    assert_eq!(a.authorize_remote(grant, b_id, 0x3002, 0x77, 1, 24), Err(NodeError::CapabilityRevoked));

    a.join_mesh(b_id, MeshRole::Member, 25, 10).unwrap();
    assert_eq!(a.mesh_members().iter().flatten().count(), 1);
    a.leave_mesh(b_id, 26, 10).unwrap();
    assert_eq!(a.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);

    let encoded = a.encode_state().unwrap();
    let mut restored = NodeRuntime::new();
    assert_eq!(restored.restore_state(&encoded).unwrap(), a_id);
    let mut corrupt = encoded;
    corrupt[140] ^= 0x80;
    assert_eq!(NodeRuntime::new().restore_state(&corrupt), Err(NodeError::StateCorrupt));

    restored.revoke_trust(b_id, 30, 11).unwrap();
    assert_eq!(restored.discovered_nodes()[0].unwrap().trust, TrustState::Revoked);
    assert_eq!(restored.open_session(b_id, &secret_a, &public_b, b"iop-v1/a-b", 31, 12), Err(NodeError::NotTrusted));

    let mut bounded = NodeRuntime::new();
    bounded.initialize(&[0x55; 32], true).unwrap();
    for index in 0..32u64 {
        let mut peer = NodeRuntime::new();
        let mut entropy = [0u8; 32];
        entropy[..8].copy_from_slice(&(index + 100).to_le_bytes());
        peer.initialize(&entropy, true).unwrap();
        let result = bounded.discover(peer.advertise(0, 1, 1).unwrap(), 1);
        if index < node::types::MAX_DISCOVERED_NODES as u64 { assert!(result.is_ok()); }
        else { assert_eq!(result, Err(NodeError::ResourceLimit)); }
    }
}
