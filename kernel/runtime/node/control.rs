//! Staged durable control mutations. The router is responsible for admission and
//! execution-time authorization. No transport, UI, or storage implementation lives here.
use super::super::iop::{execute_node_operation, NodeOperationV1, OperationId};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitError {
    InvalidOperation,
    InvalidState,
    VersionExhausted,
    PersistenceFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittedControl {
    /// Monotonic checkpoint for this local node control service, not a global IEF sequence.
    pub version: u64,
    pub subject: NodeId,
    pub operation: OperationId,
    pub value: u32,
    pub correlation: u64,
    pub causation: u64,
}

struct Staged(NodeRuntime);
impl Drop for Staged {
    // ------------------------=
    // FUNC: drop
    // DESC: Erases temporary identity and traffic-key copies on every transaction exit, including persistence failure.
    // ------------------=
    fn drop(&mut self) {
        self.0.identity_seed.zeroize();
        for session in self.0.sessions.iter_mut().flatten() {
            session.tx_key.zeroize();
            session.rx_key.zeroize();
        }
    }
}

impl NodeRuntime {
    // ------------------------=
    // FUNC: commit_wire_control
    // DESC: Stages both node and wire state, persists before exposing queued protocol packets, and never falls back to local-only pairing.
    // ------------------=
    pub fn commit_wire_control(&mut self, wire: &mut wire_trust::WireTrust, link: Option<transport::LinkSnapshot>, operation: OperationId, request: NodeOperationV1, now: u64, correlation: u64, causation: u64, persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(NodeOperationV1, CommittedControl), CommitError> {
        if request.operation != operation.machine_id() || request.schema_version != 1 { return Err(CommitError::InvalidOperation); }
        let mut staged = self.clone();
        let mut staged_wire = wire.clone();
        // Nested confirmation is allowed only inside this isolated candidate;
        // its real storage writer runs once below, before either candidate is installed.
        staged_wire.persist_pairing = staged_confirmation;
        let mut response = request;
        let subject;
        match operation {
            OperationId::NodePairBegin | OperationId::NodeSessionOpen => {
                subject = NodeId(request.node_id);
                let link = link.filter(|link| link.peer == Some(subject)).ok_or(CommitError::InvalidState)?;
                staged_wire.begin(&mut staged, link, request.scope, operation == OperationId::NodeSessionOpen, now).map_err(|_| CommitError::InvalidState)?;
                if operation == OperationId::NodePairBegin {
                    let pairing = staged.pairings.iter().flatten().find(|pairing| pairing.peer == subject && pairing.state == PairingState::AwaitingConfirmation).ok_or(CommitError::InvalidState)?;
                    response.handle = pairing.id;
                    response.value = 0; // Only the completed wire verification view may disclose a verification code.
                } else { response.handle = 0; response.value = 2; } // Pending, not an established session.
            }
            OperationId::NodePairConfirm | OperationId::NodePairCancel => {
                subject = staged.pairings.iter().flatten().find(|pairing| pairing.id == request.handle).ok_or(CommitError::InvalidState)?.peer;
                let transaction = staged_wire.transaction_for_pairing(request.handle).ok_or(CommitError::InvalidState)?;
                if operation == OperationId::NodePairConfirm {
                    staged_wire.confirm(&mut staged, transaction, request.value, request.flags & super::super::iop::NODE_OPERATION_HUMAN_APPROVED != 0, now).map_err(|_| CommitError::InvalidState)?;
                } else { staged_wire.cancel(&mut staged, transaction, now).map_err(|_| CommitError::InvalidState)?; }
                response.node_id = subject.0;
            }
            _ => return Err(CommitError::InvalidOperation),
        }
        let version = self.control_version.checked_add(1).ok_or(CommitError::VersionExhausted)?;
        staged.control_version = version;
        let encoded = zeroize::Zeroizing::new(staged.encode_state().map_err(|_| CommitError::InvalidState)?);
        if !persist(&encoded) { return Err(CommitError::PersistenceFailed); }
        staged_wire.persist_pairing = wire.persist_pairing;
        *self = staged;
        *wire = staged_wire;
        Ok((response, CommittedControl { version, subject, operation, value: request.value, correlation, causation }))
    }
    // ------------------------=
    // FUNC: control_version
    // DESC: Reads the restart-persistent checkpoint for committed control transactions only.
    // ------------------=
    pub fn control_version(&self) -> u64 {
        self.control_version
    }

    // ------------------------=
    // FUNC: commit_control
    // DESC: Stages an authorized control operation, persists its next checkpoint, and only then replaces live authoritative state.
    // ------------------=
    pub fn commit_control(
        &mut self,
        operation: OperationId,
        request: NodeOperationV1,
        now: u64,
        correlation: u64,
        causation: u64,
        persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool,
    ) -> Result<(NodeOperationV1, CommittedControl), CommitError> {
        // Pairing must use the wire verification service; membership must use a
        // synchronized domain transaction. Neither may pass through this subset.
        if !matches!(
            operation,
            OperationId::NodeTrustUpdate
                | OperationId::NodeRevokeTrust
                | OperationId::NodeBlock
                | OperationId::NodeUnblock
                | OperationId::NodePolicyUpdate
                | OperationId::NodeSessionClose
                | OperationId::NodeCapabilityGrant
                | OperationId::NodeCapabilityRevoke
        ) {
            return Err(CommitError::InvalidOperation);
        }
        let subject = if operation == OperationId::NodeSessionClose {
            self.sessions
                .iter()
                .flatten()
                .find(|s| s.id == request.handle)
                .map(|s| s.peer)
                .ok_or(CommitError::InvalidState)?
        } else if operation == OperationId::NodeCapabilityRevoke {
            self.grants.iter().flatten().find(|grant| grant.id == request.handle)
                .map(|grant| grant.peer).or_else(||self.durable_approval(request.handle).map(|a|a.peer)).ok_or(CommitError::InvalidState)?
        } else {
            NodeId(request.node_id)
        };
        let version = self
            .control_version
            .checked_add(1)
            .ok_or(CommitError::VersionExhausted)?;
        if operation == OperationId::NodePolicyUpdate
            && self
                .discovered
                .iter()
                .flatten()
                .any(|n| n.id.0 == request.node_id && n.policy.version == u32::MAX)
        {
            return Err(CommitError::VersionExhausted);
        }
        let mut staged = Staged(NodeRuntime {
            // These operations do not sign or derive keys. The authoritative
            // crypto provider never leaves the live service.
            crypto: NodeCrypto::new(),
            identity_seed: self.identity_seed,
            local_id: self.local_id,
            key_ref: self.key_ref,
            discovered: self.discovered,
            pairings: self.pairings,
            sessions: self.sessions,
            grants: self.grants,
            durable: self.durable,
            members: self.members,
            audit: self.audit,
            next_id: self.next_id,
            audit_sequence: self.audit_sequence,
            control_version: version,
            paired_digests: self.paired_digests,
            domains: self.domains,
            configured_links: self.configured_links,
            discovery_window: self.discovery_window,
            discovery_count: self.discovery_count,
        });
        let response = execute_node_operation(&mut staged.0, operation, request, now, correlation)
            .map_err(|_| CommitError::InvalidState)?;
        let encoded = zeroize::Zeroizing::new(
            staged
                .0
                .encode_state()
                .map_err(|_| CommitError::InvalidState)?,
        );
        if !persist(&encoded) {
            return Err(CommitError::PersistenceFailed);
        }
        // No fallible step remains between the successful durable commit and the
        // in-memory replacement. Publication belongs AFTER this boundary.
        self.discovered = staged.0.discovered;
        self.pairings = staged.0.pairings;
        self.sessions = staged.0.sessions;
        self.grants = staged.0.grants;
        self.durable = staged.0.durable;
        self.members = staged.0.members;
        self.audit = staged.0.audit;
        self.next_id = staged.0.next_id;
        self.audit_sequence = staged.0.audit_sequence;
        self.control_version = version;
        Ok((
            response,
            CommittedControl {
                version,
                subject,
                operation,
                value: request.value,
                correlation,
                causation,
            },
        ))
    }
}

// ------------------------=
// FUNC: staged_confirmation
// DESC: Accepts an inner confirmation only inside an isolated transaction whose outer commit owns durable storage.
// ------------------=
fn staged_confirmation(_: &[u8; NODE_STATE_BYTES]) -> bool { true }
#[cfg(test)]
mod durable_tests {
    use super::*;
    // ------------------------=
    // FUNC: durable_control_commits_before_authority_and_revoke_survives_reboot
    // DESC: Verifies failed durable approval/revoke persistence never changes live authority and a successful journal survives boot without live grants.
    // ------------------=
    #[test]
    fn durable_control_commits_before_authority_and_revoke_survives_reboot(){
        std::thread::Builder::new().stack_size(16*1024*1024).spawn(||{
            let mut n=NodeRuntime::new();n.initialize(&[71;32],true).unwrap();let mut other=NodeRuntime::new();let peer=other.initialize(&[72;32],true).unwrap();n.discover(other.advertise(1,1,1).unwrap(),1).unwrap();let pair=n.begin_pairing(peer,2).unwrap();n.confirm_pairing(pair.id,pair.verification_code,true,3,77).unwrap();
            let mut request=NodeOperationV1{node_id:peer.0,handle:0,scope:0,lease_deadline:0,operation:OperationId::NodeCapabilityGrant.machine_id(),rights:1,value:OperationId::PoolMetadata.machine_id(),flags:3,schema_version:1};
            let old=n.encode_state().unwrap();assert!(n.commit_control(OperationId::NodeCapabilityGrant,request,4,1,1,|_|false).is_err());assert_eq!(n.encode_state().unwrap(),old);
            let mut denied=request;denied.flags=2;assert!(n.commit_control(OperationId::NodeCapabilityGrant,denied,4,1,1,|_|panic!("unauthorized write")).is_err());
            denied=request;denied.lease_deadline=100;assert!(n.commit_control(OperationId::NodeCapabilityGrant,denied,4,1,1,|_|panic!("ambiguous lifetime")).is_err());
            let mut durable=[0;NODE_STATE_BYTES];let(response,_)=n.commit_control(OperationId::NodeCapabilityGrant,request,4,1,1,|b|{durable=*b;true}).unwrap();
            assert!(n.authorize_remote(response.handle,peer,request.value,0,1,4).is_ok());let mut restored=NodeRuntime::new();restored.restore_state(&durable).unwrap();assert!(restored.remote_grants().iter().all(Option::is_none));assert!(restored.authorize_remote(response.handle,peer,request.value,0,1,0).is_ok());
            request.operation=OperationId::NodeCapabilityRevoke.machine_id();request.handle=response.handle;
            assert!(restored.commit_control(OperationId::NodeCapabilityRevoke,request,1,2,2,|_|false).is_err());assert!(restored.authorize_remote(response.handle,peer,OperationId::PoolMetadata.machine_id(),0,1,1).is_ok());
            restored.commit_control(OperationId::NodeCapabilityRevoke,request,1,2,2,|b|{durable=*b;true}).unwrap();let mut reboot=NodeRuntime::new();reboot.restore_state(&durable).unwrap();assert_eq!(reboot.authorize_remote(response.handle,peer,OperationId::PoolMetadata.machine_id(),0,1,0),Err(NodeError::CapabilityRevoked));
        }).unwrap().join().unwrap();
    }
}
