//! BSP-owned compiler/linker work: one bounded user-mode slice per poll.
use super::{native_cpu as cpu, native_memory::*, native_c_image::Image,
    native_process::{Process, State}, native_mappings::PrivateMappings,
    native_job_files::{File, Files}, native_stdio, native_compiler_launch as launch,
    execution::SecurityIdentity};

pub const COMPILE_ARGS:&[&[u8]]=&[b"clang",b"-cc1",b"-triple",cpu::TRIPLE,b"-emit-obj",b"-o",b"-",b"-x",b"c",b"-O1",
    b"-pic-level",b"2",b"-pic-is-pie",b"-ffreestanding",b"-I",b"/sdk",b"-"];
pub const LINK_ARGS:&[&[u8]]=&[b"ld.lld",b"-static",b"-pie",b"--strip-all",b"--no-dynamic-linker",b"--build-id=none",
    b"-z",b"max-page-size=4096",b"--entry=infinity_app_start",b"-T",b"/app.ld",b"-o",b"-",b"/input.o",b"/crt.o"];
pub static STDIO:&[u8]=include_bytes!("../../sdk/c/include/stdio.h");
pub static STDDEF:&[u8]=include_bytes!("../../sdk/c/compiler-headers/stddef.h");
pub static STRING:&[u8]=include_bytes!("../../sdk/c/include/string.h");
pub static STDLIB:&[u8]=include_bytes!("../../sdk/c/include/stdlib.h");
pub static STDINT:&[u8]=include_bytes!("../../sdk/c/include/stdint.h");
pub static STDBOOL:&[u8]=include_bytes!("../../sdk/c/include/stdbool.h");
pub static STDARG:&[u8]=include_bytes!("../../sdk/c/include/stdarg.h");
pub static LIMITS:&[u8]=include_bytes!("../../sdk/c/include/limits.h");
pub static ERRNO:&[u8]=include_bytes!("../../sdk/c/include/errno.h");
pub static CTYPE:&[u8]=include_bytes!("../../sdk/c/include/ctype.h");
pub static ASSERT:&[u8]=include_bytes!("../../sdk/c/include/assert.h");
pub static CONSOLE:&[u8]=include_bytes!("../../sdk/c/include/infinity/console.h");
pub static CAMERA:&[u8]=include_bytes!("../../sdk/c/include/infinity/camera.h");
pub static SYSCALL:&[u8]=include_bytes!("../../sdk/c/include/infinity/syscall.h");
pub static UIKIT:&[u8]=include_bytes!("../../sdk/c/include/infinity/uikit.h");
pub static INTERACTION: &[u8] = include_bytes!("../../sdk/c/include/infinity/interaction.h");
pub static APP: &[u8] = include_bytes!("../../sdk/c/include/infinity/app.h");
pub static APP_API: &[u8] = include_bytes!("../../sdk/c/include/infinity/app_api.h");

pub struct Task {
    process: Option<Process<cpu::BoundContext>>,
    address_space: Option<ProcessAddressSpace<cpu::Tables>>,
    preparation: Option<ProcessPreparation<cpu::Tables>>,
    image: Image, bytes: &'static [u8], args: super::native_arguments::Arguments,
    owner: SecurityIdentity, id: u32, session: u32, controller: cpu::Controller,
    deadline: u64, sleep_until: u64, cache_offset: usize, cancelled: bool,
    mappings: PrivateMappings,
    files: Files,
    libraries: super::native_extensions::Libraries,
    launch_secret: Option<[u8; 32]>,
    entropy_secret: Option<[u8; 32]>,
    entropy_sequence: u64,
    directories: super::native_job_directories::Directories,
    streams: native_stdio::State,
}

impl Task {
    // ------------------------=
    // FUNC: execution_id
    // DESC: Exposes the actual native process identifier only after at least one user-mode execution visit.
    // ------------------=
    pub fn execution_id(&self) -> Option<u32> {
        self.process.as_ref().filter(|process| process.counters().0 > 0).map(|_| self.id)
    }

