//! Streaming replica adapter: constant-size RAM, native reservation/checkpoint
//! transactions, and a single atomic content seal plus Available publication.
use super::*;
const RECORD_BYTES: usize = 160;

/// Volatile verifier state owned by a bounded service job, never serialized or
/// supplied by a remote peer. The object-store borrow lasts only for one tick.
pub(crate) struct ExtentVerification {
    backing: ObjectId, extent: ObjectId, transfer: Transfer,
    verified: u64, digest: Sha256, crc: u32,
}
impl ExtentVerification {
    // ------------------------=
    // FUNC: resume
    // DESC: Creates an identity-bound volatile verifier from durable state; recovery always starts integrity checking at byte zero.
    // ------------------=
    pub(crate) fn resume<D: BlockDevice>(store: &mut ObjectStore<D>, backing: ObjectId,
        resource: ResourceId, generation: u64) -> Result<Self, ReplicaError> {
        let native = NativeExtentReplica::open(store, backing, resource, generation)?;
        let current = native.inspect().ok_or(ReplicaError::Incomplete)?;
        if current.copied != current.descriptor.bytes { return Err(ReplicaError::Incomplete); }
        Ok(Self { backing, extent: native.extent, transfer: Transfer::resume(current)?,
            verified: 0, digest: Sha256::new(), crc: 0xffff_ffff })
    }
    // ------------------------=
    // FUNC: verified_bytes
    // DESC: Reports observed verification progress independently from durable copied bytes and availability.
    // ------------------=
    pub(crate) fn verified_bytes(&self) -> u64 { self.verified }
    // ------------------------=
    // FUNC: tick
    // DESC: Reopens the exact durable replica and verifies at most one KiB; no object-store borrow or whole-content buffer survives the tick.
    // ------------------=
    pub(crate) fn tick<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>) -> Result<ReplicaState, ReplicaError> {
        let expected = self.transfer.inspect();
        let mut native = NativeExtentReplica::open(store, self.backing,
            expected.descriptor.resource, expected.descriptor.generation)?;
        if native.extent != self.extent || native.inspect() != Some(expected) { return Err(ReplicaError::Stale); }
        native.verified = self.verified; native.digest = self.digest.clone(); native.crc = self.crc;
        let result = self.transfer.verify_tick(&mut native);
        if result.is_ok() {
            self.verified = native.verified; self.digest = native.digest.clone(); self.crc = native.crc;
        } else {
            // A failed publication may have advanced either verifier before its
            // durable commit. Never carry those partial hashes into a retry.
            self.transfer = Transfer::resume(native.inspect().ok_or(ReplicaError::Incomplete)?)?;
            self.verified = 0; self.digest = Sha256::new(); self.crc = 0xffff_ffff;
        }
        result
    }
}

