//! Object-backed stream primitives for the native C ABI. No host filesystem or
//! process-global namespace is used. Callers supply explicit namespace grants.
use crate::storage::{BlockDevice, object::{ObjectStore, ObjectId, ObjectType, Space, ObjectError, MAX_CONTENT}};

pub const READ: u32 = 1;
pub const WRITE: u32 = 2;
pub const CREATE: u32 = 4;
pub const TRUNCATE: u32 = 8;
pub const APPEND: u32 = 16;
pub const EXCLUSIVE: u32 = 32;
pub const TEXT: u32 = 64;
const MAX_PATH: usize = 95;
const DOCUMENTS: &[u8] = b"/home/default/documents";
static DOCUMENT_GRANTS: [Grant<'static>; 1] = [Grant { root: DOCUMENTS, write: true }];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum IoError { Invalid = 1, Denied, NotFound, Exists, Capacity, BadHandle, Conflict, Storage, IsDirectory }

pub struct Grant<'a> { pub root: &'a [u8], pub write: bool }

#[derive(Clone, Copy)]
struct Handle {
    token: i32,
    id: ObjectId,
    version: u32,
    flags: u32,
    position: usize,
    length: usize,
    dirty: bool,
}

/// Each open stream owns a bounded snapshot. Writes publish one version at
/// flush/close, with optimistic conflict detection against edits by other apps.
pub struct ObjectIo<'a, const N: usize> {
    grants: &'a [Grant<'a>],
    cwd: [u8; MAX_PATH],
    cwd_len: usize,
    handles: [Option<Handle>; N],
    buffers: [[u8; MAX_CONTENT]; N],
    next_token: i32,
}

// ------------------------=
// FUNC: storage_error
// DESC: Maps typed storage failures to stable C ABI error categories.
// ------------------=
fn storage_error(error: ObjectError) -> IoError {
    match error {
        ObjectError::NotFound | ObjectError::NamespaceNotFound => IoError::NotFound,
        ObjectError::NameConflict => IoError::Exists,
        ObjectError::InsufficientCapacity => IoError::Capacity,
        ObjectError::Unauthorized => IoError::Denied,
        _ => IoError::Storage,
    }
}

// ------------------------=
// FUNC: normalize
// DESC: Canonicalizes a UTF-8 namespace reference and rejects NUL, overlong paths, and traversal above the root.
// ------------------=
fn normalize(cwd: &[u8], path: &[u8], out: &mut [u8; MAX_PATH]) -> Result<usize, IoError> {
    if path.is_empty() || path.contains(&0) || core::str::from_utf8(path).is_err() { return Err(IoError::Invalid); }
    out[0] = b'/';
    let mut length = 1;
    let base = if path[0] == b'/' { b"".as_slice() } else { cwd };
    for part in base.split(|b| *b == b'/').chain(path.split(|b| *b == b'/')) {
        if part.is_empty() || part == b"." { continue; }
        if part == b".." {
            if length == 1 { return Err(IoError::Denied); }
            length = out[..length].iter().rposition(|b| *b == b'/').unwrap().max(1);
            continue;
        }
        let start = length + usize::from(length > 1);
        let end = start.checked_add(part.len()).ok_or(IoError::Capacity)?;
        if end > MAX_PATH { return Err(IoError::Capacity); }
        if length > 1 { out[length] = b'/'; }
        out[start..end].copy_from_slice(part);
        length = end;
    }
    Ok(length)
}

