//! One bounded native build job owned by an authenticated desktop session.
use super::{
    execution::SecurityIdentity,
    native_compiler_package as package,
    native_compiler_task::Task,
    native_cpu as cpu,
    native_job_files::File,
    native_memory::{memory_mut, KernelAccessMap},
    native_process::State,
};
pub const SOURCE_LIMIT: usize = 16 * 1024;
const BINARY_LIMIT: usize = 256 * 1024;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Idle,
    Compile,
    Link,
    Complete,
    Failed,
    Cancelled,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    Busy,
    SourceTooLarge,
    Unavailable,
}
#[derive(Clone, Copy)]
struct Platform {
    kernel: KernelAccessMap,
    controller: cpu::Controller,
}
struct Job {
    owner: SecurityIdentity,
    status: Status,
    task: Option<Task>,
    source: [u8; SOURCE_LIMIT],
    source_length: usize,
    object: [u8; BINARY_LIMIT],
    object_length: usize,
    binary: [u8; BINARY_LIMIT],
    binary_length: usize,
    diagnostics: [u8; 8192],
    diagnostic_length: usize,
    truncated: bool,
    deadline: u64,
}
static mut PLATFORM: Option<Platform> = None;
#[no_mangle]
pub static mut INFINITY_NATIVE_BUILD_SNAPSHOT: [u64; 10] =
    [0x494e46434c414e47, 1, 0, 0, 0, 0, 0, 0, 0, 0];
static mut JOB: Job = Job {
    owner: SecurityIdentity([0; 16]),
    status: Status::Idle,
    task: None,
    source: [0; SOURCE_LIMIT],
    source_length: 0,
    object: [0; BINARY_LIMIT],
    object_length: 0,
    binary: [0; BINARY_LIMIT],
    binary_length: 0,
    diagnostics: [0; 8192],
    diagnostic_length: 0,
    truncated: false,
    deadline: 0,
};

