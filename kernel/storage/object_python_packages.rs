//! One version-fenced private package archive per authenticated profile. Files
//! inside a wheel do not consume scarce global namespace or version slots.
use super::*;

#[path = "../runtime/native_python_package_limits.rs"] mod limits;
#[path = "../runtime/native_python_library_catalog.rs"] mod library_catalog;
pub use library_catalog::{Catalog as PythonLibraryCatalog, format_context as format_python_library_context,
    CONTEXT_BYTES as PYTHON_LIBRARY_CONTEXT_BYTES};
pub const MAX_PACKAGE_STATE: usize = limits::ARCHIVE_BYTES;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythonPackageRevision { pub id: ObjectId, pub version: u32 }
pub type PythonDataRevision = PythonPackageRevision;

// ------------------------=
// FUNC: package_path
// DESC: Derives the immutable profile namespace from the exact nonzero owner identity.
// ------------------=
fn package_path(owner: ObjectId) -> Result<[u8; 56], ObjectError> {
    if owner.0 == [0; 16] { return Err(ObjectError::Unauthorized); }
    let mut path = [0; 56]; path[..24].copy_from_slice(b"/system/python-packages/");
    for (i, byte) in owner.0.iter().enumerate() {
        path[24 + i * 2] = b"0123456789abcdef"[(byte >> 4) as usize];
        path[25 + i * 2] = b"0123456789abcdef"[(byte & 15) as usize];
    }
    Ok(path)
}

