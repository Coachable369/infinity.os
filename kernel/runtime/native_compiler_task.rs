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

pub struct Task {
    process: Process<cpu::BoundContext>,
    address_space: ProcessAddressSpace<cpu::Tables>,
    mappings: PrivateMappings,
    files: Files,
    streams: native_stdio::State,
}

impl Task {
    // ------------------------=
    // FUNC: start
    // DESC: Loads a checked native compiler into private memory; rolls back all frames if launch preparation fails.
    // ------------------=
    pub unsafe fn start(bytes: &[u8], args: &[&[u8]], owner: SecurityIdentity, id: u32,
        session: u32, memory: &mut NativeMemory, _kernel: KernelAccessMap,
        controller: cpu::Controller, deadline: u64) -> Result<Self, MemoryError> {
        let image = Image::parse_compiler(bytes, cpu::MACHINE).map_err(|_| MemoryError::InvalidAddress)?;
        let layout = CompilerVirtualLayout { image_base: image.virtual_base as u64,
            heap_base: 0x200000000, heap_pages: 16384, stack_guard_base: 0x300000000, stack_pages: 256 };
        let mut space = ProcessAddressSpace::<cpu::Tables>::build(memory, id, &image, layout, cpu::private_kernel_map())?;
        let prepared = (|| {
            Image::load_compiler(bytes, cpu::MACHINE, space.image_bytes_mut()).map_err(|_| MemoryError::InvalidAddress)?;
            let template = image.tls.ok_or(MemoryError::InvalidAddress)?;
            let storage = core::slice::from_raw_parts_mut((space.heap.physical_address + 4096) as *mut u8, 4096);
            let tls = template.initialize_at(bytes, cpu::MACHINE, storage, (layout.heap_base + 4096) as usize)
                .map_err(|_| MemoryError::InvalidAddress)?.thread_pointer();
            launch::write(&mut space.page_tables, layout.heap_base, launch::Invocation {
                arguments: args, executable: b"/system/compiler", home: b"/", node: b"InfinityOS",
                process: id, session, heap: layout.heap_base + 65536, heap_bytes: 64 * 1024 * 1024 - 65536,
            })?;
            cpu::prepare(&mut space, memory, controller)?;
            Ok::<_, MemoryError>(tls)
        })();
        let tls = match prepared { Ok(tls) => tls, Err(error) => { space.release(memory)?; return Err(error); } };
        let context = cpu::Context::new(space.entry, space.stack_top, layout.heap_base, tls as u64);
        let executor = cpu::BoundContext::new(context, space.page_tables.root_physical_address(), controller);
        Ok(Self { process: Process::new(executor, owner, 1_000_000, Some(deadline)), address_space: space,
            mappings: PrivateMappings::new(id, 256), files: Files::new(), streams: native_stdio::State::default() })
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Prevents any further user execution after owner closure, cancellation or lost capability.
    // ------------------=
    pub fn cancel(&mut self) { self.process.cancel(); }
    // ------------------------=
    // FUNC: poll
    // DESC: Runs at most one preemptible slice and one checked service operation before returning to the desktop.
    // ------------------=
    pub fn poll(&mut self, now: u64, authorized: bool, memory: &mut NativeMemory, input: &[u8],
        files: &[File<'_>], output: &mut [u8], diagnostics: &mut [u8]) -> State {
        if !authorized { self.process.cancel(); }
        let Some(mut streams) = native_stdio::Streams::resume(input, output, diagnostics, self.streams) else {
            self.process.cancel();
            return State::Cancelled;
        };
        let tables = &mut self.address_space.page_tables;
        let mappings = &mut self.mappings;
        let handles = &mut self.files;
        let state = self.process.step(now, |_, operation, a| {
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
        self.process.cancel();
        self.mappings.release_all(&mut self.address_space.page_tables, memory)?;
        self.address_space.release(memory)
    }
}
