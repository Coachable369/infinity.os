//! Checked compiler file ABI. The scheduler must hold the owning address space
//! stable throughout dispatch; no user address is dereferenced as a kernel pointer.
use super::{
    capability::CapabilityManager,
    execution::SecurityIdentity,
    native_c_io::IoError,
    native_c_service::{Request, Session},
    native_memory::{check_user_range, copy_from_user, copy_to_user, PageTable},
};
use crate::storage::{object::ObjectStore, BlockDevice};

const TRANSFER: usize = 4096;

// ------------------------=
// FUNC: path
// DESC: Reads a bounded NUL-terminated namespace string, including strings that cross user pages.
// ------------------=
fn path<T: PageTable>(memory: &T, address: u64, output: &mut [u8; 96]) -> Result<usize, i64> {
    for index in 0..output.len() {
        let cursor = address.checked_add(index as u64).ok_or(-14i64)?;
        copy_from_user(memory, cursor, &mut output[index..index + 1]).map_err(|_| -14i64)?;
        if output[index] == 0 {
            return Ok(index);
        }
    }
    Err(-91) // Newlib ENAMETOOLONG
}

// ------------------------=
// FUNC: errno
// DESC: Maps storage categories to the native Newlib ABI without exposing implementation diagnostics.
// ------------------=
fn errno(error: IoError) -> i64 {
    -match error {
        IoError::Invalid => 22,
        IoError::Denied => 13,
        IoError::NotFound => 2,
        IoError::Exists => 17,
        IoError::Capacity => 28,
        IoError::BadHandle => 9,
        IoError::Conflict => 16,
        IoError::Storage => 5,
        IoError::IsDirectory => 21,
        IoError::Unsupported => 88,
    }
}

// ------------------------=
// FUNC: handle
// DESC: Rejects forged high bits instead of aliasing an unrelated valid descriptor.
// ------------------=
fn handle(value: u64) -> Result<i32, i64> {
    i32::try_from(value).map_err(|_| -9)
}

impl<'a, const N: usize> Session<'a, N> {
    // ------------------------=
    // FUNC: file_syscall
    // DESC: Decodes native compiler file requests; validates all output pages before any cursor or storage mutation.
    // ------------------=
    pub fn file_syscall<D: BlockDevice, T: PageTable>(
        &mut self,
        caller: SecurityIdentity,
        caps: &CapabilityManager,
        store: &mut ObjectStore<D>,
        memory: &mut T,
        operation: u64,
        args: [u64; 6],
        now: u64,
    ) -> i64 {
        match self.dispatch_file(caller, caps, store, memory, operation, args, now) {
            Ok(()) => 0,
            Err(error) => error,
        }
    }

    // ------------------------=
    // FUNC: dispatch_file
    // DESC: Marshals bounded file and namespace requests through live capability checks and checked user copies.
    // ------------------=
    fn dispatch_file<D: BlockDevice, T: PageTable>(
        &mut self,
        caller: SecurityIdentity,
        caps: &CapabilityManager,
        store: &mut ObjectStore<D>,
        memory: &mut T,
        operation: u64,
        a: [u64; 6],
        now: u64,
    ) -> Result<(), i64> {
        let mut first = [0; 96];
        let mut second = [0; 96];
        let mut bytes = [0; TRANSFER];
        let request;
        let mut destination = None;
        let mut result_address = None;
        let mut count = 0;
        match operation {
            0x120b => {
                let length = path(memory, a[0], &mut first)?;
                let flags = u32::try_from(a[1]).map_err(|_| -22i64)?;
                request = Request::Open {
                    path: &first[..length],
                    flags,
                };
                result_address = Some(a[2]);
            }
            0x120c | 0x1210 => {
                let (out, size, done) = if operation == 0x120c {
                    (a[1], a[2], a[3])
                } else {
                    (a[2], a[3], a[4])
                };
                count = size.min(TRANSFER as u64) as usize;
                destination = Some(out);
                result_address = Some(done);
                request = if operation == 0x120c {
                    Request::Read(handle(a[0])?)
                } else {
                    Request::ReadAt {
                        handle: handle(a[0])?,
                        offset: usize::try_from(a[1]).map_err(|_| -139i64)?,
                    }
                };
            }
            0x120d => {
                count = a[2].min(TRANSFER as u64) as usize;
                copy_from_user(memory, a[1], &mut bytes[..count]).map_err(|_| -14i64)?;
                // Input owns a separate buffer because execute receives a mutable output slice.
                check_user_range(memory, a[3], 8, true).map_err(|_| -14i64)?;
                let result = self
                    .execute(
                        caller,
                        caps,
                        store,
                        Request::Write {
                            handle: handle(a[0])?,
                            bytes: &bytes[..count],
                        },
                        &mut [],
                        now,
                    )
                    .map_err(errno)?;
                return copy_to_user(memory, a[3], &(result as u64).to_le_bytes())
                    .map_err(|_| -14i64);
            }
            0x120e => {
                request = Request::Seek {
                    handle: handle(a[0])?,
                    offset: a[1] as i64,
                    origin: u32::try_from(a[2]).map_err(|_| -22i64)?,
                };
                result_address = Some(a[3]);
            }
            0x120f => request = Request::Close(handle(a[0])?),
            0x1211 | 0x1212 => {
                count = 40;
                destination = Some(a[1]);
                request = if operation == 0x1211 {
                    Request::Inspect(handle(a[0])?)
                } else {
                    let length = path(memory, a[0], &mut first)?;
                    Request::InspectPath(&first[..length])
                };
            }
            0x1213 => {
                request = Request::Truncate {
                    handle: handle(a[0])?,
                    length: usize::try_from(a[1]).map_err(|_| -139i64)?,
                }
            }
            0x1214 | 0x1215 => {
                let (out, size, done) = if operation == 0x1214 {
                    (a[0], a[1], a[2])
                } else {
                    (a[1], a[2], a[3])
                };
                count = size.min(TRANSFER as u64) as usize;
                destination = Some(out);
                result_address = Some(done);
                request = if operation == 0x1214 {
                    Request::CurrentDirectory
                } else {
                    let length = path(memory, a[0], &mut first)?;
                    Request::Resolve(&first[..length])
                };
            }
            0x1216..=0x1218 => {
                let length = path(memory, a[0], &mut first)?;
                request = match operation {
                    0x1216 => Request::ChangeDirectory(&first[..length]),
                    0x1217 => Request::MakeDirectory(&first[..length]),
                    _ => Request::Remove(&first[..length]),
                };
            }
            0x1219 => {
                if a[2] > 1 {
                    return Err(-22);
                }
                let source = path(memory, a[0], &mut first)?;
                let target = path(memory, a[1], &mut second)?;
                request = Request::Link {
                    source: &first[..source],
                    destination: &second[..target],
                    symbolic: a[2] != 0,
                };
            }
            _ => return Err(-88),
        }
        if let Some(address) = result_address {
            check_user_range(memory, address, 8, true).map_err(|_| -14i64)?;
        }
        if let Some(address) = destination {
            check_user_range(memory, address, count, true).map_err(|_| -14i64)?;
        }
        let result = self
            .execute(caller, caps, store, request, &mut bytes[..count], now)
            .map_err(errno)?;
        if let Some(address) = destination {
            copy_to_user(memory, address, &bytes[..result]).map_err(|_| -14i64)?;
        }
        if let Some(address) = result_address {
            copy_to_user(memory, address, &(result as u64).to_le_bytes()).map_err(|_| -14i64)?;
        }
        Ok(())
    }
}