impl<'a, const N: usize> ObjectIo<'a, N> {
    // ------------------------=
    // FUNC: documents
    // DESC: Provides a statically allocated Documents-only developer session without a large temporary kernel stack allocation.
    // ------------------=
    pub const fn documents() -> ObjectIo<'static, N> {
        let mut cwd = [0; MAX_PATH];
        let mut index = 0;
        while index < DOCUMENTS.len() { cwd[index] = DOCUMENTS[index]; index += 1; }
        ObjectIo { grants: &DOCUMENT_GRANTS, cwd, cwd_len: DOCUMENTS.len(), handles: [None; N],
            buffers: [[0; MAX_CONTENT]; N], next_token: 3 }
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates a stream session with caller-owned explicit grants and no ambient write access.
    // ------------------=
    pub fn new(cwd: &[u8], grants: &'a [Grant<'a>]) -> Result<Self, IoError> {
        let mut canonical = [0; MAX_PATH];
        if cwd.first() != Some(&b'/') { return Err(IoError::Invalid); }
        let length = normalize(b"/", cwd, &mut canonical)?;
        for grant in grants {
            let mut root = [0; MAX_PATH];
            if grant.root.first() != Some(&b'/') { return Err(IoError::Invalid); }
            let size = normalize(b"/", grant.root, &mut root)?;
            if &root[..size] != grant.root { return Err(IoError::Invalid); }
        }
        Ok(Self { grants, cwd: canonical, cwd_len: length, handles: [None; N],
            buffers: [[0; MAX_CONTENT]; N], next_token: 3 })
    }

    // ------------------------=
    // FUNC: authorized
    // DESC: Requires an exact grant root or component-delimited descendant for every namespace operation.
    // ------------------=
    fn authorized(&self, path: &[u8], write: bool) -> bool {
        self.grants.iter().any(|g| (!write || g.write) &&
            (g.root == b"/" || path == g.root ||
             (path.starts_with(g.root) && path.get(g.root.len()) == Some(&b'/'))))
    }

    // ------------------------=
    // FUNC: slot
    // DESC: Resolves a session handle without accepting closed or recycled descriptors.
    // ------------------=
    fn slot(&self, token: i32) -> Result<usize, IoError> {
        self.handles.iter().position(|h| h.is_some_and(|v| v.token == token)).ok_or(IoError::BadHandle)
    }

    // ------------------------=
    // FUNC: open
    // DESC: Opens the saved object version or creates a normal namespace object after validating grants and capacity.
    // ------------------=
    pub fn open<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>, path: &[u8], flags: u32) -> Result<i32, IoError> {
        if flags & !(READ | WRITE | CREATE | TRUNCATE | APPEND | EXCLUSIVE | TEXT) != 0
            || flags & (READ | WRITE) == 0
            || (flags & (CREATE | TRUNCATE | APPEND) != 0 && flags & WRITE == 0)
            || (flags & EXCLUSIVE != 0 && flags & CREATE == 0) { return Err(IoError::Invalid); }
        let slot = self.handles.iter().position(Option::is_none).ok_or(IoError::Capacity)?;
        let next = self.next_token.checked_add(1).ok_or(IoError::Capacity)?;
        let mut canonical = [0; MAX_PATH];
        let length = normalize(&self.cwd[..self.cwd_len], path, &mut canonical)?;
        let path = &canonical[..length];
        if !self.authorized(path, flags & WRITE != 0) { return Err(IoError::Denied); }
        let id = match store.resolve(path) {
            Ok(id) => {
                if flags & EXCLUSIVE != 0 { return Err(IoError::Exists); }
                id
            },
            Err(ObjectError::NamespaceNotFound) if flags & CREATE != 0 => {
                let separator = path.iter().rposition(|b| *b == b'/').ok_or(IoError::Invalid)?;
                let parent = store.resolve(&path[..separator.max(1)]).map_err(storage_error)?;
                let metadata = store.metadata(parent).map_err(storage_error)?;
                if metadata.kind != ObjectType::NamespaceNode { return Err(IoError::Invalid); }
                if !matches!(metadata.space, Space::Personal | Space::Applications) { return Err(IoError::Denied); }
                let kind = if flags & TEXT != 0 { ObjectType::Text } else { ObjectType::Blob };
                store.create_attached(&path[separator + 1..], kind, metadata.space, b"", path).map_err(storage_error)?
            },
            Err(error) => return Err(storage_error(error)),
        };
        let metadata = store.metadata(id).map_err(storage_error)?;
        if metadata.kind == ObjectType::NamespaceNode { return Err(IoError::IsDirectory); }
        if !matches!(metadata.kind, ObjectType::Text | ObjectType::Blob | ObjectType::ApplicationData) { return Err(IoError::Denied); }
        if flags & WRITE != 0 {
            if !matches!(metadata.space, Space::Personal | Space::Applications) { return Err(IoError::Denied); }
            if self.handles.iter().flatten().any(|h| h.id == id && h.flags & WRITE != 0) { return Err(IoError::Conflict); }
        }
        if metadata.logical_size as usize > MAX_CONTENT { return Err(IoError::Capacity); }
        let size = if flags & TRUNCATE != 0 { 0 } else {
            store.read(id, Some(metadata.current_version), &mut self.buffers[slot]).map_err(storage_error)?
        };
        let token = self.next_token;
        self.next_token = next;
        self.handles[slot] = Some(Handle { token, id, version: metadata.current_version, flags,
            position: if flags & APPEND != 0 { size } else { 0 }, length: size, dirty: flags & TRUNCATE != 0 });
        Ok(token)
    }

    // ------------------------=
    // FUNC: read
    // DESC: Reads a bounded range from the opened version and advances its independent cursor.
    // ------------------=
    pub fn read(&mut self, token: i32, out: &mut [u8]) -> Result<usize, IoError> {
        let slot = self.slot(token)?;
        let h = self.handles[slot].as_mut().unwrap();
        if h.flags & READ == 0 { return Err(IoError::Denied); }
        let count = out.len().min(h.length.saturating_sub(h.position));
        if count > 0 { out[..count].copy_from_slice(&self.buffers[slot][h.position..h.position + count]); }
        h.position += count;
        Ok(count)
    }

    // ------------------------=
    // FUNC: write
    // DESC: Buffers an all-or-nothing bounded write, zero-filling seek gaps and honoring append on each call.
    // ------------------=
    pub fn write(&mut self, token: i32, bytes: &[u8]) -> Result<usize, IoError> {
        let slot = self.slot(token)?;
        let h = self.handles[slot].as_mut().unwrap();
        if h.flags & WRITE == 0 { return Err(IoError::Denied); }
        if bytes.is_empty() { return Ok(0); }
        let position = if h.flags & APPEND != 0 { h.length } else { h.position };
        let end = position.checked_add(bytes.len()).filter(|end| *end <= MAX_CONTENT).ok_or(IoError::Capacity)?;
        if position > h.length { self.buffers[slot][h.length..position].fill(0); }
        self.buffers[slot][position..end].copy_from_slice(bytes);
        h.position = end;
        h.length = h.length.max(end);
        h.dirty = true;
        Ok(bytes.len())
    }

    // ------------------------=
    // FUNC: seek
    // DESC: Moves a stream cursor relative to start, current position or end with checked signed arithmetic.
    // ------------------=
    pub fn seek(&mut self, token: i32, offset: i64, origin: u32) -> Result<usize, IoError> {
        let slot = self.slot(token)?;
        let h = self.handles[slot].as_mut().unwrap();
        let base = match origin { 0 => 0, 1 => h.position, 2 => h.length, _ => return Err(IoError::Invalid) };
        let next = (base as i64).checked_add(offset).filter(|p| *p >= 0 && *p <= MAX_CONTENT as i64).ok_or(IoError::Invalid)?;
        h.position = next as usize;
        Ok(h.position)
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Publishes dirty bytes as one version only if the saved object has not changed externally.
    // ------------------=
    pub fn flush<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>, token: i32) -> Result<(), IoError> {
        let slot = self.slot(token)?;
        let h = self.handles[slot].as_mut().unwrap();
        if !h.dirty { return Ok(()); }
        if store.metadata(h.id).map_err(storage_error)?.current_version != h.version { return Err(IoError::Conflict); }
        h.version = store.write(h.id, &self.buffers[slot][..h.length]).map_err(storage_error)?;
        h.dirty = false;
        Ok(())
    }

    // ------------------------=
    // FUNC: close
    // DESC: Flushes and releases the descriptor, reporting a write conflict without overwriting another app's version.
    // ------------------=
    pub fn close<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>, token: i32) -> Result<(), IoError> {
        let slot = self.slot(token)?;
        let result = self.flush(store, token);
        self.handles[slot] = None;
        self.buffers[slot].fill(0);
        result
    }
}
