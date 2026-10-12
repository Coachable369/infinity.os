use super::{native_memory::*, native_c_image::Image, cpu::{self, Tables, Controller, Context, Slice, MACHINE},
    emit, finish, IMAGE};
const COMPILE: bool = option_env!("INFINITY_COMPILER_SOURCE").is_some();
#[cfg(target_arch = "aarch64")]
const SOURCE: &[u8] = b"int answer(int x) { return x * 7 + 5; }\nvoid _start(void) { register long status __asm__(\"x0\") = answer(7); register long call __asm__(\"x8\") = 0x1000; __asm__ volatile(\"svc #0\" : : \"r\"(status), \"r\"(call) : \"memory\"); for (;;) {} }\n";
#[cfg(target_arch = "x86_64")]
const SOURCE: &[u8] = b"int answer(int x) { return x * 7 + 5; }\nvoid _start(void) { __asm__ volatile(\"int $0x80\" : : \"a\"(0x1000L), \"D\"((long)answer(7)) : \"memory\"); for (;;) {} }\n";
const LINK: bool = option_env!("INFINITY_LINK_INPUT").is_some();
static LINK_INPUT: &[u8] = include_bytes!(env!("INFINITY_LINK_INPUT_BYTES"));
static LINK_SCRIPT: &[u8] = include_bytes!("../../sdk/c/application.ld");

// ------------------------=
// FUNC: put
// DESC: Writes launch scalars through the checked private-memory copier.
// ------------------=
fn put(process: &mut ProcessAddressSpace<Tables>, address: u64, value: u64) {
    copy_to_user(&mut process.page_tables, address, &value.to_le_bytes()).unwrap();
}

// ------------------------=
// FUNC: fixture_index
// DESC: Resolves only the two explicitly granted read-only linker fixture objects through checked user memory.
// ------------------=
fn fixture_index(process: &ProcessAddressSpace<Tables>, address: u64) -> Option<usize> {
    let mut path = [0u8; 16];
    let mut length = 0;
    while length < path.len() {
        copy_from_user(&process.page_tables, address.checked_add(length as u64)?, &mut path[length..length + 1]).ok()?;
        if path[length] == 0 { break; }
        length += 1;
    }
    match &path[..length] { b"/input.o" => Some(0), b"/app.ld" => Some(1), _ => None }
}

// ------------------------=
// FUNC: fixture_metadata
// DESC: Publishes exact size, identity, object kind and read-only rights for a linker fixture.
// ------------------=
fn fixture_metadata(process: &mut ProcessAddressSpace<Tables>, index: usize, destination: u64) {
    let input = if index == 0 { LINK_INPUT } else { LINK_SCRIPT };
    let mut metadata = [0u8; 40];
    metadata[..8].copy_from_slice(&(3u64 + index as u64).to_le_bytes());
    metadata[8..16].copy_from_slice(&(input.len() as u64).to_le_bytes());
    metadata[28..32].copy_from_slice(&1u32.to_le_bytes());
    metadata[32..36].copy_from_slice(&1u32.to_le_bytes());
    copy_to_user(&mut process.page_tables, destination, &metadata).unwrap();
}

// ------------------------=
// FUNC: execute_linked
// DESC: Loads the guest-linked artifact into a new private EL0 process and checks its real exit value.
// ------------------=
unsafe fn execute_linked(bytes: &[u8], memory: &mut NativeMemory, kernel: KernelAccessMap,
    controller: Controller) -> u64 {
    let image = Image::parse(bytes, MACHINE).unwrap();
    let layout = CompilerVirtualLayout { image_base: 0x100000000, heap_base: 0x200000000,
        heap_pages: 4, stack_guard_base: 0x300000000, stack_pages: 16 };
    let mut child = ProcessAddressSpace::<Tables>::build(memory, 2, &image, layout, kernel).unwrap();
    Image::load_at(bytes, MACHINE, child.image_bytes_mut(), layout.image_base as usize).unwrap();
    cpu::prepare(&mut child, memory, controller).unwrap();
    let context = Context::new(child.entry, child.stack_top, 0, 0);
    let executor = cpu::BoundContext::new(context, child.page_tables.root_physical_address(), controller);
    let mut supervised = super::native_process::Process::new(executor,
        super::execution::SecurityIdentity([2; 16]), 1000, None);
    for _ in 0..1000 {
        let result = match supervised.step(0, |_, _, _| -88) {
            super::native_process::State::Runnable => continue,
            super::native_process::State::Exited(status) => status as u64,
            _ => panic!("unexpected linked-app trap"),
        };
        child.release(memory).unwrap();
        return result;
    }
    panic!("guest-linked app exceeded execution budget")
}

