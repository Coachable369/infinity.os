//! One version-fenced private package archive per authenticated profile. Files
//! inside a wheel do not consume scarce global namespace or version slots.
use super::*;

pub const MAX_PACKAGE_STATE: usize = 8 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythonPackageRevision { pub id: ObjectId, pub version: u32 }

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

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: allow_package_namespace_mutation
    // DESC: Keeps private package bindings intact through generic namespace, alias and subtree operations.
    // ------------------=
    pub(super) fn allow_package_namespace_mutation(&self, id: ObjectId) -> Result<(), ObjectError> {
        let object = &self.state.objects[self.object_index(id)?];
        if &object.name[..object.name_len as usize] == b"@hermes-memory" || object.space == Space::System as u8 && &object.name[..object.name_len as usize] == b"@python-packages" {
            Err(ObjectError::Unauthorized)
        } else { Ok(()) }
    }
    // ------------------------=
    // FUNC: python_package_revision
    // DESC: Resolves only an archive with the expected private kind, exact owner and reserved name.
    // ------------------=
    fn python_package_revision(&self, owner: ObjectId) -> Result<Option<PythonPackageRevision>, ObjectError> {
        let path = package_path(owner)?;
        let id = match self.resolve(&path) {
            Ok(id) => id,
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let object = &self.state.objects[self.object_index(id)?];
        if object.owner != owner || object.kind != ObjectType::Metadata as u8
            || object.space != Space::System as u8 || object.tombstone
            || &object.name[..object.name_len as usize] != b"@python-packages" {
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
    // FUNC: save_python_packages
    // DESC: Atomically publishes a complete authorized pip result only if its originating archive identity and version are unchanged.
    // ------------------=
    pub(crate) fn save_python_packages(&mut self, owner: ObjectId,
        expected: Option<PythonPackageRevision>, content: &[u8]) -> Result<PythonPackageRevision, ObjectError> {
        if content.len() < 16 || content.len() > MAX_PACKAGE_STATE || content.get(..8) != Some(b"IPKG0001") {
            return Err(ObjectError::CorruptContent);
        }
        let path = package_path(owner)?;
        if self.python_package_revision(owner)? != expected { return Err(ObjectError::InvalidVersion); }
        let before = self.begin()?;
        let result = (|| {
            let id = if let Some(revision) = expected { revision.id } else {
                let id = self.create_record(b"@python-packages", ObjectType::Metadata, Space::System)?;
                let index = self.object_index(id)?; self.state.objects[index].owner = owner;
                self.attach_record(&path, id)?; id
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
