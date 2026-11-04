//! Service-private SSH persistence; generic reads, aliases and history must be denied by the parent access gate.
use super::*;
pub(crate) const SSH_SERVICE_OWNER: ObjectId = ObjectId(*b"InfinitySSH00001");

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: create_user_ssh_records
    // DESC: Materializes a new owner's .ssh directory and key pair inside the account transaction without replacing any existing key.
    // ------------------=
    pub(super) fn create_user_ssh_records(&mut self, owner: ObjectId, home: user_home::Path,
        private: &[u8], public: &[u8]) -> Result<(), ObjectError> {
        if owner.0 == [0; 16] || private.is_empty() || private.len() > 512 || public.is_empty() || public.len() > 96 {
            return Err(ObjectError::InvalidObject);
        }
        let directory = home.child(b".ssh").ok_or(ObjectError::InvalidObject)?;
        for (path, name, kind, bytes) in [
            (directory, b"@ssh-user-directory".as_slice(), ObjectType::NamespaceNode, b"".as_slice()),
            (directory.child(b"id_ed25519").ok_or(ObjectError::InvalidObject)?, b"@ssh-user-private", ObjectType::Text, private),
            (directory.child(b"id_ed25519.pub").ok_or(ObjectError::InvalidObject)?, b"@ssh-user-public", ObjectType::Text, public),
        ] {
            if self.state.entries.iter().any(|e| e.used && e.path() == path.as_bytes()) { return Err(ObjectError::NameConflict); }
            let id = self.create_record(name, kind, Space::Personal)?;
            let index = self.object_index(id)?;
            self.state.objects[index].owner = owner;
            if !bytes.is_empty() { self.write_record(id, bytes)?; }
            self.attach_record(path.as_bytes(), id)?;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: read_user_ssh_key
    // DESC: Exposes current key bytes only to the exact owner after a trusted caller validates a local session; generic reads and aliases remain denied.
    // ------------------=
    pub(crate) fn read_user_ssh_key(&mut self, id: ObjectId, owner: ObjectId, out: &mut [u8]) -> Result<usize, ObjectError> {
        let record = &self.state.objects[self.object_index(id)?];
        if owner.0 == [0; 16] || record.owner != owner || record.space != Space::Personal as u8
            || record.kind != ObjectType::Text as u8
            || !matches!(&record.name[..record.name_len as usize], b"@ssh-user-private" | b"@ssh-user-public") {
            return Err(ObjectError::Unauthorized);
        }
        self.read_internal(id, None, out)
    }

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
