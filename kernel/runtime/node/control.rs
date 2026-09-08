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
            members: self.members,
            audit: self.audit,
            next_id: self.next_id,
            audit_sequence: self.audit_sequence,
            control_version: version,
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
