//! Internal bounded streaming writes into unpublished native extents. No
//! application receives a readable version before a verified atomic seal.
use super::*;
pub(super) const MAX_STAGED_CONTENT: usize = 1024 * 1024;
pub(crate) const MAX_SYSTEM_CONTENT: usize = 128 * 1024 * 1024;

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: create_system_extent
    // DESC: Reserves one unpublished large system-component extent without weakening ordinary object size limits.
    // ------------------=
    pub(crate) fn create_system_extent(&mut self, name: &[u8], size: u32) -> Result<ObjectId, ObjectError> {
        if size == 0 || size as usize > MAX_SYSTEM_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        let before = self.begin()?;
        let result = (|| {
            let id = self.create_record(name, ObjectType::SystemComponent, Space::System)?;
            let slot = self.state.versions.iter().position(|version| !version.used)
                .ok_or(ObjectError::InsufficientCapacity)?;
            let blocks = (size as usize + 4095) / 4096;
            if blocks > u16::MAX as usize { return Err(ObjectError::InsufficientCapacity); }
            let extent = self.allocate(Space::System, blocks as u16)?;
            self.state.versions[slot] = VersionRecord { used: true, storage_role: 5, object: id,
                number: 0, extent, blocks: blocks as u16, size, content_crc: 0, parent: 0,
                generation: self.state.generation + 1 };
            Ok(id)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: seal_system_extent
    // DESC: Verifies complete streamed bytes and atomically publishes a large system component at its native namespace path.
    // ------------------=
    pub(crate) fn seal_system_extent(&mut self, id: ObjectId, path: &[u8], expected_crc: u32) -> Result<(), ObjectError> {
        validate_path(path)?;
        let version = self.state.versions.iter().find(|version| version.used && version.object == id
            && version.storage_role == 5).copied().ok_or(ObjectError::InvalidVersion)?;
        let mut crc = 0xffff_ffffu32;
        let mut offset = 0u64;
        let mut buffer = [0u8; 1024];
        while offset < version.size as u64 {
            let count = (version.size as u64 - offset).min(buffer.len() as u64) as usize;
            self.read_extent_range(id, offset, &mut buffer[..count])?;
            for byte in &buffer[..count] {
                crc ^= *byte as u32;
                for _ in 0..8 { crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1)); }
            }
            offset += count as u64;
        }
        if !crc != expected_crc { return Err(ObjectError::CorruptContent); }
        let before = self.begin()?;
        let result = (|| {
            if self.state.entries.iter().any(|entry| entry.used && entry.path() == path) {
                return Err(ObjectError::NameConflict);
            }
            let slot = self.state.versions.iter().position(|candidate| candidate.used
                && candidate.object == id && candidate.storage_role == 5)
                .ok_or(ObjectError::InvalidVersion)?;
            self.state.versions[slot].storage_role = 4;
            self.state.versions[slot].number = 1;
            self.state.versions[slot].content_crc = expected_crc;
            let object = self.object_index(id)?;
            self.state.objects[object].current_version = 1;
            self.state.objects[object].modified = self.state.generation + 1;
            self.attach_record(path, id)
        })();
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: read_system_extent
    // DESC: Reads a bounded range from a sealed large system component without allocating its full logical size.
    // ------------------=
    pub(crate) fn read_system_extent(&mut self, id: ObjectId, offset: u64, out: &mut [u8]) -> Result<(), ObjectError> {
        let version = self.state.versions.iter().find(|version| version.used && version.object == id
            && version.storage_role == 4).copied().ok_or(ObjectError::InvalidObject)?;
        if out.len() > 64 * 1024 || offset.checked_add(out.len() as u64).is_none_or(|end| end > version.size as u64) {
            return Err(ObjectError::InvalidVersion);
        }
        let mut read = 0usize;
        while read < out.len() {
            let count = (out.len() - read).min(1024);
            self.read_extent_range(id, offset + read as u64, &mut out[read..read + count])?;
            read += count;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: staging_reserved_bytes
    // DESC: Reports actual committed unpublished extent reservations; these blocks are already excluded from free allocation capacity.
    // ------------------=
    pub(crate) fn staging_reserved_bytes(&self) -> u64 {
        self.state.versions.iter().filter(|v| v.used && matches!(v.storage_role, 1 | 5))
            .map(|v| v.blocks as u64 * 4096).sum()
    }
    // ------------------------=
    // FUNC: create_replica_binding
    // DESC: Commits replica ownership, checkpoint and physical reservation together so interrupted admission cannot leak an orphan extent.
    // ------------------=
    pub(crate) fn create_replica_binding<const N: usize, const M: usize>(&mut self,
        catalog: ObjectId, size: u32, encode: impl FnOnce(ObjectId, ObjectId) -> ([u8; N], [u8; M]))
        -> Result<ObjectId, ObjectError> {
        if size as usize > MAX_STAGED_CONTENT || N > MAX_CONTENT || M > MAX_CONTENT {
            return Err(ObjectError::InsufficientCapacity);
        }
        let before = self.begin()?;
        let result = (|| {
            let backing = self.create_record(b"pool-replica", ObjectType::Metadata, Space::System)?;
            let extent = self.staging_extent_record(size)?;
            let (catalog_bytes, checkpoint) = encode(backing, extent);
            self.replace_state_record(backing, &checkpoint)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            Ok(backing)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: create_staging_checkpoint
    // DESC: Reserves the staging extent and records its owning transfer in one commit, preventing orphan reservations after failed admission.
    // ------------------=
    pub(crate) fn create_staging_checkpoint<const N: usize>(&mut self, size: u32,
        checkpoint: ObjectId, encode: impl FnOnce(ObjectId) -> [u8; N]) -> Result<ObjectId, ObjectError> {
        if size as usize > MAX_STAGED_CONTENT || N > MAX_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        let before = self.begin()?;
        let result = (|| {
            let id = self.staging_extent_record(size)?;
            self.replace_state_record(checkpoint, &encode(id))?;
            Ok(id)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: create_staging_extent
    // DESC: Durably reserves an unpublished native object without allocating a content-sized RAM buffer.
    // ------------------=
    pub(crate) fn create_staging_extent(&mut self, size: u32) -> Result<ObjectId, ObjectError> {
        if size as usize > MAX_STAGED_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        let before = self.begin()?;
        let result = self.staging_extent_record(size);
        self.finish(before, result)
    }

    // ------------------------=
    // FUNC: staging_extent_record
    // DESC: Allocates only transaction-local staging metadata and capacity; its caller owns commit or rollback.
    // ------------------=
    pub(super) fn staging_extent_record(&mut self, size: u32) -> Result<ObjectId, ObjectError> {
            let id = self.create_record(b"pool-staging", ObjectType::Metadata, Space::System)?;
            let slot = self.state.versions.iter().position(|v| !v.used).ok_or(ObjectError::InsufficientCapacity)?;
            let blocks = ((size as usize + 4095) / 4096).max(1) as u16;
            let extent = self.allocate(Space::System, blocks)?;
            self.state.versions[slot] = VersionRecord { used: true, storage_role: 1, object: id,
                number: 0, extent, blocks, size, content_crc: 0, parent: 0, generation: self.state.generation + 1 };
            Ok(id)
    }

    // ------------------------=
    // FUNC: stream_extent_range
    // DESC: Validates one exact internal range; callers must supply their authenticated transfer authority separately.
    // ------------------=
    fn stream_extent_range(&self, id: ObjectId, offset: u64, size: usize, writing: bool) -> Result<VersionRecord, ObjectError> {
        let object = self.object_index(id)?;
        if self.state.objects[object].tombstone { return Err(ObjectError::NotFound); }
        let version = self.state.versions.iter().find(|v| v.used && v.object == id
            && v.storage_role != 0).copied().ok_or(ObjectError::InvalidObject)?;
        if size > 1024 || offset.checked_add(size as u64).is_none_or(|end| end > version.size as u64)
            || (writing && !matches!(version.storage_role, 1 | 5)) { return Err(ObjectError::InvalidVersion); }
        Ok(version)
    }

    // ------------------------=
    // FUNC: write_staging_range
    // DESC: Writes and flushes at most one 1-KiB transfer window, never exposing a staging version to ordinary readers.
    // ------------------=
    pub(crate) fn write_staging_range(&mut self, id: ObjectId, offset: u64, bytes: &[u8]) -> Result<(), ObjectError> {
        let version = self.stream_extent_range(id, offset, bytes.len(), true)?;
        let base = self.store_lba + CONTENT + version.extent as u64 * ALLOCATION_BLOCK_SECTORS;
        let mut written = 0;
        while written < bytes.len() {
            let at = offset + written as u64;
            let start = (at % 512) as usize;
            let count = (bytes.len() - written).min(512 - start);
            let mut sector = [0; 512];
            if (start != 0 || count != 512) && !self.device.read_sector(base + at / 512, &mut sector) {
                return Err(ObjectError::TransactionFailed);
            }
            sector[start..start + count].copy_from_slice(&bytes[written..written + count]);
            if !self.device.write_sector(base + at / 512, &sector) { return Err(ObjectError::TransactionFailed); }
            written += count;
        }
        if !self.device.flush() { return Err(ObjectError::TransactionFailed); }
        Ok(())
    }

    // ------------------------=
    // FUNC: read_extent_range
    // DESC: Reads one bounded internal range for integrity verification or manifest-directed reads, without whole-object buffering.
    // ------------------=
    pub(crate) fn read_extent_range(&mut self, id: ObjectId, offset: u64, out: &mut [u8]) -> Result<(), ObjectError> {
        let version = self.stream_extent_range(id, offset, out.len(), false)?;
        let base = self.store_lba + CONTENT + version.extent as u64 * ALLOCATION_BLOCK_SECTORS;
        let mut read = 0;
        while read < out.len() {
            let at = offset + read as u64;
            let start = (at % 512) as usize;
            let count = (out.len() - read).min(512 - start);
            let mut sector = [0; 512];
            if !self.device.read_sector(base + at / 512, &mut sector) { return Err(ObjectError::CorruptContent); }
            out[read..read + count].copy_from_slice(&sector[start..start + count]); read += count;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: seal_extent_checkpoint
    // DESC: Atomically seals an integrity-verified extent and its owner checkpoint under one native root transaction; only the trusted verifier calls this boundary.
    // ------------------=
    pub(crate) fn seal_extent_checkpoint(&mut self, id: ObjectId, crc: u32,
        checkpoint: ObjectId, bytes: &[u8],audit:[u8;128]) -> Result<(), ObjectError> {
        if id == checkpoint { return Err(ObjectError::InvalidObject); }
        let before = self.begin()?;
        let result = (|| {
            let slot = self.state.versions.iter().position(|v| v.used && v.object == id && v.storage_role == 1)
                .ok_or(ObjectError::InvalidVersion)?;
            let object = self.object_index(id)?;
            self.state.versions[slot].storage_role = 2;
            self.state.versions[slot].number = 1;
            self.state.versions[slot].content_crc = crc;
            self.state.objects[object].current_version = 1;
            self.state.objects[object].modified = self.state.generation + 1;
            self.replace_state_record(checkpoint, bytes)?;
            self.append_pool_audit_record(audit)?;
            Ok(())
        })();
        self.finish(before, result)
    }
}
