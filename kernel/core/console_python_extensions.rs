//! Immutable native extension snapshots: mutable package scratch can never move
//! or replace the bytes a running loader uses to resolve its live libraries.
use super::{native_extensions, native_job_files::File,
    native_python_packages::{Vfs, MAX_PATH, limits::EXTENSION_SNAPSHOT_BYTES}};

pub const COUNT: usize = 8;
#[derive(Clone, Copy)]
struct Entry { path: [u8; MAX_PATH], length: usize, offset: usize, size: usize }
const EMPTY: Entry = Entry { path: [0; MAX_PATH], length: 0, offset: 0, size: 0 };
pub struct Snapshots { entries: [Entry; COUNT], bytes: [u8; EXTENSION_SNAPSHOT_BYTES], count: usize, used: usize }

impl Snapshots {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves bounded native package snapshots separately from the mutable guest filesystem.
    // ------------------=
    pub const fn new() -> Self { Self { entries: [EMPTY; COUNT], bytes: [0; EXTENSION_SNAPSHOT_BYTES], count: 0, used: 0 } }

    // ------------------------=
    // FUNC: clear
    // DESC: Erases the prior user's extension snapshots only after its process has stopped.
    // ------------------=
    pub fn clear(&mut self) { self.bytes[..self.used].fill(0); self.entries.fill(EMPTY); self.count = 0; self.used = 0; }

    // ------------------------=
    // FUNC: validate
    // DESC: Validates every staged extension using the actual native loader contract before allowing package publication.
    // ------------------=
    pub fn validate(filesystem: &Vfs, executable: &[u8], machine: u16) -> bool {
        let mut total = 0usize;
        for index in 0..=COUNT {
            let Some((_, bytes)) = filesystem.native_extension_nth(index) else { return true; };
            total = match total.checked_add(bytes.len()) { Some(value) => value, None => return false };
            if total > EXTENSION_SNAPSHOT_BYTES { return false; }
            if index == COUNT || native_extensions::validate_python_extension(executable, bytes, machine).is_err() { return false; }
        }
        false
    }

    // ------------------------=
    // FUNC: capture
    // DESC: Copies only validated extensions from the initiating user's package bundle to stable read-only loader grants.
    // ------------------=
    pub fn capture(&mut self, filesystem: &Vfs, executable: &[u8], machine: u16) -> Result<(), &'static [u8]> {
        self.clear();
        if !Self::validate(filesystem, executable, machine) {
            return Err(b"python: installed native extension is incompatible with this runtime or exceeds its eight-module limit.");
        }
        for index in 0..COUNT {
            let Some((path, bytes)) = filesystem.native_extension_nth(index) else { break; };
            let end = self.used.checked_add(bytes.len()).filter(|end| *end <= self.bytes.len())
                .ok_or(b"python: native extension snapshots exceed the package budget.".as_slice())?;
            let entry = &mut self.entries[index];
            entry.path[..path.len()].copy_from_slice(path); entry.length = path.len();
            entry.offset = self.used; entry.size = bytes.len();
            self.bytes[self.used..end].copy_from_slice(bytes); self.used = end; self.count += 1;
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the bounded number of immutable native module grants.
    // ------------------=
    pub fn count(&self) -> usize { self.count }

    // ------------------------=
    // FUNC: file
    // DESC: Lends the exact unchanged file identity and image retained for this process lifetime.
    // ------------------=
    pub fn file(&self, index: usize) -> Option<File<'_>> {
        if index >= self.count { return None; }
        let entry = &self.entries[index];
        Some(File { path: &entry.path[..entry.length], bytes: &self.bytes[entry.offset..entry.offset + entry.size] })
    }
}