pub(crate) struct NativeExtentReplica<'a, D: BlockDevice> {
    store: &'a mut ObjectStore<D>, backing: ObjectId, resource: ResourceId, generation: u64,
    extent: ObjectId, current: Option<Checkpoint>, pending_end: Option<u64>,
    verified: u64, digest: Sha256, crc: u32,
}
impl<'a, D: BlockDevice> NativeExtentReplica<'a, D> {
    // ------------------------=
    // FUNC: open
    // DESC: Restores only committed transfer state and restarts bounded integrity verification after process loss.
    // ------------------=
    pub(crate) fn open(store: &'a mut ObjectStore<D>, backing: ObjectId,
        resource: ResourceId, generation: u64) -> Result<Self, ReplicaError> {
        let mut bytes = [0; RECORD_BYTES];
        let length = store.read(backing, None, &mut bytes).map_err(|_| ReplicaError::Storage)?;
        let (extent, current) = if length == 0 { (ObjectId([0; 16]), None) } else {
            if length != RECORD_BYTES || &bytes[144..152] != b"EXTENT01" || bytes[152..] != [0; 8] { return Err(ReplicaError::Invalid); }
            let extent = ObjectId(bytes[128..144].try_into().unwrap());
            if extent.0 == [0; 16] || extent == backing { return Err(ReplicaError::Invalid); }
            (extent, Some(decode_header(&bytes[..128])?))
        };
        let value = Self { store, backing, resource, generation, extent, current, pending_end: None,
            verified: 0, digest: Sha256::new(), crc: 0xffff_ffff };
        if let Some(current) = value.current { value.validate(&current.descriptor)?; }
        Ok(value)
    }
    // ------------------------=
    // FUNC: inspect
    // DESC: Returns the durable checkpoint, never uncommitted bytes or mutable backing ownership.
    // ------------------=
    pub(crate) fn inspect(&self) -> Option<Checkpoint> { self.current }
    // ------------------------=
    // FUNC: read_committed_range
    // DESC: Serves a bounded range of the exact committed object; the remote reader must authenticate the complete manifest chunk before releasing bytes to its caller.
    // ------------------=
    pub(crate) fn read_committed_range(&mut self, offset: u64, out: &mut [u8], object_hash: [u8;32]) -> Result<(), ReplicaError> {
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        if current.state != ReplicaState::Available { return Err(ReplicaError::Incomplete); }
        self.validate(&current.descriptor)?;
        if current.descriptor.hash != object_hash { return Err(ReplicaError::Integrity); }
        if out.is_empty() || out.len() > 64 || offset.checked_add(out.len() as u64).is_none_or(|n| n > current.descriptor.bytes) {
            return Err(ReplicaError::Invalid);
        }
        self.store.read_extent_range(self.extent, offset, out).map_err(|_| ReplicaError::Storage)
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Fences every range and commit by exact immutable descriptor and storage generation.
    // ------------------=
    fn validate(&self, descriptor: &ReplicaDescriptor) -> Result<(), ReplicaError> {
        if descriptor.resource != self.resource || descriptor.generation != self.generation { return Err(ReplicaError::Stale); }
        if descriptor.bytes > 1024 * 1024 { return Err(ReplicaError::Invalid); }
        if self.current.is_some_and(|c| c.descriptor != *descriptor) { return Err(ReplicaError::Conflict); }
        Ok(())
    }
    // ------------------------=
    // FUNC: record
    // DESC: Encodes the stable application identity and its separate internal physical extent reference.
    // ------------------=
    pub(super) fn record(next: &Checkpoint, extent: ObjectId) -> [u8; RECORD_BYTES] {
        let mut out = [0; RECORD_BYTES]; encode(next, &mut out[..128]);
        out[128..144].copy_from_slice(&extent.0); out[144..152].copy_from_slice(b"EXTENT01"); out
    }
    // ------------------------=
    // FUNC: read_verified_chunk
    // DESC: Returns one committed range only when its authoritative manifest chunk hash matches freshly read storage bytes.
    // ------------------=
    pub(crate) fn read_verified_chunk(&mut self, offset: u64, out: &mut [u8], hash: [u8; 32]) -> Result<(), ReplicaError> {
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        if current.state != ReplicaState::Available { return Err(ReplicaError::Incomplete); }
        self.validate(&current.descriptor)?;
        self.store.read_extent_range(self.extent, offset, out).map_err(|_| ReplicaError::Storage)?;
        let actual: [u8; 32] = Sha256::digest(&*out).into();
        if actual != hash { out.fill(0); return Err(ReplicaError::Integrity); }
        Ok(())
    }
}
impl<D: BlockDevice> ReplicaStore for NativeExtentReplica<'_, D> {
    // ------------------------=
    // FUNC: write_staging
    // DESC: Flushes a contiguous bounded chunk without advancing the authoritative durable copy offset.
    // ------------------=
    fn write_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError> {
        self.validate(descriptor)?;
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        let end = offset.checked_add(bytes.len() as u64).ok_or(ReplicaError::Invalid)?;
        if !matches!(current.state, ReplicaState::Planned | ReplicaState::Copying)
            || offset != current.copied || bytes.is_empty() || bytes.len() > TRANSFER_CHUNK || end > descriptor.bytes { return Err(ReplicaError::Invalid); }
        self.store.write_staging_range(self.extent, offset, bytes).map_err(|_| ReplicaError::Storage)?;
        self.pending_end = Some(end); Ok(())
    }
    // ------------------------=
    // FUNC: read_staging
    // DESC: Reads committed staging ranges without letting duplicate probes mutate verification progress.
    // ------------------=
    fn read_staging(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &mut [u8]) -> Result<(), ReplicaError> {
        self.validate(descriptor)?;
        let current = self.current.ok_or(ReplicaError::Incomplete)?;
        if offset.checked_add(bytes.len() as u64).is_none_or(|end| end > current.copied) { return Err(ReplicaError::Invalid); }
        self.store.read_extent_range(self.extent, offset, bytes).map_err(|_| ReplicaError::Storage)?;
        Ok(())
    }
    // ------------------------=
    // FUNC: observe_verification
    // DESC: Independently tracks only the ordered verification scan, not retransmitted chunk comparison reads.
    // ------------------=
    fn observe_verification(&mut self, descriptor: &ReplicaDescriptor, offset: u64, bytes: &[u8]) -> Result<(), ReplicaError> {
        self.validate(descriptor)?;
        if self.current.is_none_or(|c| c.state != ReplicaState::Verifying)
            || offset != self.verified || bytes.len() > TRANSFER_CHUNK
            || offset.checked_add(bytes.len() as u64).is_none_or(|end| end > descriptor.bytes) {
            return Err(ReplicaError::Conflict);
        }
            self.digest.update(&*bytes);
            for byte in &*bytes {
                self.crc ^= *byte as u32;
                for _ in 0..8 { self.crc = (self.crc >> 1) ^ if self.crc & 1 != 0 { 0xedb8_8320 } else { 0 }; }
            }
            self.verified += bytes.len() as u64;
        Ok(())
    }
    // ------------------------=
    // FUNC: checkpoint
    // DESC: Atomically persists legal transfer transitions; initial capacity and transfer ownership commit together.
    // ------------------=
    fn checkpoint(&mut self, next: &Checkpoint) -> Result<(), ReplicaError> {
        self.validate(&next.descriptor)?; Transfer::resume(*next)?;
        if self.current == Some(*next) { return Ok(()); }
        if let Some(old) = self.current {
            let allowed = match (old.state, next.state) {
                (ReplicaState::Planned | ReplicaState::Copying, ReplicaState::Copying) =>
                    self.pending_end == Some(next.copied) && next.copied > old.copied,
                (ReplicaState::Planned | ReplicaState::Copying, ReplicaState::Verifying) =>
                    next.copied == old.copied && next.copied == next.descriptor.bytes,
                (ReplicaState::Verifying, ReplicaState::Failed) => next.copied == old.copied,
                _ => false,
            };
            if !allowed { return Err(ReplicaError::Conflict); }
            self.store.replace_state(self.backing, &Self::record(next, self.extent)).map_err(|_| ReplicaError::Storage)?;
        } else {
            if next.state != ReplicaState::Planned || next.copied != 0 { return Err(ReplicaError::Conflict); }
            self.extent = self.store.create_staging_checkpoint(next.descriptor.bytes as u32, self.backing,
                |extent| Self::record(next, extent)).map_err(|_| ReplicaError::Storage)?;
        }
        if next.state == ReplicaState::Verifying { self.verified = 0; self.digest = Sha256::new(); self.crc = 0xffff_ffff; }
        self.current = Some(*next); self.pending_end = None; Ok(())
    }
    // ------------------------=
    // FUNC: publish_verified
    // DESC: Requires an independent complete SHA-256 scan before atomically sealing content and publishing Available.
    // ------------------=
    fn publish_verified(&mut self, next: &Checkpoint) -> Result<(), ReplicaError> {
        self.validate(&next.descriptor)?; Transfer::resume(*next)?;
        if self.current == Some(*next) { return Ok(()); }
        let old = self.current.ok_or(ReplicaError::Incomplete)?;
        if old.state != ReplicaState::Verifying || next.state != ReplicaState::Available || next.copied != old.copied
            || self.verified != next.descriptor.bytes { return Err(ReplicaError::Incomplete); }
        let hash: [u8; 32] = self.digest.clone().finalize().into();
        if hash != next.descriptor.hash { return Err(ReplicaError::Integrity); }
        self.store.seal_extent_checkpoint(self.extent, !self.crc, self.backing, &Self::record(next, self.extent))
            .map_err(|_| ReplicaError::Storage)?;
        self.current = Some(*next); Ok(())
    }
}