// ------------------------=
// FUNC: initialize
// DESC: Records only validated boot mappings and the firmware-discovered interrupt controller on the BSP.
// ------------------=
pub unsafe fn initialize(
    kernel: KernelAccessMap,
    kind: u32,
    reserved: u32,
    base: u64,
    auxiliary: u64,
) {
    PLATFORM = cpu::controller_from_handoff(kind, reserved, base, auxiliary)
        .map(|controller| Platform { kernel, controller });
}
// ------------------------=
// FUNC: platform
// DESC: Shares the boot-validated native adapter configuration with the isolated executable launcher.
// ------------------=
pub unsafe fn platform() -> Option<(KernelAccessMap, cpu::Controller)> {
    PLATFORM.map(|p| (p.kernel, p.controller))
}
// ------------------------=
// FUNC: start
// DESC: Snapshots approved editor source without executing it or blocking the input handler on compiler loading.
// ------------------=
pub unsafe fn start(owner: SecurityIdentity, source: &[u8], now: u64) -> Result<(), Error> {
    let job = &mut *(&raw mut JOB);
    if matches!(job.status, Status::Compile | Status::Link) || job.task.is_some() {
        return Err(Error::Busy);
    }
    if core::ptr::addr_of!(PLATFORM).read().is_none() {
        return Err(Error::Unavailable);
    }
    if source.is_empty() || source.len() > SOURCE_LIMIT {
        return Err(Error::SourceTooLarge);
    }
    job.source[..source.len()].copy_from_slice(source);
    job.source_length = source.len();
    job.owner = owner;
    job.object_length = 0;
    job.binary_length = 0;
    job.diagnostic_length = 0;
    job.truncated = false;
    job.deadline = now.saturating_add(120_000_000_000);
    job.status = Status::Compile;
    publish();
    Ok(())
}
// ------------------------=
// FUNC: status
// DESC: Exposes build state only to the owning session.
// ------------------=
pub unsafe fn status(owner: SecurityIdentity) -> Status {
    let job = &*(&raw const JOB);
    if job.owner == owner {
        job.status
    } else {
        Status::Idle
    }
}
// ------------------------=
// FUNC: cancel
// DESC: Cancels only the owning session's active build and immediately reclaims its private compiler mappings.
// ------------------=
pub unsafe fn cancel(owner: SecurityIdentity) -> bool {
    let job = &mut *(&raw mut JOB);
    if job.owner != owner || !matches!(job.status, Status::Compile | Status::Link) {
        return false;
    }
    if let Some(task) = job.task.take() {
        let _ = task.release(memory_mut());
    }
    job.status = Status::Cancelled;
    publish();
    true
}
// ------------------------=
// FUNC: poll
// DESC: Advances one compiler slice and reports owner-bound terminal jobs until the caller consumes its lease, including cancellation between polls.
// ------------------=
pub unsafe fn poll(owner: SecurityIdentity, authorized: bool, now: u64) -> bool {
    let current=&*(&raw const JOB);
    if current.owner==owner && matches!(current.status,Status::Complete|Status::Failed|Status::Cancelled) {
        return true;
    }
    let before = (&*(&raw const JOB)).status;
    let changed = advance(owner, authorized, now);
    if before != (&*(&raw const JOB)).status {
        publish();
    }
    changed
}
// ------------------------=
// FUNC: publish
// DESC: Publishes bounded read-only structured acceptance evidence on actual build state transitions, never per frame.
// ------------------=
unsafe fn publish() {
    let job = &*(&raw const JOB);
    let state = &mut *(&raw mut INFINITY_NATIVE_BUILD_SNAPSHOT);
    state[2] = state[2].wrapping_add(1) | 1;
    state[3] = match job.status {
        Status::Idle => 0,
        Status::Compile => 1,
        Status::Link => 2,
        Status::Complete => 3,
        Status::Failed => 4,
        Status::Cancelled => 5,
    };
    state[4] = job.source_length as u64;
    state[5] = job.object_length as u64;
    state[6] = job.binary_length as u64;
    state[7] = job.diagnostic_length as u64;
    state[8] = job.truncated as u64;
    state[2] = state[2].wrapping_add(1);
    state[9] = state[2];
}
// ------------------------=
// FUNC: advance
// DESC: Owns the compile/link transition while the public poll records only committed state changes.
// ------------------=
unsafe fn advance(owner: SecurityIdentity, authorized: bool, now: u64) -> bool {
    let job = &mut *(&raw mut JOB);
    if !matches!(job.status, Status::Compile | Status::Link) {
        return false;
    }
    let memory = memory_mut();
    if job.owner != owner || !authorized || now >= job.deadline {
        if let Some(task) = job.task.take() {
            let _ = task.release(memory);
        }
        job.status = Status::Cancelled;
        return true;
    }
    let Some(platform) = PLATFORM else {
        job.status = Status::Failed;
        return true;
    };
    let linking = job.status == Status::Link;
    if job.task.is_none() {
        let compile = super::native_compiler_task::COMPILE_ARGS;
        let link = super::native_compiler_task::LINK_ARGS;
        match Task::start(
            if linking {
                package::LLD
            } else {
                package::CLANG
            },
            if linking { link } else { compile },
            job.owner,
            1,
            1,
            memory,
            platform.kernel,
            platform.controller,
            job.deadline,
        ) {
            Ok(task) => job.task = Some(task),
            Err(_) => {
                job.status = Status::Failed;
                return true;
            }
        }
        return false;
    }
    let files = [
        File {
            path: b"/input.o",
            bytes: &job.object[..job.object_length],
        },
        File {
            path: b"/app.ld",
            bytes: package::LINK_SCRIPT,
        },
        File {
            path: b"/crt.o",
            bytes: package::CRT,
        },
        File {
            path: b"/sdk/stdio.h",
            bytes: super::native_compiler_task::STDIO,
        },
        File {
            path: b"/sdk/stddef.h",
            bytes: super::native_compiler_task::STDDEF,
        },
        File {
            path: b"/sdk/string.h",
            bytes: super::native_compiler_task::STRING,
        },
        File {
            path: b"/sdk/stdlib.h",
            bytes: super::native_compiler_task::STDLIB,
        },
        File {
            path: b"/sdk/stdint.h",
            bytes: super::native_compiler_task::STDINT,
        },
        File {
            path: b"/sdk/stdbool.h",
            bytes: super::native_compiler_task::STDBOOL,
        },
        File {
            path: b"/sdk/stdarg.h",
            bytes: super::native_compiler_task::STDARG,
        },
        File {
            path: b"/sdk/limits.h",
            bytes: super::native_compiler_task::LIMITS,
        },
        File {
            path: b"/sdk/errno.h",
            bytes: super::native_compiler_task::ERRNO,
        },
        File {
            path: b"/sdk/ctype.h",
            bytes: super::native_compiler_task::CTYPE,
        },
        File {
            path: b"/sdk/assert.h",
            bytes: super::native_compiler_task::ASSERT,
        },
        File {
            path: b"/sdk/infinity/console.h",
            bytes: super::native_compiler_task::CONSOLE,
        },
        File {
            path: b"/sdk/infinity/camera.h",
            bytes: super::native_compiler_task::CAMERA,
        },
        File {
            path: b"/sdk/infinity/syscall.h",
            bytes: super::native_compiler_task::SYSCALL,
        },
        File {
            path: b"/sdk/infinity/font.h",
            bytes: super::native_compiler_task::FONT,
        },
        File {
            path: b"/sdk/infinity/uikit.h",
            bytes: super::native_compiler_task::UIKIT,
        },
        File {
            path: b"/sdk/infinity/interaction.h",
            bytes: super::native_compiler_task::INTERACTION,
        },
        File {
            path: b"/sdk/infinity/app.h",
            bytes: super::native_compiler_task::APP,
        },
        File {
            path: b"/sdk/infinity/app_api.h",
            bytes: super::native_compiler_task::APP_API,
        },
    ];
    let input = &job.source[..job.source_length];
    let task = job.task.as_mut().unwrap();
    // Compiler output goes into the binary scratch buffer. On success it becomes
    // an immutable linker input; no per-slice allocation or source copy occurs.
    let state = task.poll(
        now,
        true,
        memory,
        input,
        &files,
        &mut job.binary,
        &mut job.diagnostics,
    );
    if state == State::Runnable {
        return false;
    }
    let (length, errors, truncated) = task
        .result_lengths(input, &mut job.binary, &mut job.diagnostics)
        .unwrap_or((0, 0, true));
    job.diagnostic_length = errors;
    job.truncated = truncated;
    let released = job.task.take().unwrap().release(memory).is_ok();
    if state != State::Exited(0) || !released || length == 0 {
        job.status = Status::Failed;
        return true;
    }
    if linking {
        if super::native_c_image::Image::parse(&job.binary[..length], cpu::MACHINE).is_err() {
            job.status = Status::Failed;
        } else {
            job.binary_length = length;
            job.status = Status::Complete;
        }
        true
    } else {
        job.object[..length].copy_from_slice(&job.binary[..length]);
        job.object_length = length;
        job.status = Status::Link;
        false
    }
}
// ------------------------=
// FUNC: result
// DESC: Borrows only completed output and actual diagnostics for the owning session; callers cannot mutate job storage.
// ------------------=
pub unsafe fn result<R>(
    owner: SecurityIdentity,
    read: impl FnOnce(Status, &[u8], &[u8], bool) -> R,
) -> Option<R> {
    let job = &*(&raw const JOB);
    if job.owner != owner || matches!(job.status, Status::Idle | Status::Compile | Status::Link) {
        return None;
    }
    Some(read(
        job.status,
        &job.binary[..job.binary_length],
        &job.diagnostics[..job.diagnostic_length],
        job.truncated,
    ))
}

// ------------------------=
// FUNC: stage_diagnostics
// DESC: Borrows diagnostics from one exact in-progress stage transition so the command window can report compiler output before linking replaces it.
// ------------------=
pub unsafe fn stage_diagnostics<R>(
    owner: SecurityIdentity,
    expected: Status,
    read: impl FnOnce(&[u8], bool) -> R,
) -> Option<R> {
    let job = &*(&raw const JOB);
    if job.owner != owner || job.status != expected {
        return None;
    }
    Some(read(
        &job.diagnostics[..job.diagnostic_length],
        job.truncated,
    ))
}
