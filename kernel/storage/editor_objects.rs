//! Authenticated editor adapter; private checkpoints remain inaccessible to generic object APIs.
use crate::storage::{object::{ObjectError, ObjectId, ObjectStore, Space}, BlockDevice};
use super::hermes_profile::{Document, MAX_BYTES};

pub type MemoryRevision = Option<(ObjectId, u32)>;

// ------------------------=
// FUNC: load
// DESC: Reads ordinary permitted objects or the exact owner's Hermes document without exposing other private checkpoints.
// ------------------=
pub(crate) fn load<D: BlockDevice>(store: &mut ObjectStore<D>, owner: ObjectId,
    path: &[u8], out: &mut [u8]) -> Result<(usize, MemoryRevision), ObjectError> {
    let id = store.resolve(path)?;
    authorize_owner(store, id, owner)?;
    match store.read(id, None, out) {
        Ok(length) => Ok((length, None)),
        Err(ObjectError::Unauthorized) => {
            let mut bytes = [0; MAX_BYTES];
            let length = store.read_hermes_memory(id, owner, &mut bytes)?;
            Document::parse(&bytes[..length], owner.0).map_err(|_| ObjectError::CorruptContent)?;
            let length = ascii_json(&bytes[..length], out)?;
            Ok((length, Some((id, store.current_version(id)?))))
        }
        Err(error) => Err(error),
    }
}

// ------------------------=
// FUNC: save
// DESC: Commits validated owner-bound memory with a version fence, or preserves ordinary public object write policy.
// ------------------=
pub(crate) fn save<D: BlockDevice>(store: &mut ObjectStore<D>, owner: ObjectId,
    path: &[u8], content: &[u8], expected: MemoryRevision) -> Result<MemoryRevision, ObjectError> {
    let id = store.resolve(path)?;
    authorize_owner(store, id, owner)?;
    let mut previous = [0; MAX_BYTES];
    if let Ok(length) = store.read_hermes_memory(id, owner, &mut previous) {
        let version = store.current_version(id)?;
        if expected != Some((id, version)) { return Err(ObjectError::InvalidVersion); }
        let document = Document::parse(content, owner.0).map_err(|_| ObjectError::CorruptContent)?;
        let version = if document.bytes() == &previous[..length] { version }
            else { store.replace_state(id, document.bytes())? };
        Ok(Some((id, version)))
    } else {
        store.write(id, content)?;
        Ok(None)
    }
}

// ------------------------=
// FUNC: authorize_owner
// DESC: Rejects missing identities and foreign Personal objects; public/system paths retain their existing object policy.
// ------------------=
fn authorize_owner<D: BlockDevice>(store: &ObjectStore<D>, id: ObjectId, owner: ObjectId) -> Result<(), ObjectError> {
    let metadata = store.metadata(id)?;
    if owner.0 == [0; 16] || metadata.space == Space::Personal
        && metadata.owner.0 != [0; 16] && metadata.owner != owner {
        return Err(ObjectError::Unauthorized);
    }
    Ok(())
}

// ------------------------=
// FUNC: ascii_json
// DESC: Preserves Unicode JSON values as standard escapes supported by the editor's current ASCII document model.
// ------------------=
fn ascii_json(input: &[u8], out: &mut [u8]) -> Result<usize, ObjectError> {
    let text = core::str::from_utf8(input).map_err(|_| ObjectError::CorruptContent)?;
    let mut length = 0;
    for ch in text.chars() {
        if ch.is_ascii() {
            *out.get_mut(length).ok_or(ObjectError::InsufficientCapacity)? = ch as u8;
            length += 1;
        } else {
            let mut units = [0; 2];
            for unit in ch.encode_utf16(&mut units).iter().copied() {
                let target = out.get_mut(length..length + 6).ok_or(ObjectError::InsufficientCapacity)?;
                target[..2].copy_from_slice(b"\\u");
                for (index, shift) in [12, 8, 4, 0].iter().enumerate() {
                    target[index + 2] = b"0123456789abcdef"[((unit >> shift) & 15) as usize];
                }
                length += 6;
            }
        }
    }
    Ok(length)
}
