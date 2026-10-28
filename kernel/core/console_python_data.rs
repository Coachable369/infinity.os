//! Source-scoped durable files over the existing private native VFS and ObjectStore.
use crate::runtime::{native_memory::PageTable, native_python_packages::Vfs};
use crate::storage::object::{ObjectError, PythonDataRevision};

pub(super) struct Data {
    owner: [u8; 16], session: [u8; 16], source: [u8; 16], source_version: u32,
    revision: Option<PythonDataRevision>, error: i64,
}
impl Data {
    // ------------------------=
    // FUNC: new
    // DESC: Starts without application-data authority; validation, module commands and pip never bind it.
    // ------------------=
    pub(super) const fn new() -> Self {
        Self { owner: [0; 16], session: [0; 16], source: [0; 16], source_version: 0, revision: None, error: 0 }
    }
    // ------------------------=
    // FUNC: bind
    // DESC: Loads one exact source identity's checkpoint after the authenticated launcher has snapshotted that source.
    // ------------------=
    pub(super) fn bind(&mut self, filesystem: &mut Vfs, archive: &mut [u8], owner: [u8; 16],
        session: [u8; 16], source: [u8; 16], version: u32) -> Result<(), i64> {
        if self.source != [0; 16] { return Err(-16); }
        let loaded = crate::storage::python_data_load(owner, session, source, version, archive).map_err(errno);
        let result = loaded.and_then(|(revision, length)| {
            filesystem.grant_data(&archive[..length])?;
            *self = Self { owner, session, source, source_version: version, revision, error: 0 }; Ok(())
        });
        archive.fill(0); result
    }
    // ------------------------=
    // FUNC: checkpoint
    // DESC: Acknowledges a barrier only after native copy-on-write storage commits; failed revisions and disk writes freeze further data mutation.
    // ------------------=
    pub(super) fn checkpoint(&mut self, filesystem: &mut Vfs, archive: &mut [u8]) -> Result<(), i64> {
        if self.error != 0 { return Err(self.error); }
        if self.source == [0; 16] || !filesystem.data_dirty() { return Ok(()); }
        let result = filesystem.encode_data(archive).and_then(|length|
            crate::storage::python_data_save(self.owner, self.session, self.source, self.source_version,
                self.revision, &archive[..length]).map_err(errno));
        archive.fill(0);
        match result {
            Ok(revision) => { self.revision = Some(revision); filesystem.data_checkpointed(); Ok(()) }
            Err(error) => { self.error = error; filesystem.data_checkpoint_failed(); Err(error) }
        }
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Preserves the existing filesystem ABI while making data fsync, close and namespace mutation genuinely durable before userspace observes success.
    // ------------------=
    pub(super) fn syscall<T: PageTable>(&mut self, filesystem: &mut Vfs, archive: &mut [u8],
        tables: &mut T, operation: u64, arguments: [u64; 6]) -> Option<i64> {
        let result = filesystem.syscall(tables, operation, arguments)?;
        Some(if filesystem.data_barrier(operation, result) {
            self.checkpoint(filesystem, archive).err().unwrap_or(result)
        } else { result })
    }
}

// ------------------------=
// FUNC: errno
// DESC: Converts storage failures into actionable native file errors without reporting unsuccessful durability as success.
// ------------------=
fn errno(error: ObjectError) -> i64 {
    match error {
        ObjectError::Unauthorized => -13,
        ObjectError::InsufficientCapacity | ObjectError::SpaceUnavailable => -28,
        ObjectError::InvalidVersion => -133, // ESTALE in the pinned Newlib target ABI, not Linux's errno number.
        ObjectError::Busy => -16,
        ObjectError::NotFound | ObjectError::NamespaceNotFound => -2,
        _ => -5,
    }
}
