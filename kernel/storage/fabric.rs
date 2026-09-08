//! Internal replica persistence over native object transactions. This is not a
//! public authority boundary: the Network/IOP adapter must authorize each call.
//! One bounded state object commits checkpoint and staging bytes together.
use crate::storage::{object::{ObjectId, ObjectStore, MAX_CONTENT}, BlockDevice};
use crate::runtime::fabric::{replica::{Checkpoint, ReplicaDescriptor, ReplicaError, ReplicaState,
    ReplicaStore, Transfer, TRANSFER_CHUNK}, resources::ResourceId};
use sha2::{Digest, Sha256};

const HEADER: usize = 128;
pub(crate) const MAX_REPLICA_BYTES: usize = MAX_CONTENT - HEADER;

// ------------------------=
// FUNC: load_manifest
// DESC: Reads an authoritative manifest from a native object, fencing its application identity independently from the backing record.
// ------------------=
pub(crate) fn load_manifest<D: BlockDevice>(store: &mut ObjectStore<D>, backing: ObjectId,
    object: ObjectId) -> Result<crate::runtime::fabric::manifest::Manifest, crate::runtime::fabric::manifest::ManifestError> {
    use crate::runtime::fabric::manifest::{Manifest, ManifestError, MANIFEST_BYTES};
    let mut bytes = [0; MANIFEST_BYTES];
    let length = store.read(backing, None, &mut bytes).map_err(|_| ManifestError::Storage)?;
    let manifest = Manifest::decode(&bytes[..length])?;
    if manifest.object != object.0 { return Err(ManifestError::Conflict); }
    Ok(manifest)
}

// ------------------------=
// FUNC: commit_manifest
// DESC: Performs generation-fenced durable replacement under the native transaction root; an identical committed retry is idempotent.
// ------------------=
pub(crate) fn commit_manifest<D: BlockDevice>(store: &mut ObjectStore<D>, backing: ObjectId,
    expected: u64, next: &crate::runtime::fabric::manifest::Manifest) -> Result<(), crate::runtime::fabric::manifest::ManifestError> {
    use crate::runtime::fabric::manifest::{Manifest, ManifestError, MANIFEST_BYTES};
    let mut bytes = [0; MANIFEST_BYTES];
    let length = store.read(backing, None, &mut bytes).map_err(|_| ManifestError::Storage)?;
    next.validate()?;
    if length == 0 {
        if expected != 0 || next.generation != 1 { return Err(ManifestError::Stale); }
    } else {
        let previous = Manifest::decode(&bytes[..length])?;
        if previous == *next && expected.checked_add(1) == Some(next.generation) { return Ok(()); }
        if previous.generation != expected { return Err(ManifestError::Stale); }
        previous.successor(next)?;
    }
    next.encode(&mut bytes)?;
    store.replace_state(backing, &bytes).map_err(|_| ManifestError::Storage)?;
    Ok(())
}

pub(crate) struct NativeReplica<'a, D: BlockDevice> {
    store: &'a mut ObjectStore<D>,
    backing: ObjectId,
    resource: ResourceId,
    generation: u64,
    bytes: [u8; MAX_CONTENT],
    current: Option<Checkpoint>,
    pending_end: Option<u64>,
}

