//! Read-only directory projections over immutable native-process file grants.
use super::native_job_files::File;
use super::native_memory::{PageTable, check_user_range, copy_from_user, copy_to_user};

#[derive(Clone, Copy)]
struct Cursor { path: [u8; 128], length: usize, next: usize, open: bool }
pub struct Directories { cursors: [Cursor; 64], used: usize }

// ------------------------=
// FUNC: child
// DESC: Projects one direct child name without exposing a namespace outside the grant set.
// ------------------=
fn child<'a>(parent: &[u8], path: &'a [u8]) -> Option<&'a [u8]> {
    let rest = if parent == b"/" { path.strip_prefix(b"/".as_slice())? }
        else { path.strip_prefix(parent)?.strip_prefix(b"/".as_slice())? };
    let length = rest.iter().position(|b| *b == b'/').unwrap_or(rest.len());
    if length == 0 { None } else { Some(&rest[..length]) }
}

// ------------------------=
// FUNC: path
// DESC: Copies a bounded NUL-terminated user path; only explicit absolute grants and the root alias are accepted.
// ------------------=
fn path<T: PageTable>(tables: &T, address: u64, out: &mut [u8; 128]) -> Result<usize, i64> {
    for index in 0..out.len() {
        let pointer = address.checked_add(index as u64).ok_or(-14i64)?;
        copy_from_user(tables, pointer, &mut out[index..index + 1]).map_err(|_| -14i64)?;
        if out[index] == 0 {
            if index == 1 && out[0] == b'.' { out[0] = b'/'; return Ok(1); }
            if index == 0 || out[0] != b'/' { return Err(-2); }
            let mut length = index;
            while length > 1 && out[length - 1] == b'/' { length -= 1; }
            return Ok(length);
        }
    }
    Err(-36)
}

impl Directories {
    // ------------------------=
    // FUNC: new
    // DESC: Creates non-recycled directory handles scoped to one native process.
    // ------------------=
    pub const fn new() -> Self {
        Self { cursors: [Cursor { path: [0; 128], length: 0, next: 0, open: false }; 64], used: 0 }
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Implements bounded directory enumeration and canonical grant lookup with failure-atomic cursors.
    // ------------------=
    pub fn syscall<T: PageTable>(&mut self, tables: &mut T, files: &[File<'_>], operation: u64, a: [u64; 6]) -> i64 {
        self.dispatch(tables, files, operation, a).map_or_else(|error| error, |_| 0)
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Validates complete outputs before advancing directory state or returning a resolved path.
    // ------------------=
    fn dispatch<T: PageTable>(&mut self, tables: &mut T, files: &[File<'_>], operation: u64, a: [u64; 6]) -> Result<(), i64> {
        if operation == 0x1206 || operation == 0x1215 {
            let mut name = [0; 128]; let length = path(tables, a[0], &mut name)?;
            let directory = &name[..length] == b"/" || files.iter().any(|file| child(&name[..length], file.path).is_some());
            if operation == 0x1215 {
                if !directory && !files.iter().any(|file| file.path == &name[..length]) { return Err(-2); }
                if a[2] < length as u64 { return Err(-34); }
                check_user_range(tables, a[1], length, true).map_err(|_| -14i64)?;
                check_user_range(tables, a[3], 8, true).map_err(|_| -14i64)?;
                copy_to_user(tables, a[1], &name[..length]).map_err(|_| -14i64)?;
                return copy_to_user(tables, a[3], &(length as u64).to_le_bytes()).map_err(|_| -14);
            }
            if !directory { return Err(-20); }
            if self.used == self.cursors.len() { return Err(-24); }
            copy_to_user(tables, a[1], &(self.used as u64 + 1).to_le_bytes()).map_err(|_| -14i64)?;
            self.cursors[self.used] = Cursor { path: name, length, next: 0, open: true };
            self.used += 1; return Ok(());
        }
        let index = a[0].checked_sub(1).filter(|i| *i < self.used as u64).ok_or(-9i64)? as usize;
        let cursor = &mut self.cursors[index];
        if !cursor.open { return Err(-9); }
        if operation == 0x1208 { cursor.open = false; return Ok(()); }
        if operation != 0x1207 { return Err(-88); }
        let mut position = cursor.next;
        let mut output = [0; 128]; let mut length = 0;
        while position < files.len() {
            let current = position; position += 1;
            let Some(name) = child(&cursor.path[..cursor.length], files[current].path) else { continue; };
            if files[..current].iter().any(|file| child(&cursor.path[..cursor.length], file.path) == Some(name)) { continue; }
            if name.len() >= output.len() { return Err(-36); }
            output[..name.len()].copy_from_slice(name); length = name.len(); break;
        }
        if a[2] < (length + 1) as u64 { return Err(-34); }
        copy_to_user(tables, a[1], &output[..length + 1]).map_err(|_| -14i64)?;
        cursor.next = position;
        Ok(())
    }
}
