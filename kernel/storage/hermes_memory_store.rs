//! Owner-checked Hermes persistence shared by the kernel and disk-backed harness.
use crate::storage::{
    object::{ObjectError, ObjectId, ObjectStore, ObjectType, Space},
    BlockDevice,
};
use super::hermes_profile::{Document, FILE_NAME, MAX_BYTES};

// ------------------------=
// FUNC: load_or_edit
// DESC: Loads validated owner data and commits only changed normalized documents after the caller authorizes the active session.
// ------------------=
pub(crate) fn load_or_edit<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    owner: [u8; 16],
    name: &[u8],
    wake: &[u8],
    edit: Option<(&[u8], &[u8])>,
) -> Result<Document, ObjectError> {
    let home = store.ensure_user_home(ObjectId(owner), name)?;
    let path = home.child(FILE_NAME).ok_or(ObjectError::InvalidObject)?;
    store.migrate_hermes_home(ObjectId(owner), path.as_bytes())?;
    let id = match store.resolve(path.as_bytes()) {
        Ok(id) => Some(id),
        Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => None,
        Err(error) => return Err(error),
    };
    let value = if let Some(id) = id {
        let mut bytes = [0; MAX_BYTES];
        let n = store.read_hermes_memory(id, ObjectId(owner), &mut bytes)?;
        Document::parse(&bytes[..n], owner).map_err(|_| ObjectError::CorruptContent)?
    } else {
        Document::new(owner, name, wake).map_err(|_| ObjectError::CorruptContent)?
    };
    let Some((key, value_bytes)) = edit else {
        if id.is_none() {
            store.create_owned_attached(
                b"@hermes-memory", ObjectType::Metadata, Space::Personal,
                value.bytes(), path.as_bytes(), ObjectId(owner),
            )?;
        }
        return Ok(value);
    };
    let mut next = value.updated(owner, key, value_bytes).map_err(|_| ObjectError::CorruptContent)?;
    if matches!(key, b"interests" | b"dislikes") {
        let opposite = if key == b"interests" { b"dislikes".as_slice() } else { b"interests".as_slice() };
        let mut previous = [0; 1024];
        if next.get(opposite, &mut previous).is_ok_and(|n| previous[..n].eq_ignore_ascii_case(value_bytes)) {
            next = next.updated(owner, opposite, b"").map_err(|_| ObjectError::CorruptContent)?;
        }
        next = next.updated(owner, b"inferred_interests", b"").map_err(|_| ObjectError::CorruptContent)?;
    }
    if let Some(id) = id {
        if next.bytes() != value.bytes() {
            store.replace_state(id, next.bytes())?;
        }
    } else {
        store.create_owned_attached(
            b"@hermes-memory", ObjectType::Metadata, Space::Personal,
            next.bytes(), path.as_bytes(), ObjectId(owner),
        )?;
    }
    Ok(next)
}