impl<'a, D: BlockDevice> NativeReplica<'a, D> {
    // ------------------------=
    // FUNC: open
    // DESC: Loads one internal replica object, rejecting malformed or replaced-resource state before recovery.
    // ------------------=
    pub(crate) fn open(store: &'a mut ObjectStore<D>, backing: ObjectId,
        resource: ResourceId, generation: u64) -> Result<Self, ReplicaError> {
        if resource.0 == [0; 16] || generation == 0 { return Err(ReplicaError::Invalid); }
        let mut bytes = [0; MAX_CONTENT];
        let length = store.read(backing, None, &mut bytes).map_err(|_| ReplicaError::Storage)?;
        let current = if length == 0 { None } else { Some(decode(&bytes[..length])?) };
        let result = Self { store, backing, resource, generation, bytes, current, pending_end: None };
        if let Some(checkpoint) = current { result.validate(&checkpoint.descriptor)?; }
        Ok(result)
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns the last committed checkpoint, never an uncommitted staging advance.
    // ------------------=
    pub(crate) fn inspect(&self) -> Option<Checkpoint> { self.current }

    // ------------------------=
    // FUNC: validate
    // DESC: Fences every access to exact resource generation and immutable transfer identity.
    // ------------------=
    fn validate(&self, descriptor: &ReplicaDescriptor) -> Result<(), ReplicaError> {
        if descriptor.resource != self.resource || descriptor.generation != self.generation {
            return Err(ReplicaError::Stale);
        }
        if descriptor.bytes > MAX_REPLICA_BYTES as u64 {
            return Err(ReplicaError::Invalid);
        }
        if self.current.is_some_and(|c| c.descriptor != *descriptor) { return Err(ReplicaError::Conflict); }
        Ok(())
    }

    // ------------------------=
    // FUNC: commit
    // DESC: Atomically replaces checkpoint and payload through the existing bounded-history native COW transaction.
    // ------------------=
    fn commit(&mut self, checkpoint: &Checkpoint) -> Result<(), ReplicaError> {
        encode(checkpoint, &mut self.bytes[..HEADER]);
        self.store.replace_state(self.backing, &self.bytes[..HEADER + checkpoint.copied as usize])
            .map_err(|_| ReplicaError::Storage)?;
        self.current = Some(*checkpoint);
        self.pending_end = None;
        Ok(())
    }

    // ------------------------=
    // FUNC: read_verified
    // DESC: Serves only committed Available content with full integrity verification; staging is never readable here.
    // ------------------=
    pub(crate) fn read_verified(&mut self, out: &mut [u8]) -> Result<usize, ReplicaError> {
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        if current.state != ReplicaState::Available { return Err(ReplicaError::Incomplete); }
        // Read from the device again rather than trusting the cached transfer buffer.
        let length = self.store.read(self.backing, None, &mut self.bytes).map_err(|_| ReplicaError::Storage)?;
        if decode(&self.bytes[..length])? != current { return Err(ReplicaError::Conflict); }
        let size = current.copied as usize;
        if out.len() < size { return Err(ReplicaError::Invalid); }
        let payload = &self.bytes[HEADER..HEADER + size];
        let hash: [u8; 32] = Sha256::digest(payload).into();
        if hash != current.descriptor.hash { return Err(ReplicaError::Integrity); }
        out[..size].copy_from_slice(payload);
        Ok(size)
    }
}

impl<D: BlockDevice> ReplicaStore for NativeReplica<'_, D> {
    // ------------------------=
    // FUNC: write_staging
    // DESC: Buffers one contiguous chunk; only the following native checkpoint transaction makes it durable.
    // ------------------=
    fn write_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError> {
        self.validate(descriptor)?;
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        let end = offset.checked_add(bytes.len() as u64).ok_or(ReplicaError::Invalid)?;
        if !matches!(current.state, ReplicaState::Planned | ReplicaState::Copying)
            || offset != current.copied || bytes.is_empty() || bytes.len() > TRANSFER_CHUNK || end > descriptor.bytes {
            return Err(ReplicaError::Invalid);
        }
        self.bytes[HEADER + offset as usize..HEADER + end as usize].copy_from_slice(bytes);
        self.pending_end = Some(end);
        Ok(())
    }