// ------------------------=
// FUNC: data_path
// DESC: Binds private data to a profile and immutable source identity, independently of renamed source paths or package state.
// ------------------=
fn data_path(owner: ObjectId, source: ObjectId) -> Result<[u8; 85], ObjectError> {
    if owner.0 == [0; 16] || source.0 == [0; 16] { return Err(ObjectError::Unauthorized); }
    let mut path = [0; 85]; path[..20].copy_from_slice(b"/system/python-data/");
    for (start, id) in [(20, owner), (53, source)] {
        for (i, byte) in id.0.iter().enumerate() {
            path[start + i * 2] = b"0123456789abcdef"[(byte >> 4) as usize];
            path[start + i * 2 + 1] = b"0123456789abcdef"[(byte & 15) as usize];
        }
    }
    path[52] = b'/'; Ok(path)
}

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: allow_package_namespace_mutation
    // DESC: Keeps private package bindings intact through generic namespace, alias and subtree operations.
    // ------------------=
    pub(super) fn allow_package_namespace_mutation(&self, id: ObjectId) -> Result<(), ObjectError> {
        let object = &self.state.objects[self.object_index(id)?];
        if &object.name[..object.name_len as usize] == b"@hermes-memory"
            || object.space == Space::System as u8 && matches!(&object.name[..object.name_len as usize], b"@python-packages" | b"@python-data") {
            Err(ObjectError::Unauthorized)
        } else { Ok(()) }
    }
    // ------------------------=
    // FUNC: python_package_revision
    // DESC: Resolves only an archive with the expected private kind, exact owner and reserved name.
    // ------------------=
    fn python_package_revision(&self, owner: ObjectId) -> Result<Option<PythonPackageRevision>, ObjectError> {
        let path = package_path(owner)?;
        self.private_python_revision(owner, &path, b"@python-packages")
    }
    // ------------------------=
    // FUNC: private_python_revision
    // DESC: Resolves only the reserved archive type and exact profile within its kernel-derived namespace.
    // ------------------=
    fn private_python_revision(&self, owner: ObjectId, path: &[u8], name: &[u8]) -> Result<Option<PythonPackageRevision>, ObjectError> {
        let id = match self.resolve(path) {
            Ok(id) => id,
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let object = &self.state.objects[self.object_index(id)?];
        if object.owner != owner || object.kind != ObjectType::Metadata as u8
            || object.space != Space::System as u8 || object.tombstone
            || &object.name[..object.name_len as usize] != name {
            return Err(ObjectError::Unauthorized);
        }
        Ok(Some(PythonPackageRevision { id, version: object.current_version }))
    }
    // ------------------------=
    // FUNC: read_python_packages
    // DESC: Reads one private immutable archive after its caller authenticates the live session.
    // ------------------=
    pub(crate) fn read_python_packages(&mut self, owner: ObjectId, out: &mut [u8])
        -> Result<(Option<PythonPackageRevision>, usize), ObjectError> {
        let Some(revision) = self.python_package_revision(owner)? else { return Ok((None, 0)); };
        let length = self.read_internal(revision.id, Some(revision.version), out)?;
        if length > MAX_PACKAGE_STATE { return Err(ObjectError::CorruptContent); }
        Ok((Some(revision), length))
    }
    // ------------------------=
    // FUNC: read_python_library_catalog
    // DESC: Streams one owner-checked immutable package archive, verifies its CRC and returns only sanitized import names; unchanged revisions reuse a trusted cached directory.
    // ------------------=
    pub(crate) fn read_python_library_catalog(&mut self, owner: ObjectId,
        cached: Option<(PythonPackageRevision, PythonLibraryCatalog)>)
        -> Result<(Option<PythonPackageRevision>, PythonLibraryCatalog), ObjectError> {
        let Some(revision) = self.python_package_revision(owner)? else { return Ok((None, PythonLibraryCatalog::new())); };
        if let Some((previous, catalog)) = cached { if previous == revision { return Ok((Some(revision), catalog)); } }
        let version = self.state.versions.iter().find(|v| v.used && v.object == revision.id && v.number == revision.version)
            .copied().ok_or(ObjectError::InvalidVersion)?;
        if version.storage_role != 0 || version.size as usize > MAX_PACKAGE_STATE { return Err(ObjectError::CorruptContent); }
        let base = self.store_lba + CONTENT + version.extent as u64 * ALLOCATION_BLOCK_SECTORS;
        let mut parser = library_catalog::Reader::new(); let mut crc = 0xffff_ffffu32; let mut at = 0;
        while at < version.size as usize {
            let mut sector = [0; 512];
            if !self.device.read_sector(base + at as u64 / 512, &mut sector) { return Err(ObjectError::CorruptContent); }
            let n = (version.size as usize - at).min(sector.len());
            parser.feed(&sector[..n]).map_err(|_| ObjectError::CorruptContent)?;
            for byte in &sector[..n] { crc ^= *byte as u32; for _ in 0..8 { crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 }; } }
            at += n;
        }
        if !crc != version.content_crc { return Err(ObjectError::CorruptContent); }
        Ok((Some(revision), parser.finish().map_err(|_| ObjectError::CorruptContent)?))
    }
    // ------------------------=
    // FUNC: save_python_packages
    // DESC: Atomically publishes a complete authorized pip result only if its originating archive identity and version are unchanged.
    // ------------------=
    pub(crate) fn save_python_packages(&mut self, owner: ObjectId,
        expected: Option<PythonPackageRevision>, content: &[u8]) -> Result<PythonPackageRevision, ObjectError> {
        self.save_private_python(owner, &package_path(owner)?, b"@python-packages", b"IPKG0001", expected, content)
    }
    // ------------------------=
    // FUNC: check_python_source
    // DESC: Requires the originally granted source identity and version to remain an existing personal object of this profile or the shared compatibility namespace.
    // ------------------=
    fn check_python_source(&self, owner: ObjectId, source: ObjectId, version: u32) -> Result<(), ObjectError> {
        let record = &self.state.objects[self.object_index(source)?];
        if owner.0 == [0; 16] || version == 0 || record.tombstone || record.space != Space::Personal as u8
            || record.current_version < version || (record.owner != owner && record.owner.0 != [0; 16]) {
            return Err(ObjectError::Unauthorized);
        }
        if record.owner.0 == [0; 16] && !self.state.entries.iter().any(|entry| entry.used && entry.target == source
            && entry.path[..entry.path_len as usize].starts_with(b"/home/default/documents/")) {
            return Err(ObjectError::Unauthorized);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: read_python_data
    // DESC: Loads one granted source's last durable checkpoint without exposing other apps, packages or user objects.
    // ------------------=
    pub(crate) fn read_python_data(&mut self, owner: ObjectId, source: ObjectId, source_version: u32, out: &mut [u8])
        -> Result<(Option<PythonDataRevision>, usize), ObjectError> {
        self.check_python_source(owner, source, source_version)?;
        let Some(revision) = self.private_python_revision(owner, &data_path(owner, source)?, b"@python-data")? else { return Ok((None, 0)); };
        let length = self.read_internal(revision.id, Some(revision.version), out)?;
        if length > MAX_PACKAGE_STATE { return Err(ObjectError::CorruptContent); }
        Ok((Some(revision), length))
    }
    // ------------------------=
    // FUNC: save_python_data
    // DESC: Commits a complete application-data checkpoint under the exact source grant and optimistic archive revision.
    // ------------------=
    pub(crate) fn save_python_data(&mut self, owner: ObjectId, source: ObjectId, source_version: u32,
        expected: Option<PythonDataRevision>, content: &[u8]) -> Result<PythonDataRevision, ObjectError> {
        self.check_python_source(owner, source, source_version)?;
        self.save_private_python(owner, &data_path(owner, source)?, b"@python-data", b"IDAT0001", expected, content)
    }
    // ------------------------=
    // FUNC: save_private_python
    // DESC: Uses the existing ObjectStore copy-on-write transaction for private archives while retaining old blocks until the durable root commits.
    // ------------------=
    fn save_private_python(&mut self, owner: ObjectId, path: &[u8], name: &[u8], magic: &[u8; 8],
        expected: Option<PythonPackageRevision>, content: &[u8]) -> Result<PythonPackageRevision, ObjectError> {
        if content.len() < 16 || content.len() > MAX_PACKAGE_STATE || content.get(..8) != Some(magic) {
            return Err(ObjectError::CorruptContent);
        }
        if self.private_python_revision(owner, path, name)? != expected { return Err(ObjectError::InvalidVersion); }
        let before = self.begin()?;
        let result = (|| {
            let id = if let Some(revision) = expected { revision.id } else {
                let id = self.create_record(name, ObjectType::Metadata, Space::System)?;
                let index = self.object_index(id)?; self.state.objects[index].owner = owner;
                self.attach_record(path, id)?; id
            };
            let previous = self.state.versions;
            for version in &mut self.state.versions {
                if version.used && version.object == id { version.used = false; }
            }
            // Old data remains protected until the new archive and metadata root
            // have committed. Failure rolls back to the entire previous state.
            let version = self.write_record_limit(id, content, MAX_PACKAGE_STATE)?;
            for old in previous.iter().filter(|v| v.used && v.object == id) {
                for block in old.extent as usize..old.extent as usize + old.blocks as usize {
                    self.release_unreferenced_block(block);
                }
            }
            Ok(PythonPackageRevision { id, version })
        })();
        self.finish(before, result)
    }
}
