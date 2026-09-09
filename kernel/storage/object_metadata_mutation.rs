//! Shared Pool content can change only inside the fresh-quorum coordinator.
use super::*;
impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: require_pool_mutation_authority
    // DESC: Rejects direct writes once any distributed metadata exists, including staged or corrupt metadata; ordinary local objects retain their existing path.
    // ------------------=
    pub(crate) fn require_pool_mutation_authority(
        &self,
        object: ObjectId,
    ) -> Result<(), ObjectError> {
        if self.metadata_mutation_permit == Some(object) {
            return Ok(());
        }
        let prefix = b"/system/storage/pool-quorum-payload/";
        let mut path = [0; 95];
        path[..prefix.len()].copy_from_slice(prefix);
        for (i, byte) in object.0.iter().enumerate() {
            path[prefix.len() + 2 * i] = b"0123456789abcdef"[(byte >> 4) as usize];
            path[prefix.len() + 2 * i + 1] = b"0123456789abcdef"[(byte & 15) as usize];
        }
        match self.resolve(&path[..prefix.len() + 32]) {
            Ok(_) => Err(ObjectError::Unauthorized),
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => Ok(()),
            Err(e) => Err(e),
        }
    }
    // ------------------------=
    // FUNC: with_pool_mutation_authority
    // DESC: Grants one internal object-scoped mutation permit for a validated coordinator operation and clears it on both success and failure.
    // ------------------=
    pub(crate) fn with_pool_mutation_authority<T>(
        &mut self,
        object: ObjectId,
        operation: impl FnOnce(&mut Self) -> Result<T, ObjectError>,
    ) -> Result<T, ObjectError> {
        if self.metadata_mutation_permit.is_some() {
            return Err(ObjectError::Busy);
        }
        self.metadata_mutation_permit = Some(object);
        let result = operation(self);
        self.metadata_mutation_permit = None;
        result
    }
}
