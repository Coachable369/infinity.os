//! Internal bounded streaming writes into unpublished native extents. No
//! application receives a readable version before a verified atomic seal.
use super::*;
pub(super) const MAX_STAGED_CONTENT: usize = 1024 * 1024;

impl<D: BlockDevice> ObjectStore<D> {
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
    fn staging_extent_record(&mut self, size: u32) -> Result<ObjectId, ObjectError> {
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
            || (writing && version.storage_role != 1) { return Err(ObjectError::InvalidVersion); }
        Ok(version)
    }

    // ------------------------=
    // FUNC: write_staging_range
    // DESC: Writes and flushes at most one 1-KiB transfer window, never exposing a staging version to ordinary readers.
    // ------------------=
    pub(crate) fn write_staging_range(&mut self, id: ObjectId, offset: u64, bytes: &[u8]) -> Result<(), ObjectError> {
        let version = self.stream_extent_range(id, offset, bytes.len(), true)?;
        let base = self.container_lba + STORE_RELATIVE_LBA + CONTENT + version.extent as u64 * ALLOCATION_BLOCK_SECTORS;
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
        let base = self.container_lba + STORE_RELATIVE_LBA + CONTENT + version.extent as u64 * ALLOCATION_BLOCK_SECTORS;
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
        checkpoint: ObjectId, bytes: &[u8]) -> Result<(), ObjectError> {
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
            Ok(())
        })();
        self.finish(before, result)
    }
}
