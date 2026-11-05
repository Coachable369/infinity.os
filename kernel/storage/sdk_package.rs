//! Immutable SDK resources shared by live and installed native kernels.
use super::{ContentType, ObjectError, ObjectId, ObjectMetadata, ObjectRef, ObjectType, Space};

pub struct Entry {
    pub path: &'static [u8],
    pub id: ObjectId,
    pub data: Option<&'static [u8]>,
    pub text: bool,
    pub crc: u32,
}

include!("../../build/sdk/catalog.rs");

// ------------------------=
// FUNC: owns
// DESC: Reserves the complete SDK subtree against persistent namespace shadowing.
// ------------------=
pub fn owns(path: &[u8]) -> bool {
    path == b"/InfinitySDK" || path.starts_with(b"/InfinitySDK/")
        || path==b"/System" || (path.starts_with(b"/System/") && resolve(path).is_some())
}

// ------------------------=
// FUNC: get
// DESC: Looks up an immutable package resource by stable object identity.
// ------------------=
pub fn get(id: ObjectId) -> Option<&'static Entry> {
    ENTRIES.iter().find(|entry| entry.id == id)
}

// ------------------------=
// FUNC: resolve
// DESC: Resolves an exact public SDK path without consuming persistent object slots.
// ------------------=
pub fn resolve(path: &[u8]) -> Option<ObjectId> {
    ENTRIES.iter().find(|entry| entry.path == path).map(|entry| entry.id)
}

// ------------------------=
// FUNC: metadata
// DESC: Exposes shared system-owned metadata for both directories and package files.
// ------------------=
pub fn metadata(entry: &Entry) -> ObjectMetadata {
    let text = entry.text;
    ObjectMetadata {
        object: ObjectRef { id: entry.id },
        kind: if entry.data.is_none() { ObjectType::NamespaceNode } else if text { ObjectType::Text } else { ObjectType::Blob },
        space: Space::System,
        owner: ObjectId([0; 16]),
        content_type: if entry.data.is_none() { ContentType::Namespace } else if text { ContentType::Utf8Text } else { ContentType::Binary },
        flags: 0, current_version: 1,
        logical_size: entry.data.map_or(0, |data| data.len() as u32),
        created: 0, modified: 0, tags: [0; 8], tags_len: 0,
    }
}

// ------------------------=
// FUNC: read
// DESC: Reads a complete immutable resource without returning silently truncated content.
// ------------------=
pub fn read(entry: &Entry, version: Option<u32>, out: &mut [u8]) -> Result<usize, ObjectError> {
    if version.is_some_and(|v| v != 1) { return Err(ObjectError::InvalidVersion); }
    let data = entry.data.ok_or(ObjectError::UnsupportedFormat)?;
    if out.len() < data.len() { return Err(ObjectError::InsufficientCapacity); }
    out[..data.len()].copy_from_slice(data);
    Ok(data.len())
}