    // ------------------------=
    // FUNC: start
    // DESC: Reserves a checked compiler job; bulk preparation is deferred to bounded cancellable polls.
    // ------------------=
    pub unsafe fn start(bytes: &'static [u8], args: &[&[u8]], owner: SecurityIdentity, id: u32,
        session: u32, memory: &mut NativeMemory, _kernel: KernelAccessMap,
        controller: cpu::Controller, deadline: u64) -> Result<Self, MemoryError> {
        let args = super::native_arguments::Arguments::new(args).ok_or(MemoryError::InvalidAddress)?;
        let image = Image::parse_compiler(bytes, cpu::MACHINE).map_err(|_| MemoryError::InvalidAddress)?;
        let layout = CompilerVirtualLayout { image_base: image.virtual_base as u64,
            heap_base: 0x200000000, heap_pages: 16384, stack_guard_base: 0x300000000, stack_pages: 256 };
        let preparation = ProcessPreparation::<cpu::Tables>::new(memory, id, image, layout)?;
        Ok(Self { process: None, address_space: None, preparation: Some(preparation), image, bytes, args,
            owner, id, session, controller, deadline, sleep_until: 0, cache_offset: 0, cancelled: false,
            mappings: PrivateMappings::new(id, 256), files: Files::new(),
            libraries: super::native_extensions::Libraries::new(id), launch_secret: None,
            entropy_secret: None, entropy_sequence: 0,
            directories: super::native_job_directories::Directories::new(), streams: native_stdio::State::default() })
    }
    // ------------------------=
    // FUNC: prepare_next
    // DESC: Performs one bounded load/cache chunk or the final small launch setup before enabling user execution.
    // ------------------=
    unsafe fn prepare_next(&mut self, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        if let Some(preparation) = self.preparation.as_mut() {
            if preparation.poll(memory, self.bytes)? {
                self.address_space = Some(self.preparation.take().unwrap().finish().ok().ok_or(MemoryError::InvalidAddress)?);
            }
            return Ok(());
        }
        let space = self.address_space.as_mut().ok_or(MemoryError::InvalidAddress)?;
        let size = space.image.page_count as usize * 4096;
        if self.cache_offset < size {
            let n = (size-self.cache_offset).min(ProcessPreparation::<cpu::Tables>::CHUNK);
            cpu::clean_instruction_range(space.image.physical_address+self.cache_offset as u64, n);
            self.cache_offset += n;
            return Ok(());
        }
        let template = self.image.tls.ok_or(MemoryError::InvalidAddress)?;
        let storage = core::slice::from_raw_parts_mut((space.heap.physical_address+4096) as *mut u8, 4096);
        let tls = template.initialize_at(self.bytes, cpu::MACHINE, storage, 0x200001000)
            .map_err(|_| MemoryError::InvalidAddress)?.thread_pointer();
        let mut arguments: [&[u8]; 32] = [&[]; 32];
        for (index, argument) in arguments.iter_mut().take(self.args.count()).enumerate() {
            *argument = self.args.get(index).ok_or(MemoryError::InvalidAddress)?;
        }
        launch::write(&mut space.page_tables, 0x200000000, launch::Invocation {
            arguments: &arguments[..self.args.count()],
            executable: if arguments[0].starts_with(b"/") { arguments[0] } else { b"/system/compiler" }, home: b"/", node: b"InfinityOS",
            process: self.id, session: self.session, heap: 0x200010000, heap_bytes: 64*1024*1024-65536,
        })?;
        cpu::finish_preparation(space, memory, self.controller)?;
        let context = cpu::Context::new(space.entry, space.stack_top, 0x200000000, tls as u64);
        let executor = cpu::BoundContext::new(context, space.page_tables.root_physical_address(), self.controller);
        self.process = Some(Process::new(executor, self.owner, 1_000_000, Some(self.deadline)));
        Ok(())
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Prevents any further user execution after owner closure, cancellation or lost capability.
    // ------------------=
    pub fn cancel(&mut self) { self.cancelled = true; if let Some(process) = self.process.as_mut() { process.cancel(); } }
    // ------------------------=
    // FUNC: set_launch_secret
    // DESC: Separates the one-use hash secret from a bounded random stream rooted in validated boot entropy.
    // ------------------=
    pub fn set_launch_secret(&mut self, secret: [u8; 32]) {
        if self.process.is_none() && self.entropy_secret.is_none() {
            let mut launch = [0; 32];
            if hkdf::Hkdf::<sha2::Sha256>::new(Some(b"InfinityOS native Python hash seed v1"), &secret)
                .expand(b"launch", &mut launch).is_ok() {
                self.launch_secret = Some(launch);
                self.entropy_secret = Some(secret);
            }
        }
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Runs at most one preemptible slice and one checked service operation before returning to the desktop.
    // ------------------=
    pub fn poll(&mut self, now: u64, authorized: bool, memory: &mut NativeMemory, input: &[u8],
        files: &[File<'_>], output: &mut [u8], diagnostics: &mut [u8]) -> State {
        self.poll_with_filesystem(now, authorized, memory, input, files, output, diagnostics, |_, _, _| None)
    }
    // ------------------------=
    // FUNC: poll_with_filesystem
    // DESC: Runs the same isolated task with an explicitly granted filesystem adapter; authorization precedes every operation.
    // ------------------=
    pub fn poll_with_filesystem(
        &mut self, now: u64, authorized: bool, memory: &mut NativeMemory,
        input: &[u8], files: &[File<'_>], output: &mut [u8], diagnostics: &mut [u8],
        mut filesystem: impl FnMut(&mut cpu::Tables, u64, [u64; 6]) -> Option<i64>,
    ) -> State {
        if !authorized { self.cancel(); }
        if self.cancelled { return State::Cancelled; }
        if now >= self.deadline { return State::Deadline; }
        if now < self.sleep_until { return State::Runnable; }
        if self.process.is_none() {
            // Amortize firmware HID polling without allowing an unbounded load.
            // The timer is optional; without it, retain one bounded chunk.
            let started = unsafe { cpu::clock_ns() };
            for _ in 0..64 {
                if unsafe { self.prepare_next(memory) }.is_err() {
                    self.cancelled = true;
                    return State::PlatformFailure;
                }
                if self.process.is_some() { break; }
                match (started, unsafe { cpu::clock_ns() }) {
                    (Some(start), Some(end)) if end.saturating_sub(start) < 4_000_000 => {},
                    _ => break,
                }
            }
            return State::Runnable;
        }
        let Some(mut streams) = native_stdio::Streams::resume(input, output, diagnostics, self.streams) else {
            self.cancel();
            return State::Cancelled;
        };
        let tables = &mut self.address_space.as_mut().unwrap().page_tables;
        let mappings = &mut self.mappings;
        let handles = &mut self.files;
        let libraries = &mut self.libraries;
        let executable = self.bytes;
        let secret = &mut self.launch_secret;
        let entropy = &self.entropy_secret;
        let entropy_sequence = &mut self.entropy_sequence;
        let directories = &mut self.directories;
        let sleep_until = &mut self.sleep_until;
        let state = self.process.as_mut().unwrap().step(now, |_, operation, a| {
                // Only filesystem operations may be delegated. A package mount
                // cannot override memory, entropy, extension, or process services.
                if matches!(operation, 0x1206..=0x1208 | 0x120b..=0x121a | 0x1233..=0x1237) {
                    if let Some(result) = filesystem(tables, operation, a) { return result; }
                }
                match operation {
                0x1205 => {
                    if a[1] > 256 { return -22; }
                    if a[1] == 0 { return 0; }
                    let Some(seed) = entropy.as_ref() else { return -13; };
                    let Some(sequence) = entropy_sequence.checked_add(1) else { return -75; };
                    if check_user_range(tables, a[0], a[1] as usize, true).is_err() { return -14; }
                    let mut bytes = [0; 256];
                    let count = a[1] as usize;
                    if hkdf::Hkdf::<sha2::Sha256>::new(Some(b"InfinityOS native Python random stream v1"), seed)
                        .expand(&sequence.to_le_bytes(), &mut bytes[..count]).is_err() { return -5; }
                    let result = copy_to_user(tables, a[0], &bytes[..count]).map_or(-14, |_| 0);
                    bytes.fill(0);
                    if result == 0 { *entropy_sequence = sequence; }
                    result
                }
                0x1206 | 0x1207 | 0x1208 | 0x1215 => directories.syscall(tables, files, operation, a),
                0x1232 => {
                    if a[1] != 32 { return -22; }
                    let Some(bytes) = secret.as_ref() else { return -13; };
                    if copy_to_user(tables, a[0], bytes).is_err() { return -14; }
                    if let Some(bytes) = secret.as_mut() { bytes.fill(0); }
                    *secret = None; 0
                }
                0x1230 | 0x1231 => libraries.syscall(memory, tables, executable, files,
                    cpu::MACHINE, operation, a, |address, size| unsafe { cpu::clean_instruction_range(address, size) }),
                0x121b | 0x121c | 0x121d => streams.syscall(tables, operation, a),
                0x120b | 0x120c | 0x1210 | 0x120e | 0x120f | 0x1211 | 0x1212 => handles.syscall(tables, files, operation, a),
                0x1201 if matches!(a[2],1|3) && a[3] == 34 && a[4] as i32 == -1 && a[5] == 0 => {
                    if check_user_range(tables, a[0], 8, true).is_err() { return -14; }
                    match mappings.map(tables, memory, a[1], a[2] == 3) {
                        Ok(address) => copy_to_user(tables,a[0],&address.to_le_bytes()).map_or(-14, |_| 0),
                        Err(_) => -12,
                    }
                }
                0x1202 => mappings.unmap(tables, memory, a[0], a[1]).map_or(-22, |_| 0),
                0x1200 => {
                    let value = match super::system_clock::SYSTEM.native_nanoseconds(a[0], now) {
                        Ok(value) => value,
                        Err(error) => return error,
                    };
                    copy_to_user(tables, a[1], &value.to_le_bytes()).map_or(-14, |_| 0)
                },
                0x1209 => {
                    let Some(wake) = now.checked_add(a[0]) else { return -75; };
                    if copy_to_user(tables, a[1], &0u64.to_le_bytes()).is_err() { return -14; }
                    *sleep_until = wake;
                    0
                }
                0x1214 => {
                    if a[1] < 1 { return -34; }
                    if check_user_range(tables,a[2],8,true).is_err() { return -14; }
                    if copy_to_user(tables,a[0],b"/").is_err() { return -14; }
                    copy_to_user(tables,a[2],&1u64.to_le_bytes()).map_or(-14, |_| 0)
                }
                _ => -88,
            }
        });
        self.streams = streams.checkpoint();
        state
    }
    // ------------------------=
    // FUNC: result_lengths
    // DESC: Reports actual binary and diagnostic lengths without exposing unused capacity.
    // ------------------=
    pub fn result_lengths(&self, input: &[u8], output: &mut [u8], diagnostics: &mut [u8]) -> Option<(usize, usize, bool)> {
        let streams = native_stdio::Streams::resume(input, output, diagnostics, self.streams)?;
        let (diagnostic, truncated) = streams.diagnostics();
        Some((streams.output().len(), diagnostic.len(), truncated))
    }
    // ------------------------=
    // FUNC: release
    // DESC: Reclaims dynamic mappings and the entire private address space only after execution has been stopped.
    // ------------------=
    pub fn release(mut self, memory: &mut NativeMemory) -> Result<(), MemoryError> {
        if let Some(secret) = self.launch_secret.as_mut() { secret.fill(0); }
        self.launch_secret = None;
        if let Some(secret) = self.entropy_secret.as_mut() { secret.fill(0); }
        self.entropy_secret = None;
        self.cancel();
        if let Some(preparation) = self.preparation.take() { preparation.release(memory)?; }
        if let Some(mut space) = self.address_space.take() {
            self.libraries.release_all(&mut space.page_tables, memory)?;
            self.mappings.release_all(&mut space.page_tables, memory)?;
            space.release(memory)?;
        }
        Ok(())
    }
}
