//! Service-private SSH persistence; generic reads, aliases and history must be denied by the parent access gate.
use super::*;
pub(crate) const SSH_SERVICE_OWNER: ObjectId = ObjectId(*b"InfinitySSH00001");

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: read_ssh_private
    // DESC: Reads an exact typed SSH service object through the trusted service boundary only.
    // ------------------=
    pub(crate) fn read_ssh_private(&mut self, id: ObjectId, name: &[u8], out: &mut [u8]) -> Result<usize, ObjectError> {
        let record = &self.state.objects[self.object_index(id)?];
        if !matches!(name, b"@ssh-host-key" | b"@ssh-config") || record.space != Space::System as u8
            || record.kind != ObjectType::Metadata as u8 || record.owner != SSH_SERVICE_OWNER
            || &record.name[..record.name_len as usize] != name {
            return Err(ObjectError::Unauthorized);
        }
        self.read_internal(id, None, out)
    }
}
