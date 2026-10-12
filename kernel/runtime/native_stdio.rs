//! Bounded compiler streams. Binary stdout and diagnostics never share storage.
//! The trusted job owns all buffers and keeps the process mapping stable.
use super::native_memory::{PageTable, check_user_range, copy_from_user, copy_to_user};

pub struct Streams<'a> {
    input: &'a [u8],
    position: usize,
    output: &'a mut [u8],
    output_length: usize,
    diagnostics: &'a mut [u8],
    diagnostic_length: usize,
    diagnostic_truncated: bool,
}

impl<'a> Streams<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Binds an immutable source snapshot and separate bounded result buffers to one compiler job.
    // ------------------=
    pub fn new(input: &'a [u8], output: &'a mut [u8], diagnostics: &'a mut [u8]) -> Self {
        Self { input, position: 0, output, output_length: 0, diagnostics,
            diagnostic_length: 0, diagnostic_truncated: false }
    }
    // ------------------------=
    // FUNC: output
    // DESC: Exposes only bytes actually written by the compiler, never unwritten buffer capacity.
    // ------------------=
    pub fn output(&self) -> &[u8] { &self.output[..self.output_length] }
    // ------------------------=
    // FUNC: diagnostics
    // DESC: Returns captured diagnostics and an explicit truncation indication independent of compilation success.
    // ------------------=
    pub fn diagnostics(&self) -> (&[u8], bool) {
        (&self.diagnostics[..self.diagnostic_length], self.diagnostic_truncated)
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Marshals compiler console operations with bounded transfers and validates all user ranges before mutation.
    // ------------------=
    pub fn syscall<T: PageTable>(&mut self, tables: &mut T, operation: u64, a: [u64; 6]) -> i64 {
        self.dispatch(tables, operation, a).map_or_else(|e| e, |_| 0)
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Preserves source position and binary output on invalid pointers or exhausted output capacity.
    // ------------------=
    fn dispatch<T: PageTable>(&mut self, tables: &mut T, op: u64, a: [u64; 6]) -> Result<(), i64> {
        if op == 0x121d {
            if a[0] > 2 { return Err(-9); }
            return copy_to_user(tables, a[1], &0i32.to_le_bytes()).map_err(|_| -14);
        }
        if !matches!(op, 0x121b | 0x121c) { return Err(-88); }
        if (op == 0x121b && a[0] != 0) || (op == 0x121c && !matches!(a[0], 1 | 2)) {
            return Err(-9);
        }
        let count = a[2].min(4096) as usize;
        check_user_range(tables, a[3], 8, true).map_err(|_| -14i64)?;
        if op == 0x121b {
            let count = count.min(self.input.len() - self.position);
            if count != 0 {
                check_user_range(tables, a[1], count, true).map_err(|_| -14i64)?;
                copy_to_user(tables, a[1], &self.input[self.position..self.position + count]).map_err(|_| -14i64)?;
            }
            copy_to_user(tables, a[3], &(count as u64).to_le_bytes()).map_err(|_| -14i64)?;
            self.position += count;
            return Ok(());
        }
        let mut buffer = [0; 4096];
        if count != 0 { copy_from_user(tables, a[1], &mut buffer[..count]).map_err(|_| -14i64)?; }
        if a[0] == 1 {
            if count > self.output.len() - self.output_length { return Err(-28); }
            self.output[self.output_length..self.output_length + count].copy_from_slice(&buffer[..count]);
            self.output_length += count;
        } else {
            let retained = count.min(self.diagnostics.len() - self.diagnostic_length);
            self.diagnostics[self.diagnostic_length..self.diagnostic_length + retained].copy_from_slice(&buffer[..retained]);
            self.diagnostic_length += retained;
            self.diagnostic_truncated |= retained != count;
        }
        copy_to_user(tables, a[3], &(count as u64).to_le_bytes()).map_err(|_| -14i64)
    }
}
