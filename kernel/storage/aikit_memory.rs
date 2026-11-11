//! AIKit checkpoint adapter; native copy-on-write object transactions own crash consistency.
use crate::storage::{
    object::{ObjectError, ObjectId, ObjectStore, ObjectType, Space},
    BlockDevice,
};
use crate::runtime::ai::persistent_memory::{Memory, Owner};
// ------------------------=
// FUNC: path
// DESC: Uses immutable identity rather than mutable display names for private memory addressing.
// ------------------=
fn path(owner: Owner) -> [u8; 50] {
    let mut p = [0; 50];
    p[..18].copy_from_slice(b"/system/ai-memory/");
    for (i, b) in owner.iter().enumerate() {
        p[18 + i * 2] = b"0123456789abcdef"[(b >> 4) as usize];
        p[19 + i * 2] = b"0123456789abcdef"[(b & 15) as usize];
    }
    p
}
// ------------------------=
// FUNC: checkpoint
// DESC: Reads or commits a complete owner-bound memory snapshot; caller must first validate the live identity session.
// ------------------=
pub(crate) fn checkpoint<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    owner: Owner,
    save: Option<&Memory>,
    out: &mut [u8],
) -> Result<usize, ObjectError> {
    if owner == [0; 16] {
        return Err(ObjectError::Unauthorized);
    }
    let p = path(owner);
    let id = match store.resolve(&p) {
        Ok(id) => Some(id),
        Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => None,
        Err(e) => return Err(e),
    };
    if let Some(id) = id {
        let n = store.read_aikit_memory(id, ObjectId(owner), out)?;
        Memory::decode(&out[..n], owner).map_err(|_| ObjectError::CorruptContent)?;
    }
    if let Some(value) = save {
        value
            .records(owner)
            .map_err(|_| ObjectError::Unauthorized)?;
        let n = value
            .encode(out)
            .map_err(|_| ObjectError::InsufficientCapacity)?;
        if let Some(id) = id {
            store.replace_state(id, &out[..n])?;
        } else {
            store.create_owned_attached(
                b"@aikit-memory",
                ObjectType::Metadata,
                Space::Personal,
                &out[..n],
                &p,
                ObjectId(owner),
            )?;
        }
        Ok(n)
    } else if let Some(id) = id {
        store.read_aikit_memory(id, ObjectId(owner), out)
    } else {
        Memory::new(owner)
            .encode(out)
            .map_err(|_| ObjectError::InsufficientCapacity)
    }
}