// ------------------------=
// FUNC: run
// DESC: Executes the actual native Clang entrypoint at EL0 with bounded slices and checked console/clock adapters.
// ------------------=
pub unsafe fn run(memory: &mut NativeMemory, kernel: KernelAccessMap,
    controller: Controller) -> ! {
    let initial = memory.available_pages();
    let image = Image::parse_compiler(IMAGE, MACHINE).unwrap();
    let layout = CompilerVirtualLayout { image_base: image.virtual_base as u64,
        heap_base: 0x200000000, heap_pages: 16384, stack_guard_base: 0x300000000, stack_pages: 256 };
    let mut process = ProcessAddressSpace::<Tables>::build(memory, 1, &image, layout, kernel).unwrap();
    Image::load_compiler(IMAGE, MACHINE, process.image_bytes_mut()).unwrap();
    let tls = image.tls.unwrap();
    let tls_storage = core::slice::from_raw_parts_mut((process.heap.physical_address + 4096) as *mut u8, 4096);
    let tp = tls.initialize_at(IMAGE, MACHINE, tls_storage, (layout.heap_base + 4096) as usize).unwrap().thread_pointer();
    let base = layout.heap_base;
    let compile: &[&[u8]] = &[b"clang", b"-cc1", b"-triple", cpu::TRIPLE,
        b"-emit-obj", b"-o", b"-", b"-x", b"c", b"-O1", b"-"];
    let link: &[&[u8]] = &[b"ld.lld", b"-static", b"-pie", b"--no-dynamic-linker", b"--build-id=none",
        b"-z", b"max-page-size=4096", b"--entry=_start", b"-T", b"/app.ld", b"-o", b"-", b"/input.o"];
    let version: &[&[u8]] = &[b"clang", b"--version"];
    super::native_compiler_launch::write(&mut process.page_tables, base,
        super::native_compiler_launch::Invocation {
            arguments: if LINK { link } else if COMPILE { compile } else { version },
            executable: b"/system/clang", home: b"/home/default", node: b"native-probe",
            process: 1, session: 1, heap: base + 65536, heap_bytes: 64 * 1024 * 1024 - 65536,
        }).unwrap();
    cpu::prepare(&mut process, memory, controller).unwrap();
    let mut context = Context::new(process.entry, process.stack_top, base, tp as u64);
    let mut written = 0u64;
    let mut diagnostic = [0u8; 65536];
    let mut output = [0u8; 65536];
    let source: &[u8] = if option_env!("INFINITY_COMPILER_INVALID_SOURCE").is_some() {
        b"int main(void) { return ; this is not C; }\n"
    } else { SOURCE };
    let mut streams = super::native_stdio::Streams::new(source, &mut output, &mut diagnostic);
    let mut calls = 0u64;
    let mut file_positions = [0usize; 2];
    let mut file_opens = [false; 2];
    let mut mappings = super::native_mappings::PrivateMappings::new(1, 256);
    for _ in 0..100000 {
        let (operation, args) = match context.run(process.page_tables.root_physical_address(), controller).unwrap() {
            Slice::Preempted => continue,
            Slice::Call(operation, args) => (operation, args),
            Slice::Fault(reason, address, pc) => {
                for value in [0xfa017u64, reason, address, pc] { emit(value); }
                finish();
            }
        };
        calls += 1;
        let mut result = (-88i64) as u64; // Newlib ENOSYS: unsupported services fail explicitly.
        match operation {
            0x1000 => {
                let status = args[0];
                mappings.release_all(&mut process.page_tables, memory).unwrap();
                process.release(memory).unwrap();
                let app_status = if LINK && status == 0 {
                    execute_linked(streams.output(), memory, kernel, controller)
                } else { u64::MAX };
                for value in [0x434c414e47454c30, status, written, calls, memory.available_pages() as u64, initial as u64] { emit(value); }
                let (diagnostic, truncated) = streams.diagnostics();
                assert!(!truncated);
                emit(diagnostic.len() as u64);
                for chunk in diagnostic.chunks(8) {
                    let mut bytes = [0u8; 8];
                    bytes[..chunk.len()].copy_from_slice(chunk);
                    emit(u64::from_le_bytes(bytes));
                }
                emit(streams.output().len() as u64);
                for chunk in streams.output().chunks(8) {
                    let mut bytes = [0u8; 8];
                    bytes[..chunk.len()].copy_from_slice(chunk);
                    emit(u64::from_le_bytes(bytes));
                }
                emit(app_status);
                finish();
            }
            0x121b | 0x121c | 0x121d => {
                result = streams.syscall(&mut process.page_tables, operation, args) as u64;
                if operation == 0x121c && result == 0 {
                    written += args[2].min(4096);
                }
            }
            0x1201 if matches!(args[2], 1 | 3) && args[3] == 34 && args[4] as i32 == -1 && args[5] == 0 => {
                let mut hint = [0u8; 8];
                copy_from_user(&process.page_tables, args[0], &mut hint).unwrap();
                check_user_range(&process.page_tables, args[0], 8, true).unwrap();
                let address = mappings.map(&mut process.page_tables, memory, args[1], args[2] == 3).unwrap();
                put(&mut process, args[0], address);
                result = 0;
            }
            0x1202 => {
                mappings.unmap(&mut process.page_tables, memory, args[0], args[1]).unwrap();
                result = 0;
            }
            0x120b if LINK => {
                result = (-2i64) as u64;
                let index = fixture_index(&process, args[0]);
                if let Some(index) = index {
                    if args[1] == 1 && !file_opens[index] {
                        put(&mut process, args[2], 3 + index as u64);
                        file_opens[index] = true;
                        file_positions[index] = 0;
                        result = 0;
                    }
                }
            }
            0x120c | 0x1210 if LINK && (3..=4).contains(&args[0]) && file_opens[args[0] as usize - 3] => {
                let index = args[0] as usize - 3;
                let input = if index == 0 { LINK_INPUT } else { LINK_SCRIPT };
                let positional = operation == 0x1210;
                let position = if positional { args[1] as usize } else { file_positions[index] };
                let out = args[if positional { 2 } else { 1 }];
                let requested = args[if positional { 3 } else { 2 }] as usize;
                let count = requested.min(input.len().saturating_sub(position));
                if count > 0 { copy_to_user(&mut process.page_tables, out, &input[position..position + count]).unwrap(); }
                put(&mut process, args[if positional { 4 } else { 3 }], count as u64);
                if !positional { file_positions[index] += count; }
                result = 0;
            }
            0x120e if LINK && (3..=4).contains(&args[0]) && file_opens[args[0] as usize - 3] => {
                let index = args[0] as usize - 3;
                let input = if index == 0 { LINK_INPUT } else { LINK_SCRIPT };
                let base = match args[2] { 0 => 0, 1 => file_positions[index] as i64, 2 => input.len() as i64, _ => -1 };
                if let Some(position) = base.checked_add(args[1] as i64).filter(|value| base >= 0 && *value >= 0) {
                    put(&mut process, args[3], position as u64);
                    file_positions[index] = position as usize;
                    result = 0;
                } else { result = (-22i64) as u64; }
            }
            0x120f if LINK && (3..=4).contains(&args[0]) && file_opens[args[0] as usize - 3] => { file_opens[args[0] as usize - 3] = false; result = 0; }
            0x1211 if LINK && (3..=4).contains(&args[0]) && file_opens[args[0] as usize - 3] => {
                fixture_metadata(&mut process, args[0] as usize - 3, args[1]);
                result = 0;
            }
            0x1212 => {
                result = (-2i64) as u64;
                if LINK {
                    if let Some(index) = fixture_index(&process, args[0]) {
                        fixture_metadata(&mut process, index, args[1]);
                        result = 0;
                    }
                }
            }
            0x1214 => {
                if args[1] >= 1 {
                    copy_to_user(&mut process.page_tables, args[0], b"/").unwrap();
                    put(&mut process, args[2], 1);
                    result = 0;
                } else { result = (-34i64) as u64; }
            }
            0x1200 => {
                if let Some(ns) = cpu::clock_ns() { put(&mut process, args[1], ns); result = 0; }
            }
            _ => {}
        }
        context.result(result);
    }
    panic!("compiler exceeded disposable probe slice budget")
}
