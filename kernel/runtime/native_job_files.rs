//! Immutable job inputs. No ambient namespace or host filesystem is exposed.
use super::native_memory::{check_user_range, copy_from_user, copy_to_user, PageTable};

pub struct File<'a> {
    pub path: &'a [u8],
    pub bytes: &'a [u8],
}
#[derive(Clone, Copy)]
struct Handle {
    file: usize,
    position: u64,
    open: bool,
}
pub struct Files {
    handles: [Handle; 16],
}
impl Files {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded handle table with no implicit file authority.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            handles: [Handle {
                file: 0,
                position: 0,
                open: false,
            }; 16],
        }
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Dispatches read-only compiler inputs with checked pointers and failure-atomic positions.
    // ------------------=
    pub fn syscall<T: PageTable>(
        &mut self,
        tables: &mut T,
        files: &[File<'_>],
        op: u64,
        a: [u64; 6],
    ) -> i64 {
        self.dispatch(tables, files, op, a)
            .map_or_else(|e| e, |_| 0)
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Opens exact granted paths and implements native read, seek, inspect and close without granting writes.
    // ------------------=
    fn dispatch<T: PageTable>(
        &mut self,
        t: &mut T,
        files: &[File<'_>],
        op: u64,
        a: [u64; 6],
    ) -> Result<(), i64> {
        if op == 0x120b || op == 0x1212 {
            let mut path = [0; 128];
            let mut length = 0;
            loop {
                if length == path.len() {
                    return Err(-36);
                }
                let address = a[0].checked_add(length as u64).ok_or(-14i64)?;
                copy_from_user(t, address, &mut path[length..length + 1]).map_err(|_| -14i64)?;
                if path[length] == 0 {
                    break;
                }
                length += 1;
            }
            let name = &path[..length];
            if op == 0x1212
                && !name.is_empty()
                && (name == b"/"
                    || files
                        .iter()
                        .any(|f| f.path.starts_with(name) && f.path.get(length) == Some(&b'/')))
            {
                return metadata(t, files.len(), 0, a[1], 2);
            }
            let index = files.iter().position(|f| f.path == name).ok_or(-2i64)?;
            if op == 0x1212 {
                return metadata(t, index, files[index].bytes.len(), a[1], 1);
            }
            if a[1] != 1 {
                return Err(-13);
            }
            let slot = self.handles.iter().position(|h| !h.open).ok_or(-24i64)?;
            copy_to_user(t, a[2], &(slot as u64 + 3).to_le_bytes()).map_err(|_| -14i64)?;
            self.handles[slot] = Handle {
                file: index,
                position: 0,
                open: true,
            };
            return Ok(());
        }
        if !matches!(op, 0x120c | 0x1210 | 0x120e | 0x120f | 0x1211) {
            return Err(-88);
        }
        let slot = a[0].checked_sub(3).filter(|s| *s < 16).ok_or(-9i64)? as usize;
        let h = &mut self.handles[slot];
        if !h.open {
            return Err(-9);
        }
        let file = files.get(h.file).ok_or(-9i64)?;
        match op {
            0x120f => {
                h.open = false;
                Ok(())
            }
            0x1211 => metadata(t, h.file, file.bytes.len(), a[1], 1),
            0x120e => {
                let base = match a[2] {
                    0 => 0,
                    1 => h.position,
                    2 => file.bytes.len() as u64,
                    _ => return Err(-22),
                };
                let position = (base as i128)
                    .checked_add(a[1] as i64 as i128)
                    .filter(|p| *p >= 0 && *p <= i64::MAX as i128)
                    .ok_or(-22i64)? as u64;
                copy_to_user(t, a[3], &position.to_le_bytes()).map_err(|_| -14i64)?;
                h.position = position;
                Ok(())
            }
            _ => {
                let positional = op == 0x1210;
                let position = if positional { a[1] } else { h.position };
                let output = a[if positional { 2 } else { 1 }];
                let requested = a[if positional { 3 } else { 2 }].min(4096) as usize;
                let result = a[if positional { 4 } else { 3 }];
                let count =
                    requested.min((file.bytes.len() as u64).saturating_sub(position) as usize);
                check_user_range(t, result, 8, true).map_err(|_| -14i64)?;
                if count != 0 {
                    copy_to_user(
                        t,
                        output,
                        &file.bytes[position as usize..position as usize + count],
                    )
                    .map_err(|_| -14i64)?;
                }
                copy_to_user(t, result, &(count as u64).to_le_bytes()).map_err(|_| -14i64)?;
                if !positional {
                    h.position += count as u64;
                }
                Ok(())
            }
        }
    }
}
// ------------------------=
// FUNC: metadata
// DESC: Returns exact immutable input size and read-only rights using the native file ABI.
// ------------------=
fn metadata<T: PageTable>(
    tables: &mut T,
    index: usize,
    length: usize,
    destination: u64,
    kind: u32,
) -> Result<(), i64> {
    let mut bytes = [0; 40];
    bytes[..8].copy_from_slice(&(index as u64 + 1).to_le_bytes());
    bytes[8..16].copy_from_slice(&(length as u64).to_le_bytes());
    bytes[28..32].copy_from_slice(&kind.to_le_bytes());
    bytes[32..36].copy_from_slice(&1u32.to_le_bytes());
    copy_to_user(tables, destination, &bytes).map_err(|_| -14)
}
