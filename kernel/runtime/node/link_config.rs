//! Explicit operator-approved peer endpoints. Configuration grants discovery carriage, never peer trust or membership.
use super::*;
use super::super::iop::{NodeOperationV1, OperationId};
use control::{CommitError, CommittedControl};
pub const MAX_CONFIGURED_LINKS: usize = 4;
const OFFSET: usize = 9472;
pub const LINK_RECORD_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkConfiguration {
    pub local: [u8; 4], pub remote: [u8; 4], pub local_port: u16, pub remote_port: u16, pub revision: u64,
}
impl LinkConfiguration {
    // ------------------------=
    // FUNC: valid
    // DESC: Accepts explicit unicast IPv4 endpoints only; rejects loopback, multicast, unspecified, broadcast and identical endpoints.
    // ------------------=
    pub fn valid(&self) -> bool {
        [self.local, self.remote].iter().all(|ip| ip[0] != 0 && ip[0] != 127 && ip[0] < 224 && *ip != [255; 4]) && self.local != self.remote && self.local_port != 0 && self.remote_port != 0 && self.revision != 0
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes the public endpoint intent without runtime capabilities or connection handles.
    // ------------------=
    pub fn encode(self) -> [u8; LINK_RECORD_BYTES] {
        let mut out = [0; LINK_RECORD_BYTES]; out[0] = 1;
        out[4..8].copy_from_slice(&self.local); out[8..12].copy_from_slice(&self.remote);
        out[12..14].copy_from_slice(&self.local_port.to_le_bytes()); out[14..16].copy_from_slice(&self.remote_port.to_le_bytes()); out[16..24].copy_from_slice(&self.revision.to_le_bytes()); out
    }
}
impl NodeRuntime {
    // ------------------------=
    // FUNC: configured_links
    // DESC: Returns durable operator-approved endpoints without implying discovery or reachability.
    // ------------------=
    pub fn configured_links(&self) -> &[Option<LinkConfiguration>; MAX_CONFIGURED_LINKS] { &self.configured_links }

    // ------------------------=
    // FUNC: commit_link_configuration
    // DESC: Commits one explicit endpoint slot through the authoritative durable transaction boundary before the background reconciler may create network authority.
    // ------------------=
    pub fn commit_link_configuration(&mut self, operation: OperationId, request: NodeOperationV1, now: u64, correlation: u64, causation: u64, persist: impl FnOnce(&[u8; NODE_STATE_BYTES]) -> bool) -> Result<(NodeOperationV1, CommittedControl), CommitError> {
        if request.operation != operation.machine_id() || request.schema_version != 1 || !(1..=MAX_CONFIGURED_LINKS as u64).contains(&request.handle) { return Err(CommitError::InvalidOperation); }
        let index = request.handle as usize - 1;
        let mut staged = self.clone();
        let next = match operation {
            OperationId::NodeLinkConfigure => {
                if request.node_id[8..].iter().any(|v| *v != 0) || request.scope != 0 || request.flags != 0 || request.lease_deadline != 0 || request.rights > u16::MAX as u32 || request.value > u16::MAX as u32 { return Err(CommitError::InvalidOperation); }
                let mut local = [0; 4]; let mut remote = [0; 4]; local.copy_from_slice(&request.node_id[..4]); remote.copy_from_slice(&request.node_id[4..8]);
                let value = LinkConfiguration { local, remote, local_port: request.rights as u16, remote_port: request.value as u16, revision: self.control_version.checked_add(1).ok_or(CommitError::VersionExhausted)? };
                if !value.valid() || self.configured_links.iter().enumerate().any(|(i, existing)| i != index && existing.map(|e| e.local == local && e.local_port == value.local_port).unwrap_or(false)) { return Err(CommitError::InvalidOperation); }
                Some(value)
            }
            OperationId::NodeLinkRemove => None,
            _ => return Err(CommitError::InvalidOperation),
        };
        staged.configured_links[index] = next;
        staged.control_version = self.control_version.checked_add(1).ok_or(CommitError::VersionExhausted)?;
        let subject = self.local_id.ok_or(CommitError::InvalidState)?;
        staged.record(0xda12, subject, now, correlation, next.is_some() as u8);
        let bytes = zeroize::Zeroizing::new(staged.encode_state().map_err(|_| CommitError::InvalidState)?);
        if !persist(&bytes) { return Err(CommitError::PersistenceFailed); }
        let version = staged.control_version; *self = staged;
        Ok((request, CommittedControl { version, subject, operation, value: next.is_some() as u32, correlation, causation }))
    }

    // ------------------------=
    // FUNC: encode_link_configuration
    // DESC: Writes fixed slots so removing one endpoint never renumbers another operator reference.
    // ------------------=
    pub(super) fn encode_link_configuration(&self, bytes: &mut [u8; NODE_STATE_BYTES]) {
        for (index, link) in self.configured_links.iter().enumerate() { if let Some(link) = link { bytes[OFFSET + index * LINK_RECORD_BYTES..OFFSET + (index + 1) * LINK_RECORD_BYTES].copy_from_slice(&link.encode()); } }
    }
    // ------------------------=
    // FUNC: decode_link_configuration
    // DESC: Validates endpoint intent atomically during restore; old version-three objects contain zero slots and gain no authority.
    // ------------------=
    pub(super) fn decode_link_configuration(&mut self, bytes: &[u8]) -> Result<(), NodeError> {
        for index in 0..MAX_CONFIGURED_LINKS {
            let data = &bytes[OFFSET + index * LINK_RECORD_BYTES..OFFSET + (index + 1) * LINK_RECORD_BYTES];
            if data.iter().all(|v| *v == 0) { continue; }
            if data[0] != 1 || data[1..4].iter().chain(data[24..].iter()).any(|v| *v != 0) { return Err(NodeError::StateCorrupt); }
            let mut local = [0; 4]; let mut remote = [0; 4]; local.copy_from_slice(&data[4..8]); remote.copy_from_slice(&data[8..12]);
            let link = LinkConfiguration { local, remote, local_port: u16::from_le_bytes(data[12..14].try_into().unwrap()), remote_port: u16::from_le_bytes(data[14..16].try_into().unwrap()), revision: u64::from_le_bytes(data[16..24].try_into().unwrap()) };
            if !link.valid() || link.revision > self.control_version || self.configured_links.iter().flatten().any(|l| l.local == local && l.local_port == link.local_port) { return Err(NodeError::StateCorrupt); }
            self.configured_links[index] = Some(link);
        }
        Ok(())
    }
}
