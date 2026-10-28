//! Packed, immutable, owner-scoped Python invocation inputs. File count and
//! byte budgets are independent: many small applications do not consume one
//! megabyte each merely because that is the per-file ceiling.

pub const BYTES: usize = 8 * 1024 * 1024;
pub const FILE_BYTES: usize = 1024 * 1024;
pub const COUNT: usize = 256;
pub const PATH: usize = 95;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub object: [u8; 16], pub version: u32, pub owner: [u8; 16],
    pub personal: bool, pub length: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub object: [u8; 16], pub version: u32, pub length: usize,
    pub current: Metadata,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Path, Authority, Metadata, Duplicate, FileBudget, ByteBudget, CountBudget, Read, Changed }

#[derive(Clone, Copy)]
struct Entry { path: [u8; PATH], path_len: usize, offset: usize, metadata: Metadata }
const EMPTY: Entry = Entry { path: [0; PATH], path_len: 0, offset: 0,
    metadata: Metadata { object: [0; 16], version: 0, owner: [0; 16], personal: false, length: 0 } };
pub struct Input<'a> {
    pub path: &'a [u8], pub bytes: &'a [u8], pub object: [u8; 16], pub version: u32,
}
pub struct Inputs { entries: [Entry; COUNT], bytes: [u8; BYTES], count: usize, used: usize }

impl Inputs {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves the existing eight-megabyte input budget with independently bounded file descriptors.
    // ------------------=
    pub const fn new() -> Self { Self { entries: [EMPTY; COUNT], bytes: [0; BYTES], count: 0, used: 0 } }

    // ------------------------=
    // FUNC: clear
    // DESC: Erases the prior invocation's committed bytes and identities only after its process has stopped.
    // ------------------=
    pub fn clear(&mut self) {
        self.bytes[..self.used].fill(0); self.entries.fill(EMPTY); self.count = 0; self.used = 0;
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the number of committed immutable input grants.
    // ------------------=
    pub const fn count(&self) -> usize { self.count }

    // ------------------------=
    // FUNC: used
    // DESC: Reports actual committed source bytes rather than reserved per-file slots.
    // ------------------=
    pub const fn used(&self) -> usize { self.used }

    // ------------------------=
    // FUNC: get
    // DESC: Lends a stable path, byte range and pinned object version for the complete invocation lifetime.
    // ------------------=
    pub fn get(&self, index: usize) -> Option<Input<'_>> {
        if index >= self.count { return None; }
        let entry = &self.entries[index];
        Some(Input { path: &entry.path[..entry.path_len],
            bytes: &self.bytes[entry.offset..entry.offset + entry.metadata.length],
            object: entry.metadata.object, version: entry.metadata.version })
    }

    // ------------------------=
    // FUNC: capture
    // DESC: Commits one authorized immutable read only after its identity, current version and exact size revalidate; failures preserve every prior grant.
    // ------------------=
    pub fn capture(&mut self, user: [u8; 16], path: &[u8], metadata: Metadata,
        read: impl FnOnce(&mut [u8]) -> Result<Receipt, ()>) -> Result<(), Error> {
        if path.len() < 2 || path.len() > PATH || path[0] != b'/' || path.contains(&0)
            || path[1..].split(|byte| *byte == b'/').any(|part| part.is_empty() || part == b"." || part == b"..") {
            return Err(Error::Path);
        }
        if user == [0; 16] || metadata.owner != user || !metadata.personal { return Err(Error::Authority); }
        if metadata.object == [0; 16] || metadata.version == 0 { return Err(Error::Metadata); }
        if self.entries[..self.count].iter().any(|entry| &entry.path[..entry.path_len] == path) { return Err(Error::Duplicate); }
        if metadata.length > FILE_BYTES { return Err(Error::FileBudget); }
        if self.count == COUNT { return Err(Error::CountBudget); }
        let end = self.used.checked_add(metadata.length).filter(|end| *end <= BYTES).ok_or(Error::ByteBudget)?;
        let pending = &mut self.bytes[self.used..end];
        let outcome = match read(pending) {
            Ok(receipt) if receipt.object == metadata.object && receipt.version == metadata.version
                && receipt.length == metadata.length && receipt.current == metadata => Ok(()),
            Ok(_) => Err(Error::Changed),
            Err(()) => Err(Error::Read),
        };
        if let Err(error) = outcome { pending.fill(0); return Err(error); }
        let mut entry = EMPTY;
        entry.path[..path.len()].copy_from_slice(path); entry.path_len = path.len();
        entry.offset = self.used; entry.metadata = metadata;
        self.entries[self.count] = entry; self.count += 1; self.used = end;
        Ok(())
    }
}
