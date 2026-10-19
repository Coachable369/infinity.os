//! Real upstream pip in isolated user processes sharing only serialized private package state.
use super::super::{cpu, emit, finish, execution::SecurityIdentity, native_compiler_task::Task,
    native_memory::{KernelAccessMap, NativeMemory}, native_process::State,
    native_python_packages::{Vfs, MAX_ARCHIVE}, IMAGE};

static mut FILESYSTEM: Vfs = Vfs::new();
static mut ARCHIVE: [u8; MAX_ARCHIVE] = [0; MAX_ARCHIVE];
static mut OUTPUT: [u8; 65536] = [0; 65536];
static mut DIAGNOSTICS: [u8; 65536] = [0; 65536];

const INPUTS: &[(&[u8], &[u8])] = &[
    (b"/work/infinity_test_base-1.0.0-py3-none-any.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_base-1.0.0-py3-none-any.whl"))),
    (b"/work/infinity_test_app-1.0.0-py3-none-any.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_app-1.0.0-py3-none-any.whl"))),
    (b"/work/infinity_test_app-1.1.0-py3-none-any.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_app-1.1.0-py3-none-any.whl"))),
    (b"/work/infinity_test_escape-1.0.0-py3-none-any.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_escape-1.0.0-py3-none-any.whl"))),
    (b"/work/infinity_test_platform-1.0.0-cp313-cp313-win_amd64.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_platform-1.0.0-cp313-cp313-win_amd64.whl"))),
    (b"/work/infinity_test_native-1.0.0-cp313-cp313-infinityos_aarch64.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_native-1.0.0-cp313-cp313-infinityos_aarch64.whl"))),
    (b"/work/infinity_test_native-1.0.0-cp313-cp313-infinityos_x86_64.whl", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/infinity_test_native-1.0.0-cp313-cp313-infinityos_x86_64.whl"))),
    (b"/work/import.py", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/import.py"))),
    (b"/work/metadata.py", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/metadata.py"))),
    (b"/work/absent.py", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/absent.py"))),
    (b"/work/isolated.py", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/isolated.py"))),
    (b"/work/upgraded.py", include_bytes!(concat!(env!("INFINITY_PIP_FIXTURES"), "/upgraded.py"))),
];

// ------------------------=
// FUNC: arguments
// DESC: Supplies real upstream command arguments or independent assertion scripts for each bounded lifecycle step.
// ------------------=
fn arguments(stage: usize) -> &'static [&'static [u8]] {
    if option_env!("INFINITY_PIP_IMPORT_ONLY").is_some() {
        return &[b"/system/python", b"-c", b"import pip._internal.operations.install.wheel; import pip._internal.req.req_uninstall"];
    }
    match stage {
        0 => &[b"/system/python", b"-m", b"pip", b"--version"],
        1 => &[b"/system/python", b"-m", b"pip", b"install", b"--no-index", b"--find-links=/work",
               b"/work/infinity_test_app-1.0.0-py3-none-any.whl", b"--disable-pip-version-check",
               b"--no-compile", b"--progress-bar=off", b"--only-binary=:all:"],
        2 | 6 => &[b"/system/python", b"/work/import.py"],
        3 => &[b"/system/python", b"/work/metadata.py"],
        4 => &[b"/system/python", b"-m", b"pip", b"install", b"--no-index",
               b"/work/infinity_test_platform-1.0.0-cp313-cp313-win_amd64.whl"],
        5 => &[b"/system/python", b"-m", b"pip", b"install", b"--no-index",
               b"/work/infinity_test_escape-1.0.0-py3-none-any.whl"],
        7 => &[b"/system/python", b"/work/isolated.py"],
        8 => &[b"/system/python", b"-m", b"pip", b"install", b"--upgrade", b"--no-index", b"--find-links=/work",
               b"/work/infinity_test_app-1.1.0-py3-none-any.whl", b"--disable-pip-version-check",
               b"--no-compile", b"--progress-bar=off", b"--only-binary=:all:"],
        9 => &[b"/system/python", b"/work/upgraded.py"],
        10 if cfg!(target_arch = "aarch64") => &[b"/system/python", b"-m", b"pip", b"install", b"--no-index",
               b"/work/infinity_test_native-1.0.0-cp313-cp313-infinityos_aarch64.whl"],
        10 => &[b"/system/python", b"-m", b"pip", b"install", b"--no-index",
               b"/work/infinity_test_native-1.0.0-cp313-cp313-infinityos_x86_64.whl"],
        11 => &[b"/system/python", b"-m", b"pip", b"uninstall", b"-y", b"infinity-test-app"],
        12 => &[b"/system/python", b"/work/absent.py"],
        _ => unreachable!(),
    }
}

// ------------------------=
// FUNC: fingerprint
// DESC: Correlates complete committed archive bytes across process teardown without interpreting diagnostic text.
// ------------------=
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(1469598103934665603, |hash, byte| (hash ^ *byte as u64).wrapping_mul(1099511628211))
}

// ------------------------=
// FUNC: emit_bytes
// DESC: Emits already-length-delimited stdout or stderr as padded numeric serial words for diagnostic retention.
// ------------------=
fn emit_bytes(bytes: &[u8]) {
    for chunk in bytes.chunks(8) {
        let mut value = [0; 8]; value[..chunk.len()].copy_from_slice(chunk);
        emit(u64::from_le_bytes(value));
    }
}

// ------------------------=
// FUNC: run
// DESC: Installs, remounts, imports, upgrades, rejects unsafe wheels and uninstalls through isolated native CPython processes.
// ------------------=
pub unsafe fn run(memory: &mut NativeMemory, kernel: KernelAccessMap, controller: cpu::Controller) -> ! {
    let filesystem = &mut *(&raw mut FILESYSTEM);
    let archive = &mut *(&raw mut ARCHIVE);
    let output = &mut *(&raw mut OUTPUT);
    let diagnostics = &mut *(&raw mut DIAGNOSTICS);
    filesystem.reset(false);
    let mut archive_length = filesystem.encode(archive).unwrap();
    let stages = if option_env!("INFINITY_PIP_IMPORT_ONLY").is_some() { 1 } else { 13 };
    for value in [0x5049504c49464531, 1, stages] { emit(value); }
    for stage in 0..stages as usize {
        filesystem.reset(matches!(stage, 1 | 4 | 5 | 8 | 10 | 11));
        if stage != 7 { filesystem.load_archive(&archive[..archive_length]).unwrap(); }
        for (path, bytes) in INPUTS { filesystem.mount_readonly(path, bytes).unwrap(); }
        output.fill(0); diagnostics.fill(0);
        let initial_pages = memory.available_pages();
        let mut task = Task::start(IMAGE, arguments(stage), SecurityIdentity([if stage == 7 { 2 } else { 1 }; 16]),
            stage as u32 + 1, 1, memory, kernel, controller, u64::MAX).unwrap();
        task.set_launch_secret([0x5a; 32]);
        let mut completed = None;
        for visit in 0..1_000_000 {
            let state = task.poll_with_filesystem(cpu::clock_ns().unwrap_or(0), true, memory, b"", &[],
                output, diagnostics, |tables, operation, arguments| filesystem.syscall(tables, operation, arguments));
            if matches!(state, State::Runnable) { continue; }
            completed = Some((state, visit + 1)); break;
        }
        let (state, visits) = completed.expect("native pip exceeded its bounded probe slice count");
        let (stdout, stderr, truncated) = task.result_lengths(b"", output, diagnostics).unwrap();
        let status = match state { State::Exited(code) => code as u64, _ => u64::MAX };
        let reclaimed = task.release(memory).is_ok();
        let mut committed = false;
        // This is the production commit boundary: never publish staged package
        // mutations after a failed, faulted or unauthorized process.
        if status == 0 && reclaimed && filesystem.dirty() && stage != 7 {
            archive_length = filesystem.encode(archive).unwrap(); committed = true;
        }
        for value in [stage as u64, status, visits, initial_pages as u64, memory.available_pages() as u64,
            archive_length as u64, fingerprint(&archive[..archive_length]), committed as u64, stdout as u64, stderr as u64] {
            emit(value);
        }
        emit_bytes(&output[..stdout]); emit_bytes(&diagnostics[..stderr]);
        let expected_exit = if matches!(stage, 4 | 5 | 10) { status > 0 && status < 256 } else { status == 0 };
        if !reclaimed || truncated || !expected_exit { finish(); }
    }
    emit(0x504950434f4d504c);
    finish()
}
