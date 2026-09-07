use crate::node::{NodeRuntime, types::{NodeError, TrustState, MAX_PAIRINGS}};

// ------------------------=
// FUNC: run
// DESC: Exercises terminal pairing states, bounded reuse, and trust changes against delayed approvals.
// ------------------=
pub fn run() {
    let mut local = NodeRuntime::new();
    let mut peer = NodeRuntime::new();
    local.initialize(&[0x91; 32], true).unwrap();
    let id = peer.initialize(&[0x92; 32], true).unwrap();
    local.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
    assert_eq!(local.set_trust(id, TrustState::Trusted, 1, 1), Err(NodeError::HumanApprovalRequired));
    for now in 2..(MAX_PAIRINGS as u64 * 3 + 2) {
        let pending = local.begin_pairing(id, now).unwrap();
        assert_eq!(local.begin_pairing(id, now), Err(NodeError::ResourceLimit));
        local.cancel_pairing(pending.id).unwrap();
        assert_eq!(local.confirm_pairing(pending.id, pending.verification_code, true, now, now), Err(NodeError::PairingNotFound));
    }
    let pending = local.begin_pairing(id, 20).unwrap();
    local.revoke_trust(id, 21, 21).unwrap();
    assert_eq!(local.confirm_pairing(pending.id, pending.verification_code, true, 22, 22), Err(NodeError::PairingNotFound));
    assert_eq!(local.cancel_pairing(pending.id), Err(NodeError::PairingNotFound));
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Revoked);
    local.set_trust(id, TrustState::Untrusted, 23, 23).unwrap();
    let pending = local.begin_pairing(id, 24).unwrap();
    local.set_trust(id, TrustState::Blocked, 25, 25).unwrap();
    assert_eq!(local.confirm_pairing(pending.id, pending.verification_code, true, 26, 26), Err(NodeError::PairingNotFound));
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Blocked);
    local.set_trust(id, TrustState::Untrusted, 27, 27).unwrap();
    let pending = local.begin_pairing(id, 28).unwrap();
    local.confirm_pairing(pending.id, pending.verification_code, true, 29, 29).unwrap();
    assert_eq!(local.cancel_pairing(pending.id), Err(NodeError::PairingNotFound));
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Trusted);
    let grant = local.grant_remote(id, 0x3002, 7, 1, 100, 30, 30).unwrap();
    local.set_trust(id, TrustState::Restricted, 31, 31).unwrap();
    assert_eq!(local.authorize_remote(grant, id, 0x3002, 7, 1, 32), Err(NodeError::CapabilityRevoked));
    local.set_trust(id, TrustState::Trusted, 33, 33).unwrap();
    assert_eq!(local.authorize_remote(grant, id, 0x3002, 7, 1, 34), Err(NodeError::CapabilityRevoked));
    local.set_trust(id, TrustState::Untrusted, 35, 35).unwrap();
    let pending = local.begin_pairing(id, 36).unwrap();
    local.sweep(pending.expires_at);
    assert_eq!(local.discovered_nodes()[0].unwrap().trust, TrustState::Untrusted);
    assert_eq!(local.confirm_pairing(pending.id, pending.verification_code, true, pending.expires_at, 36), Err(NodeError::PairingExpired));
}
