//! One bounded, isolated build child using the same native Task and staged VFS.
use super::{extensions, network};
use crate::runtime::{execution::SecurityIdentity, native_compiler_task::Task,
    native_job_files::File, native_memory::{NativeMemory, PageTable, copy_from_user, copy_to_user},
    native_process::State, native_python_packages::{Client, Vfs}};

pub struct Child {
    task: Option<Task>, generation: u64, completed: bool, exit: i64,
    client: Client, output: [u8; 65536], errors: [u8; 65536], lengths: (usize, usize),
    extensions: extensions::Snapshots,
}
impl Child {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty child slot without allocating guest pages or granting execution.
    // ------------------=
    pub const fn new() -> Self {
        Self { task: None, generation: 0, completed: false, exit: 0,
            client: Client::new(), output: [0; 65536], errors: [0; 65536], lengths: (0, 0),
            extensions: extensions::Snapshots::new() }
    }
    // ------------------------=
    // FUNC: running
    // DESC: Reports whether an actual child Task still owns native process resources.
    // ------------------=
    pub fn running(&self) -> bool { self.task.is_some() }
    // ------------------------=
    // FUNC: cancel
    // DESC: Couples parent cancellation to its exact native child without affecting another process.
    // ------------------=
    pub fn cancel(&mut self) { if let Some(task) = self.task.as_mut() { task.cancel(); } }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases a consumed child's captured output and immutable library snapshots; live children must be polled to reclaim first.
    // ------------------=
    pub fn clear(&mut self) {
        if self.task.is_some() { return; }
        self.completed = false; self.exit = 0; self.output.fill(0); self.errors.fill(0);
        self.lengths = (0, 0); self.extensions.clear();
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Admits only pip-owned real child processes, validates their scoped launch record and marshals lifecycle receipts through checked memory.
    // ------------------=
    pub fn syscall(&mut self, memory: &mut NativeMemory, tables: &mut impl PageTable,
        operation: u64, a: [u64; 6], filesystem: &mut Vfs, owner: SecurityIdentity,
        pip: bool, id: &mut u32, now: u64, deadline: u64) -> Option<i64> {
        if !(0x1280..=0x1283).contains(&operation) { return None; }
        if !pip { return Some(-13); }
        Some(if operation == 0x1280 {
            self.start(memory, tables, a, filesystem, owner, id, now, deadline)
        } else {
            if a[0] != self.generation { return Some(-9); }
            match operation {
                0x1281 if self.task.is_some() => -11,
                0x1281 if self.completed => {
                    let mut receipt = [0; 24]; receipt[..8].copy_from_slice(&self.exit.to_le_bytes());
                    receipt[8..16].copy_from_slice(&(self.lengths.0 as u64).to_le_bytes());
                    receipt[16..].copy_from_slice(&(self.lengths.1 as u64).to_le_bytes());
                    copy_to_user(tables, a[1], &receipt).map_or(-14, |_| 0)
                }
                0x1282 if self.completed => {
                    let bytes = match a[1] { 0 => &self.output[..self.lengths.0],
                        1 => &self.errors[..self.lengths.1], _ => return Some(-22) };
                    if a[2] > bytes.len() as u64 || a[4] > 65536 { return Some(-22); }
                    let count = (bytes.len() - a[2] as usize).min(a[4] as usize);
                    copy_to_user(tables, a[3], &bytes[a[2] as usize..a[2] as usize + count]).map_or(-14, |_| count as i64)
                }
                0x1283 if a[1] == 0 => { self.cancel(); 0 }
                0x1283 if a[1] == 1 && self.completed => { self.clear(); 0 }
                _ => -9,
            }
        })
    }
    // ------------------------=
    // FUNC: start
    // DESC: Starts only the installed CPython executable with an immutable scoped record path and private native memory; the parent remains parked.
    // ------------------=
    fn start(&mut self, memory: &mut NativeMemory, tables: &mut impl PageTable,
        a: [u64; 6], filesystem: &mut Vfs, owner: SecurityIdentity, id: &mut u32, now: u64, deadline: u64) -> i64 {
        if self.task.is_some() || self.completed { return -16; }
        if !(6..=255).contains(&a[1]) { return -22; }
        let mut path = [0; 255];
        if copy_from_user(tables, a[0], &mut path[..a[1] as usize]).is_err() { return -14; }
        let path = &path[..a[1] as usize];
        if !path.starts_with(b"/tmp/") || path.contains(&0)
            || path.split(|b| *b == b'/').any(|part| part == b"..") { return -13; }
        if !filesystem.read(path).is_ok_and(|bytes| !bytes.is_empty() && bytes.len() <= 16384) { return -22; }
        let Some((kernel, controller)) = (unsafe { crate::runtime::native_compile_job::platform() }) else { return -100; };
        let Some(next) = id.checked_add(1) else { return -75; };
        let Some(generation) = self.generation.checked_add(1).filter(|n| *n < (1u64 << 31)) else { return -75; };
        let Some(secret) = (unsafe { crate::runtime::native_python_entropy::next(&owner.0) }) else { return -100; };
        if self.extensions.capture(filesystem, crate::runtime::native_python_package::PYTHON,
            crate::runtime::native_cpu::MACHINE).is_err() { return -8; }
        let mut task = match unsafe { Task::start(crate::runtime::native_python_package::PYTHON,
            &[b"/system/python", b"-m", b"_infinity_child_entry", path], owner, *id, 1,
            memory, kernel, controller, deadline.min(now.saturating_add(120_000_000_000))) } {
            Ok(task) => task, Err(_) => { self.extensions.clear(); return -12; }
        };
        task.set_launch_secret(secret);
        if let Err(error) = filesystem.enter_child(&mut self.client, generation) {
            let _ = task.release(memory); self.extensions.clear(); return error;
        }
        *id = next; self.generation = generation; self.task = Some(task);
        self.output.fill(0); self.errors.fill(0); self.lengths = (0, 0); self.exit = 0;
        generation as i64
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Executes a child slice with the parent's subset of storage/network authority, prohibits recursive children and reclaims all pages before resuming the parent.
    // ------------------=
    pub fn poll(&mut self, now: u64, authorized: bool, memory: &mut NativeMemory,
        filesystem: &mut Vfs, network: &mut network::Network) {
        let Some(task) = self.task.as_mut() else { return; };
        let mut files: [File<'_>; 8] = core::array::from_fn(|_| File { path: b"", bytes: b"" });
        for (i, file) in files.iter_mut().enumerate().take(self.extensions.count()) {
            *file = self.extensions.file(i).unwrap();
        }
        let result = task.poll_with_filesystem(now, authorized, memory, b"", &files[..self.extensions.count()],
            &mut self.output, &mut self.errors, |tables, operation, args| {
                if (0x1280..=0x1283).contains(&operation) { return Some(-95); }
                network.syscall(tables, operation, args).or_else(|| filesystem.syscall(tables, operation, args))
            });
        if result == State::Runnable { return; }
        let lengths = task.result_lengths(b"", &mut self.output, &mut self.errors);
        self.exit = match result { State::Exited(code) => code, State::Cancelled => -15,
            State::Deadline => 124, _ => 125 };
        if let Some((stdout, stderr, truncated)) = lengths {
            self.lengths = (stdout, stderr); if truncated { self.exit = 125; }
        } else { self.exit = 125; }
        if self.task.take().unwrap().release(memory).is_err() { self.exit = 125; }
        network.close(); filesystem.leave_child(&mut self.client);
        self.extensions.clear(); self.completed = true;
    }
}