    // ------------------------=
    // FUNC: read_staging
    // DESC: Returns only committed staging ranges for duplicate comparison and incremental verification.
    // ------------------=
    fn read_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &mut [u8]) -> Result<(), ReplicaError> {
        self.validate(descriptor)?;
        let copied = self.current.ok_or(ReplicaError::Incomplete)?.copied;
        let end = offset.checked_add(bytes.len() as u64).ok_or(ReplicaError::Invalid)?;
        if bytes.len() > TRANSFER_CHUNK || end > copied { return Err(ReplicaError::Invalid); }
        let mut committed = [0u8; MAX_CONTENT];
        let length = self.store.read(self.backing, None, &mut committed).map_err(|_| ReplicaError::Storage)?;
        if Some(decode(&committed[..length])?) != self.current { return Err(ReplicaError::Conflict); }
        bytes.copy_from_slice(&committed[HEADER + offset as usize..HEADER + end as usize]);
        Ok(())
    }

    // ------------------------=
    // FUNC: checkpoint
    // DESC: Rejects regression or fabricated transfer advances and persists allowed state transitions atomically.
    // ------------------=
    fn checkpoint(&mut self, next: &Checkpoint) -> Result<(), ReplicaError> {
        self.validate(&next.descriptor)?;
        Transfer::resume(*next)?;
        if self.current == Some(*next) { return Ok(()); }
        let allowed = match self.current {
            None => next.state == ReplicaState::Planned && next.copied == 0,
            Some(old) => match (old.state, next.state) {
                (ReplicaState::Planned | ReplicaState::Copying, ReplicaState::Copying) =>
                    self.pending_end == Some(next.copied) && next.copied > old.copied,
                (ReplicaState::Copying, ReplicaState::Verifying) => next.copied == old.copied,
                (ReplicaState::Planned, ReplicaState::Verifying) =>
                    old.descriptor.bytes == 0 && next.copied == 0,
                (ReplicaState::Verifying, ReplicaState::Failed) => next.copied == old.copied,
                _ => false,
            },
        };
        if !allowed { return Err(ReplicaError::Conflict); }
        self.commit(next)
    }

    // ------------------------=
    // FUNC: publish_verified
    // DESC: Commits Available only for the exact fully copied, integrity-checked version; publication retries are idempotent.
    // ------------------=
    fn publish_verified(&mut self, next: &Checkpoint) -> Result<(), ReplicaError> {
        self.validate(&next.descriptor)?;
        Transfer::resume(*next)?;
        if self.current == Some(*next) { return Ok(()); }
        let old = self.current.ok_or(ReplicaError::Incomplete)?;
        if old.state != ReplicaState::Verifying || next.state != ReplicaState::Available || next.copied != old.copied {
            return Err(ReplicaError::Conflict);
        }
        let hash: [u8; 32] = Sha256::digest(&self.bytes[HEADER..HEADER + old.copied as usize]).into();
        if hash != next.descriptor.hash { return Err(ReplicaError::Integrity); }
        self.commit(next)
    }
}

// ------------------------=
// FUNC: encode
// DESC: Writes a versioned fixed-width native replica checkpoint without pointers or process-local IDs.
// ------------------=
fn encode(checkpoint: &Checkpoint, out: &mut [u8]) {
    out.fill(0);
    out[..8].copy_from_slice(b"INFREPL1");
    out[8] = match checkpoint.state { ReplicaState::Planned => 1, ReplicaState::Copying => 2,
        ReplicaState::Verifying => 3, ReplicaState::Available => 4, ReplicaState::Failed => 5 };
    let d = checkpoint.descriptor;
    for (offset, value) in [(16, d.job), (40, d.version), (64, d.generation), (72, d.bytes), (112, checkpoint.copied)] {
        out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    out[24..40].copy_from_slice(&d.object);
    out[48..64].copy_from_slice(&d.resource.0);
    out[80..112].copy_from_slice(&d.hash);
}

// ------------------------=
// FUNC: decode
// DESC: Validates bounded checkpoint schema and payload extent before constructing recoverable typed state.
// ------------------=
fn decode(bytes: &[u8]) -> Result<Checkpoint, ReplicaError> {
    if bytes.len() < HEADER || &bytes[..8] != b"INFREPL1" || bytes[9..16] != [0; 7] || bytes[120..128] != [0; 8] {
        return Err(ReplicaError::Invalid);
    }
    let number = |offset: usize| u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
    let state = match bytes[8] { 1 => ReplicaState::Planned, 2 => ReplicaState::Copying,
        3 => ReplicaState::Verifying, 4 => ReplicaState::Available, 5 => ReplicaState::Failed,
        _ => return Err(ReplicaError::Invalid) };
    let checkpoint = Checkpoint { descriptor: ReplicaDescriptor { job: number(16), object: bytes[24..40].try_into().unwrap(),
        version: number(40), resource: ResourceId(bytes[48..64].try_into().unwrap()), generation: number(64),
        bytes: number(72), hash: bytes[80..112].try_into().unwrap() }, copied: number(112), state };
    if checkpoint.descriptor.bytes > MAX_REPLICA_BYTES as u64 || checkpoint.copied > MAX_REPLICA_BYTES as u64
        || bytes.len() != HEADER + checkpoint.copied as usize { return Err(ReplicaError::Invalid); }
    Transfer::resume(checkpoint)?;
    Ok(checkpoint)
}
