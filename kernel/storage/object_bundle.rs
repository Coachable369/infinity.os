//! Transaction composition independent of runtime services and wire protocols.
//! Owning services provide their bounded encoded metadata before root commit.
use super::*;

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: copy_owned_bundle
    // DESC: Atomically creates an independent object sharing the source's current immutable extent and commits its distinct manifest and catalog binding without copying payload bytes.
    // ------------------=
    pub(crate) fn copy_owned_bundle<const N: usize, const M: usize>(&mut self, source: ObjectId,
        expected_version: u64, catalog: ObjectId,
        encode: impl FnOnce(ObjectId, ObjectId) -> Result<([u8; N], [u8; M]), ObjectError>) -> Result<ObjectId, ObjectError> {
        if N > MAX_CONTENT || M > MAX_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        if self.metadata(source)?.current_version as u64 != expected_version { return Err(ObjectError::InvalidVersion); }
        let before = self.begin()?;
        let result = (|| {
            let object = self.copy_record(source, b"Pool Copy")?;
            let backing = self.create_record(b"object-manifest", ObjectType::Metadata, Space::System)?;
            let (catalog_bytes, manifest) = encode(object, backing)?;
            self.replace_state_record(backing, &manifest)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            Ok(object)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: create_owned_bundle
    // DESC: Atomically creates application content, a distinct private metadata record and its catalog binding, with rollback on any encoding or sector failure.
    // ------------------=
    pub(crate) fn create_owned_bundle<const N: usize, const M: usize>(&mut self, catalog: ObjectId,
        content: &[u8], encode: impl FnOnce(ObjectId, ObjectId) -> Result<([u8; N], [u8; M]), ObjectError>)
        -> Result<ObjectId, ObjectError> {
        if N > MAX_CONTENT || M > MAX_CONTENT || content.len() > MAX_CONTENT { return Err(ObjectError::InsufficientCapacity); }
        let before = self.begin()?;
        let result = (|| {
            let object = self.create_record(b"Pool Object", ObjectType::Metadata, Space::Personal)?;
            self.write_record(object, content)?;
            let backing = self.create_record(b"object-manifest", ObjectType::Metadata, Space::System)?;
            let (catalog_bytes, manifest) = encode(object, backing)?;
            self.replace_state_record(backing, &manifest)?;
            self.replace_state_record(catalog, &catalog_bytes)?;
            Ok(object)
        })();
        self.finish(before, result)
    }
    // ------------------------=
    // FUNC: update_owned_bundle
    // DESC: Commits a content successor and its owning metadata under one native transaction root, preserving the previous version on failed validation or persistence.
    // ------------------=
    pub(crate) fn update_owned_bundle<const N: usize>(&mut self, object: ObjectId, backing: ObjectId,
        content: &[u8], encode: impl FnOnce(u32) -> Result<[u8; N], ObjectError>) -> Result<u32, ObjectError> {
        if object == backing || N > MAX_CONTENT || content.len() > MAX_CONTENT { return Err(ObjectError::InvalidObject); }
        let before = self.begin()?;
        let result = (|| {
            let version = self.write_record(object, content)?;
            self.replace_state_record(backing, &encode(version)?)?;
            Ok(version)
        })();
        self.finish(before, result)
    }
}
