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
pub static SYSCALL:&[u8]=include_bytes!("../../sdk/c/include/infinity/syscall.h");
pub static UIKIT:&[u8]=include_bytes!("../../sdk/c/include/infinity/uikit.h");
pub static INTERACTION: &[u8] = include_bytes!("../../sdk/c/include/infinity/interaction.h");

pub struct Task {
    process: Option<Process<cpu::BoundContext>>,
    address_space: Option<ProcessAddressSpace<cpu::Tables>>,
    preparation: Option<ProcessPreparation<cpu::Tables>>,
    image: Image, bytes: &'static [u8], args: &'static [&'static [u8]],
    owner: SecurityIdentity, id: u32, session: u32, controller: cpu::Controller,
    deadline: u64, cache_offset: usize, cancelled: bool,
    mappings: PrivateMappings,
    files: Files,
    streams: native_stdio::State,
}

impl Task {
    // ------------------------=
    // FUNC: start
    // DESC: Reserves a checked compiler job; bulk preparation is deferred to bounded cancellable polls.
    // ------------------=
    pub unsafe fn start(bytes: &'static [u8], args: &'static [&'static [u8]], owner: SecurityIdentity, id: u32,
        session: u32, memory: &mut NativeMemory, _kernel: KernelAccessMap,
        controller: cpu::Controller, deadline: u64) -> Result<Self, MemoryError> {
        let image = Image::parse_compiler(bytes, cpu::MACHINE).map_err(|_| MemoryError::InvalidAddress)?;
        let layout = CompilerVirtualLayout { image_base: image.virtual_base as u64,
            heap_base: 0x200000000, heap_pages: 16384, stack_guard_base: 0x300000000, stack_pages: 256 };
        let preparation = ProcessPreparation::<cpu::Tables>::new(memory, id, image, layout)?;
        Ok(Self { process: None, address_space: None, preparation: Some(preparation), image, bytes, args,
            owner, id, session, controller, deadline, cache_offset: 0, cancelled: false,
            mappings: PrivateMappings::new(id, 256), files: Files::new(), streams: native_stdio::State::default() })
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
        launch::write(&mut space.page_tables, 0x200000000, launch::Invocation {
            arguments: self.args, executable: b"/system/compiler", home: b"/", node: b"InfinityOS",
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
    // FUNC: poll
    // DESC: Runs at most one preemptible slice and one checked service operation before returning to the desktop.
    // ------------------=
    pub fn poll(&mut self, now: u64, authorized: bool, memory: &mut NativeMemory, input: &[u8],
        files: &[File<'_>], output: &mut [u8], diagnostics: &mut [u8]) -> State {
        if !authorized { self.cancel(); }
        if self.cancelled { return State::Cancelled; }
        if now >= self.deadline { return State::Deadline; }
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
        let state = self.process.as_mut().unwrap().step(now, |_, operation, a| {
            match operation {
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
                0x1200 => copy_to_user(tables,a[1],&now.to_le_bytes()).map_or(-14, |_| 0),
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
        self.cancel();
        if let Some(preparation) = self.preparation.take() { preparation.release(memory)?; }
        if let Some(mut space) = self.address_space.take() {
            self.mappings.release_all(&mut space.page_tables, memory)?;
            space.release(memory)?;
        }
        Ok(())
    }
}
