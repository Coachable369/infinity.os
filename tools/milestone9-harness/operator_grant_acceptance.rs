use crate::iop::{OperationId, NODE_OPERATION_HUMAN_APPROVED};
use crate::node::{NodeRuntime, control::CommitError, reconciliation::request, types::NodeError};

// ------------------------=
// FUNC: durable_operator_grant
// DESC: Exercises exact-operation operator grants, durable failure rollback, explicit consent, revocation, and lease boundaries through the production control transaction.
// ------------------=
#[test]
fn durable_operator_grant() {
    let mut local = NodeRuntime::new();
    let mut peer = NodeRuntime::new();
    local.initialize(&[0x71; 32], true).unwrap();
    let id = peer.initialize(&[0x72; 32], true).unwrap();
    local.discover(peer.advertise(1, 1, 1).unwrap(), 1).unwrap();
    let pending = local.begin_pairing(id, 2).unwrap();
    local.confirm_pairing(pending.id, pending.verification_code, true, 3, 3).unwrap();
    let mut grant = request(id.0, OperationId::NodeCapabilityGrant);
    grant.value = OperationId::NodeInspect.machine_id();
    grant.rights = 1;
    grant.scope = 7;
    grant.lease_deadline = 60;
    assert_eq!(local.commit_control(OperationId::NodeCapabilityGrant, grant, 4, 4, 4, |_| true), Err(CommitError::InvalidState));
    grant.flags = NODE_OPERATION_HUMAN_APPROVED;
    let before = local.encode_state().unwrap();
    assert_eq!(local.commit_control(OperationId::NodeCapabilityGrant, grant, 4, 4, 4, |_| false), Err(CommitError::PersistenceFailed));
    assert_eq!(local.encode_state().unwrap(), before);
    assert!(local.remote_grants().iter().all(Option::is_none));
    let (result, notice) = local.commit_control(OperationId::NodeCapabilityGrant, grant, 4, 4, 4, |_| true).unwrap();
    assert_eq!(notice.subject, id);
    assert_eq!(local.authorize_remote(result.handle, id, grant.value, 7, 1, 5), Ok(()));
    assert_eq!(local.authorize_remote(result.handle, id, grant.value, 8, 1, 5), Err(NodeError::CapabilityDenied));
    assert_eq!(local.authorize_remote(result.handle, id, OperationId::NodePolicyUpdate.machine_id(), 7, 1, 5), Err(NodeError::CapabilityDenied));
    assert_eq!(local.authorize_remote(result.handle, id, grant.value, 7, 1, 60), Err(NodeError::CapabilityExpired));
    let mut revoke = request([0; 32], OperationId::NodeCapabilityRevoke);
    revoke.handle = result.handle;
    assert_eq!(local.commit_control(OperationId::NodeCapabilityRevoke, revoke, 6, 6, 6, |_| false), Err(CommitError::PersistenceFailed));
    assert_eq!(local.authorize_remote(result.handle, id, grant.value, 7, 1, 7), Ok(()));
    let (_, notice) = local.commit_control(OperationId::NodeCapabilityRevoke, revoke, 8, 8, 8, |_| true).unwrap();
    assert_eq!(notice.subject, id);
    assert_eq!(local.authorize_remote(result.handle, id, grant.value, 7, 1, 9), Err(NodeError::CapabilityRevoked));
    grant.value = OperationId::NodePairConfirm.machine_id();
    assert_eq!(local.commit_control(OperationId::NodeCapabilityGrant, grant, 9, 9, 9, |_| true), Err(CommitError::InvalidState));
}
