//! Bounded, resumable immutable replica transfer. The native storage adapter
//! owns durable checkpoints and atomic publication; transport never selects an
//! object version or turns an unverified staging object into readable content.
use sha2::{Digest, Sha256};
use super::resources::ResourceId;

pub const TRANSFER_CHUNK: usize = 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaState { Planned, Copying, Verifying, Available, Failed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaError { Invalid, Conflict, Incomplete, Integrity, Storage, AccessDenied, Stale }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicaDescriptor {
    pub job: u64,
    pub object: [u8; 16],
    pub version: u64,
    pub resource: ResourceId,
    pub generation: u64,
    pub bytes: u64,
    pub hash: [u8; 32],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint { pub descriptor: ReplicaDescriptor, pub copied: u64, pub state: ReplicaState }

pub trait ReplicaStore {
    // ------------------------=
    // FUNC: write_staging
    // DESC: Writes one bounded immutable-version range without publishing it to readers.
    // ------------------=
    fn write_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError>;
    // ------------------------=
    // FUNC: read_staging
    // DESC: Reads an exact bounded staging range for duplicate detection and full read-back verification.
    // ------------------=
    fn read_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &mut [u8]) -> Result<(), ReplicaError>;
    // ------------------------=
    // FUNC: checkpoint
    // DESC: Durably records transfer state, returning success only after the normal native commit boundary.
    // ------------------=
    fn checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), ReplicaError>;
    // ------------------------=
    // FUNC: publish_verified
    // DESC: Atomically makes verified immutable content and its Available checkpoint durable; retries must be idempotent.
    // ------------------=
    fn publish_verified(&mut self, checkpoint: &Checkpoint) -> Result<(), ReplicaError>;
}

pub struct Transfer { current: Checkpoint, verified: u64, digest: Sha256 }
impl Transfer {
    // ------------------------=
    // FUNC: begin
    // DESC: Creates a durable planned job with exact object/version/resource generation and expected integrity.
    // ------------------=
    pub fn begin(store: &mut impl ReplicaStore, descriptor: ReplicaDescriptor) -> Result<Self, ReplicaError> {
        if descriptor.job == 0 || descriptor.object == [0; 16] || descriptor.version == 0
            || descriptor.resource.0 == [0; 16] || descriptor.generation == 0 {
            return Err(ReplicaError::Invalid);
        }
        let current = Checkpoint { descriptor, copied: 0, state: ReplicaState::Planned };
        store.checkpoint(&current)?;
        Ok(Self { current, verified: 0, digest: Sha256::new() })
    }

    // ------------------------=
    // FUNC: resume
    // DESC: Resumes durable transfer state; partial hash verification always restarts from byte zero after recovery.
    // ------------------=
    pub fn resume(current: Checkpoint) -> Result<Self, ReplicaError> {
        let descriptor = current.descriptor;
        if descriptor.job == 0 || descriptor.object == [0; 16] || descriptor.version == 0
            || descriptor.resource.0 == [0; 16] || descriptor.generation == 0
            || current.copied > descriptor.bytes
            || (matches!(current.state, ReplicaState::Verifying | ReplicaState::Available) && current.copied != descriptor.bytes)
            || (current.state == ReplicaState::Planned && current.copied != 0) {
            return Err(ReplicaError::Invalid);
        }
        Ok(Self { current, verified: 0, digest: Sha256::new() })
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns typed observable state without exposing mutable job ownership.
    // ------------------=
    pub fn inspect(&self) -> Checkpoint { self.current }

    // ------------------------=
    // FUNC: receive
    // DESC: Commits contiguous bounded chunks; exact duplicate retries succeed, conflicts and gaps cannot advance state.
    // ------------------=
    pub fn receive(&mut self, store: &mut impl ReplicaStore, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError> {
        if bytes.is_empty() || bytes.len() > TRANSFER_CHUNK { return Err(ReplicaError::Invalid); }
        let end = offset.checked_add(bytes.len() as u64).ok_or(ReplicaError::Invalid)?;
        if end > self.current.descriptor.bytes { return Err(ReplicaError::Invalid); }
        if end <= self.current.copied {
            let mut existing = [0u8; TRANSFER_CHUNK];
            store.read_staging(&self.current.descriptor, offset, &mut existing[..bytes.len()])?;
            return if existing[..bytes.len()] == *bytes { Ok(()) } else { Err(ReplicaError::Conflict) };
        }
        if !matches!(self.current.state, ReplicaState::Planned | ReplicaState::Copying) { return Err(ReplicaError::Invalid); }
        if offset != self.current.copied { return Err(ReplicaError::Incomplete); }
        store.write_staging(&self.current.descriptor, offset, bytes)?;
        let next = Checkpoint { copied: end, state: ReplicaState::Copying, ..self.current };
        store.checkpoint(&next)?;
        self.current = next;
        Ok(())
    }

    // ------------------------=
    // FUNC: verify_tick
    // DESC: Reads at most one chunk per tick, rejects corrupt replicas and publishes only after complete SHA-256 read-back.
    // ------------------=
    pub fn verify_tick(&mut self, store: &mut impl ReplicaStore) -> Result<ReplicaState, ReplicaError> {
        if self.current.state == ReplicaState::Available { return Ok(ReplicaState::Available); }
        if self.current.state == ReplicaState::Failed { return Err(ReplicaError::Integrity); }
        if self.current.copied != self.current.descriptor.bytes { return Err(ReplicaError::Incomplete); }
        if self.current.state != ReplicaState::Verifying {
            let next = Checkpoint { state: ReplicaState::Verifying, ..self.current };
            store.checkpoint(&next)?; self.current = next;
        }
        if self.verified < self.current.descriptor.bytes {
            let size = (self.current.descriptor.bytes - self.verified).min(TRANSFER_CHUNK as u64) as usize;
            let mut bytes = [0u8; TRANSFER_CHUNK];
            store.read_staging(&self.current.descriptor, self.verified, &mut bytes[..size])?;
            self.digest.update(&bytes[..size]); self.verified += size as u64;
        }
        if self.verified != self.current.descriptor.bytes { return Ok(ReplicaState::Verifying); }
        let actual: [u8; 32] = self.digest.clone().finalize().into();
        if actual != self.current.descriptor.hash {
            let failed = Checkpoint { state: ReplicaState::Failed, ..self.current };
            store.checkpoint(&failed)?; self.current = failed;
            return Err(ReplicaError::Integrity);
        }
        let ready = Checkpoint { state: ReplicaState::Available, ..self.current };
        store.publish_verified(&ready)?;
        self.current = ready;
        Ok(ReplicaState::Available)
    }
}
