//! Native Python command execution. Object snapshots are private to one authenticated session.
use super::ConsoleRuntime;
use crate::runtime::{execution::SecurityIdentity, native_compiler_task::Task,
    native_job_files::File, native_memory::memory_mut, native_process::State,
    native_python_packages::{Vfs, MAX_ARCHIVE, admit_pip, PipMode, PolicyError}};
#[path = "../runtime/native_python_workflow.rs"]
mod workflow;
use crate::runtime::ai::intelligence::software::Event;

const LIMIT: usize = 1024 * 1024;
struct Snapshot { path: [u8; 95], path_len: usize, bytes: [u8; LIMIT], len: usize }
impl Snapshot {
    const EMPTY: Self = Self { path: [0; 95], path_len: 0, bytes: [0; LIMIT], len: 0 };
}
struct Job {
    task: Option<Task>, owner: SecurityIdentity,
    files: [Snapshot; 8], count: usize,
    output: [u8; 32768], errors: [u8; 8192],
    initial_pages: u64,
    generated: Option<Generated>,
    filesystem: Vfs,
    archive: [u8; MAX_ARCHIVE],
    package_revision: Option<crate::storage::object::PythonPackageRevision>,
    package_owner: [u8; 16],
    pip: bool,
}
#[derive(Clone, Copy)]
struct Generated {
    goal: Option<u64>, user: [u8; 16], source: [u8; 16], revision: u64,
    request: u64, phase: workflow::Phase, launch: bool,
}
static mut JOB: Job = Job { task: None, owner: SecurityIdentity([0; 16]),
    files: [const { Snapshot::EMPTY }; 8], count: 0, output: [0; 32768], errors: [0; 8192], initial_pages: 0, generated: None,
    filesystem: Vfs::new(), archive: [0; MAX_ARCHIVE], package_revision: None, package_owner: [0; 16], pip: false };
static mut NEXT_TASK_ID: u32 = 0x50590001;

#[no_mangle]
pub static mut INFINITY_PYTHON_SNAPSHOT: [u64; 10] = [0x494e46505954484e, 1, 0, 0, 0, 0, 0, 0, 0, 0];

// ------------------------=
// FUNC: publish
// DESC: Publishes numeric execution and reclamation evidence without exposing source bytes or launch secrets.
// ------------------=
unsafe fn publish(state: u64, exit: i64, output: usize, errors: usize, before: u64, after: u64) {
    let snapshot = &raw mut INFINITY_PYTHON_SNAPSHOT;
    let sequence = (*snapshot)[2].wrapping_add(2) & !1;
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*snapshot)[2]), sequence | 1);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    for (index, value) in [state, exit as u64, output as u64, errors as u64, before, after].iter().enumerate() {
        core::ptr::write_volatile(core::ptr::addr_of_mut!((*snapshot)[index + 3]), *value);
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*snapshot)[9]), sequence);
    core::ptr::write_volatile(core::ptr::addr_of_mut!((*snapshot)[2]), sequence);
}

// ------------------------=
// FUNC: active
// DESC: Requires the command's authenticated session to remain active for every execution slice.
// ------------------=
fn active(console: &ConsoleRuntime) -> bool {
    crate::runtime::with_runtime(|runtime| (0..crate::runtime::identity::MAX_SESSIONS)
        .filter_map(|i| runtime.identity.session_nth(i)).any(|s| s.id == console.current_session
            && s.user == console.current_user && s.state == crate::runtime::identity::SessionState::Active)).unwrap_or(false)
}

// ------------------------=
// FUNC: snapshot
// DESC: Reads an immutable version only after checking native Documents read authority and bounded namespace geometry.
// ------------------=
fn snapshot(target: &mut Snapshot, path: &[u8], user: [u8; 16]) -> Result<(), &'static [u8]> {
    if path.is_empty() || path[0] != b'/' || path.len() > target.path.len() {
        return Err(b"python: use an absolute Object Store path of at most 95 bytes.");
    }
    let (metadata, _) = crate::storage::object_inspect_path(path)
        .map_err(|_| b"python: object not found.".as_slice())?;
    if !crate::runtime::native_python_scope::readable(metadata.owner.0, user,
        metadata.space == crate::storage::object::Space::Personal, path) {
        return Err(b"python: object is not readable by this user.");
    }
    let (id, length) = crate::storage::object_read_path(path, Some(metadata.current_version), &mut target.bytes)
        .map_err(|_| b"python: cannot snapshot object (maximum 1 MiB per input).".as_slice())?;
    if id != metadata.object.id { return Err(b"python: object changed while opening."); }
    target.path[..path.len()].copy_from_slice(path);
    target.path_len = path.len(); target.len = length;
    Ok(())
}

// ------------------------=
// FUNC: prepare_packages
// DESC: Starts each invocation from the current user's committed archive with no inherited staged writes or previous-user files.
// ------------------=
fn prepare_packages(console: &ConsoleRuntime, job: &mut Job, writable: bool) -> Result<(), &'static [u8]> {
    job.filesystem.reset(writable); job.archive.fill(0); job.count = 0; job.generated = None;
    job.pip = writable; job.package_owner = console.current_user.0;
    let (revision, length) = crate::storage::python_packages_load(console.current_user.0,
        console.current_session.0, &mut job.archive)
        .map_err(|_| b"python: cannot load this user's installed packages.".as_slice())?;
    job.package_revision = revision;
    if length > 0 { job.filesystem.load_archive(&job.archive[..length])
        .map_err(|_| b"python: installed package archive is invalid; no code was executed.".as_slice())?; }
    job.archive.fill(0); Ok(())
}

// ------------------------=
// FUNC: mount_wheel
// DESC: Snapshots an authorized wheel into the read-only offline wheelhouse without exposing its native directory to pip.
// ------------------=
fn mount_wheel(console: &ConsoleRuntime, job: &mut Job, path: &[u8]) -> Result<(), &'static [u8]> {
    if !path.starts_with(b"/") || path.len() > 95 || !path.ends_with(b".whl") {
        return Err(b"pip: supply an absolute ObjectStore path to a .whl file.");
    }
    let name = path.rsplit(|b| *b == b'/').next().unwrap_or(b"");
    if name.is_empty() || name.len() > 95 { return Err(b"pip: invalid wheel filename."); }
    let mut mounted = [0; 101]; mounted[..6].copy_from_slice(b"/work/"); mounted[6..6 + name.len()].copy_from_slice(name);
    let mounted = &mounted[..6 + name.len()];
    let (metadata, _) = crate::storage::object_inspect_path(path).map_err(|_| b"pip: wheel object not found.".as_slice())?;
    if !crate::runtime::native_python_scope::readable(metadata.owner.0, console.current_user.0,
        metadata.space == crate::storage::object::Space::Personal, path) { return Err(b"pip: wheel belongs to another user or protected space."); }
    let (id, length) = crate::storage::object_read_path(path, Some(metadata.current_version), &mut job.archive[..4 * 1024 * 1024])
        .map_err(|_| b"pip: cannot snapshot wheel (maximum 4 MiB).".as_slice())?;
    if id != metadata.object.id { return Err(b"pip: wheel changed while opening."); }
    if let Ok(existing) = job.filesystem.read(mounted) {
        let same = existing == &job.archive[..length]; job.archive[..length].fill(0);
        return if same { Ok(()) } else { Err(b"pip: different wheels share one filename; use a single unambiguous wheelhouse.") };
    }
    job.filesystem.mount_readonly(mounted, &job.archive[..length])
        .map_err(|_| b"pip: offline wheelhouse exceeds the bounded process filesystem.".as_slice())?;
    job.archive[..length].fill(0); Ok(())
}

// ------------------------=
// FUNC: start_pip
// DESC: Admits only bounded offline wheel operations into a staged per-user package transaction; no source builds, URLs or target overrides.
// ------------------=
unsafe fn start_pip(console: &ConsoleRuntime, arguments: &[&[u8]]) -> Result<(), &'static [u8]> {
    let job = &mut *(&raw mut JOB);
    if job.task.is_some() { return Err(b"pip: Python is already running; use python-cancel."); }
    if !active(console) { return Err(b"pip: an authenticated session is required."); }
    let mode = admit_pip(arguments).map_err(|error| match error {
        PolicyError::Command => b"pip: supported offline commands: install, uninstall -y, list, show, check, freeze, help, --version.".as_slice(),
        PolicyError::Confirmation => b"pip: use uninstall -y NAME to explicitly confirm removal.",
        PolicyError::Option => b"pip: this option is not supported by the confined offline installer.",
        PolicyError::Input => b"pip: only local wheels and package names are supported; URLs, source trees and requirement files are unavailable.",
        PolicyError::SelfModification => b"pip: the OS-managed pip installation cannot be replaced or removed.",
    })?;
    let install = mode == PipMode::Install;
    prepare_packages(console, job, mode != PipMode::ReadOnly)?;
    job.pip = true;
    let mut owned = [[0u8; 256]; 24]; let mut lengths = [0usize; 24];
    if arguments.len() > owned.len() { return Err(b"pip: too many arguments."); }
    for (i, value) in arguments.iter().enumerate() {
        if install && value.starts_with(b"/") {
            mount_wheel(console, job, value)?;
            let slash = value.iter().rposition(|b| *b == b'/').ok_or(b"pip: invalid wheel path.".as_slice())?;
            let directory = if slash == 0 { b"/".as_slice() } else { &value[..slash] };
            for n in 0..crate::storage::object::NAMESPACE_CAPACITY {
                let Some(entry) = crate::storage::namespace_child_nth(directory, n)
                    .map_err(|_| b"pip: cannot inspect offline dependency wheels.".as_slice())? else { break; };
                let sibling = &entry.path[..entry.path_len as usize];
                if sibling.ends_with(b".whl") {
                    let (metadata, _) = crate::storage::object_inspect_path(sibling)
                        .map_err(|_| b"pip: dependency wheel changed.".as_slice())?;
                    if crate::runtime::native_python_scope::readable(metadata.owner.0, console.current_user.0,
                        metadata.space == crate::storage::object::Space::Personal, sibling) { mount_wheel(console, job, sibling)?; }
                }
            }
            let name = &value[slash + 1..]; owned[i][..6].copy_from_slice(b"/work/");
            owned[i][6..6 + name.len()].copy_from_slice(name); lengths[i] = 6 + name.len();
        } else {
            if value.len() > owned[i].len() { return Err(b"pip: argument too long."); }
            owned[i][..value.len()].copy_from_slice(value); lengths[i] = value.len();
        }
    }
    let mut args: [&[u8]; 32] = [&[]; 32];
    args[..5].copy_from_slice(&[b"/system/python", b"-m", b"pip", b"--disable-pip-version-check", b"--no-cache-dir"]);
    let mut count = 5;
    for i in 0..arguments.len() { args[count] = &owned[i][..lengths[i]]; count += 1; }
    if arguments.is_empty() { args[count] = b"--help"; count += 1; }
    if install {
        if count + 5 > args.len() { return Err(b"pip: too many install arguments."); }
        for value in [b"--no-index".as_slice(), b"--find-links=/work", b"--only-binary=:all:", b"--progress-bar=off", b"--no-compile"] { args[count] = value; count += 1; }
    }
    start_task(console, job, &args[..count])
}

// ------------------------=
// FUNC: start
// DESC: Captures script-local imports and launches one isolated, cancellable native CPython process.
// ------------------=
unsafe fn start(console: &ConsoleRuntime, path: &[u8], arguments: &[&[u8]], expected_source: Option<&[u8]>) -> Result<(), &'static [u8]> {
    let job = &mut *(&raw mut JOB);
    if job.task.is_some() { return Err(b"python: a script is already running; use python-cancel."); }
    if !active(console) { return Err(b"python: an authenticated session is required."); }
    prepare_packages(console, job, false)?;
    snapshot(&mut job.files[0], path, console.current_user.0)?;
    if expected_source.is_some_and(|source| source != &job.files[0].bytes[..job.files[0].len]) {
        return Err(b"python: saved source changed before validation; no code was executed.");
    }
    job.count = 1;
    let slash = path.iter().rposition(|b| *b == b'/').unwrap_or(0);
    let directory = if slash == 0 { b"/".as_slice() } else { &path[..slash] };
    for index in 0..256 {
        let entry = crate::storage::namespace_child_nth(directory, index)
            .map_err(|_| b"python: cannot inspect script imports.".as_slice())?;
        let Some(entry) = entry else { break; };
        let candidate = &entry.path[..entry.path_len as usize];
        if candidate == path || !(candidate.ends_with(b".py") || candidate.ends_with(b".so")) { continue; }
        let (metadata, _) = crate::storage::object_inspect_path(candidate)
            .map_err(|_| b"python: import changed during snapshot.".as_slice())?;
        if !crate::runtime::native_python_scope::readable(metadata.owner.0, console.current_user.0,
            metadata.space == crate::storage::object::Space::Personal, candidate) { continue; }
        if job.count == job.files.len() { return Err(b"python: script directory exceeds eight Python/native input objects."); }
        snapshot(&mut job.files[job.count], candidate, console.current_user.0)?;
        job.count += 1;
    }
    for source in job.files.iter().take(job.count) {
        job.filesystem.mount_readonly(&source.path[..source.path_len], &source.bytes[..source.len])
            .map_err(|_| b"python: cannot mount private script inputs.".as_slice())?;
    }
    start_task(console, job, arguments)
}

// ------------------------=
// FUNC: start_task
// DESC: Admits one native CPython process with a unique process identity while retaining immutable input snapshots between validation and launch.
// ------------------=
unsafe fn start_task(console: &ConsoleRuntime, job: &mut Job, arguments: &[&[u8]]) -> Result<(), &'static [u8]> {
    let Some((kernel, controller)) = crate::runtime::native_compile_job::platform() else {
        return Err(b"python: native process runtime unavailable.");
    };
    let now = crate::runtime::ai::qwen::workers::clock_ns();
    if now == 0 { return Err(b"python: native clock is unavailable."); }
    job.owner = SecurityIdentity(console.current_session.0);
    let secret = crate::runtime::native_python_entropy::next(&console.current_session.0)
        .ok_or(b"python: validated boot entropy is unavailable.".as_slice())?;
    job.output.fill(0); job.errors.fill(0);
    job.initial_pages = memory_mut().available_pages() as u64;
    let id = NEXT_TASK_ID;
    NEXT_TASK_ID = id.checked_add(1).ok_or(b"python: process identifiers exhausted.".as_slice())?;
    job.task = Some(Task::start(crate::runtime::native_python_package::PYTHON, arguments,
        job.owner, id, 1, memory_mut(), kernel, controller,
        now.saturating_add(120_000_000_000)).map_err(|_| b"python: cannot allocate native process.".as_slice())?);
    job.task.as_mut().unwrap().set_launch_secret(secret);
    publish(1, 0, 0, 0, job.initial_pages, memory_mut().available_pages() as u64);
    Ok(())
}

// ------------------------=
// FUNC: generated
// DESC: Validates a saved generated Python file through native CPython before optionally running the exact same immutable source.
// ------------------=
pub(super) fn generated(console: &mut ConsoleRuntime, path: &[u8], goal: Option<u64>, launch: bool) -> Result<(), &'static [u8]> {
    if !path.ends_with(b".py") || !console.editor_document.is_saved()
        || path != &console.editor_document_path[..console.editor_document_path_length] {
        return Err(b"python: save the current Python source before validation.");
    }
    if !console.ai_chat_allowed() || !crate::runtime::ai::with_ai_runtime(|ai| ai.generation_workflow_current(goal)) {
        return Err(b"python: this generation request no longer owns execution authority.");
    }
    let (metadata, _) = crate::storage::object_inspect_path(path)
        .map_err(|_| b"python: saved source is unavailable.".as_slice())?;
    let panel = crate::ui::app_assistant::read(2);
    if panel.workflow_goal != goal { return Err(b"python: the source request was replaced."); }
    let generated = Generated { goal, user: console.current_user.0, source: metadata.object.id.0,
        revision: console.assistant_revision(),
        request: crate::ui::app_assistant::fingerprint(&panel.request[..panel.request_len], 0, None),
        phase: workflow::Phase::Validate, launch };
    unsafe {
        start(console, path, &[b"/system/python", b"-c", workflow::VALIDATE_SOURCE, path], Some(console.editor_document.bytes()))?;
        (&mut *(&raw mut JOB)).generated = Some(generated);
    }
    console.open_command_window();
    console.output.write_line(b"Python: validating saved source without running it.");
    console.output.write_line(path);
    console.redraw();
    Ok(())
}

// ------------------------=
// FUNC: generated_current
// DESC: Fences asynchronous Python completion to the same session, editor snapshot, user request and active software goal.
// ------------------=
fn generated_current(console: &ConsoleRuntime, generated: Generated) -> bool {
    let panel = crate::ui::app_assistant::read(2);
    console.ai_chat_allowed() && console.assistant_owner_alive(2)
        && generated.user == console.current_user.0 && generated.revision == console.assistant_revision()
        && panel.workflow_goal == generated.goal
        && !matches!(panel.generation, crate::ui::app_assistant::GenerationStatus::Cancelled | crate::ui::app_assistant::GenerationStatus::Failed)
        && generated.request == crate::ui::app_assistant::fingerprint(&panel.request[..panel.request_len], 0, None)
        && crate::runtime::ai::with_ai_runtime(|ai| ai.generation_workflow_current(generated.goal))
}

// ------------------------=
// FUNC: generated_event
// DESC: Delivers only ordered native receipts to the initiating user's exact software goal.
// ------------------=
fn generated_event(generated: Generated, event: Event) {
    if let Some(goal) = generated.goal {
        crate::runtime::ai::with_ai_runtime(|ai| ai.record_software_event(goal, generated.user, event));
    }
}

// ------------------------=
// FUNC: finish_generated
// DESC: Advances validation to execution only with native success evidence, or retains diagnostics for a bounded unchanged-source repair.
// ------------------=
unsafe fn finish_generated(console: &mut ConsoleRuntime, job: &mut Job, generated: Generated,
    result: State, reclaimed: bool, execution: Option<u32>, error_length: usize) -> bool {
    use crate::ui::app_assistant as assistant;
    let mut panel = assistant::read(2);
    let exit = match result { State::Exited(code) => Some(code), _ => None };
    match workflow::outcome(generated.phase, exit, reclaimed, generated.launch, execution) {
        workflow::Outcome::Validated => {
            generated_event(generated, Event::Compiled { artifact: generated.source });
            panel.reply(b"Python source saved and syntax validated by the native interpreter. No application was launched.");
        }
        workflow::Outcome::Launch => {
            generated_event(generated, Event::Compiled { artifact: generated.source });
            let path = job.files[0].path;
            let length = job.files[0].path_len;
            match start_task(console, job, &[b"/system/python", &path[..length]]) {
                Ok(()) => {
                    job.generated = Some(Generated { phase: workflow::Phase::Run, ..generated });
                    console.output.write_line(b"Launching Python application:");
                    console.output.write_line(&path[..length]);
                    panel.reply(b"Syntax validation passed. Launching the saved Python application in a native process; its output appears in Command.");
                    assistant::write(2, panel);
                    return true;
                }
                Err(error) => {
                    generated_event(generated, Event::Failed { code: 3 });
                    panel.generation = assistant::GenerationStatus::Failed;
                    panel.reply(error);
                    console.output.write_line(error);
                }
            }
        }
        workflow::Outcome::Executed(context) => {
            generated_event(generated, Event::Launched { context: context as u64 });
            panel.reply(b"Python application ran successfully in a native process. Its output is available in Command.");
        }
        workflow::Outcome::Failed => {
            let request = panel.request;
            let request_length = panel.request_len;
            let repairing = workflow::repairable(generated.phase, exit, reclaimed)
                && panel.queue_compiler_repair(&request[..request_length], generated.revision,
                    console.assistant_revision(), &job.errors[..error_length]);
            if repairing {
                panel.reply(b"The native Python interpreter found syntax errors. Repairing this unchanged saved source using its diagnostics (maximum two attempts). Nothing has launched.");
            } else {
                generated_event(generated, Event::Failed { code: if generated.phase == workflow::Phase::Validate { 2 } else { 3 } });
                panel.generation = if matches!(result, State::Cancelled) { assistant::GenerationStatus::Cancelled }
                    else { assistant::GenerationStatus::Failed };
                panel.reply(if error_length > 0 { &job.errors[..error_length] } else {
                    b"Python validation or execution did not complete successfully. Your saved source remains available in the editor."
                });
            }
        }
    }
    assistant::write(2, panel);
    false
}

// ------------------------=
// FUNC: command
// DESC: Accepts native Python script commands and cancellation without running user code inside input dispatch.
// ------------------=
pub(super) fn command(console: &mut ConsoleRuntime, input: &[u8]) -> bool { unsafe {
    if input == b"python-cancel" {
        let job = &mut *(&raw mut JOB);
        if job.owner == SecurityIdentity(console.current_session.0) {
            if let Some(task) = job.task.as_mut() { task.cancel(); }
        }
        return true;
    }
    let mut words = input.split(|b| b.is_ascii_whitespace()).filter(|word| !word.is_empty());
    let name = words.next();
    if !matches!(name, Some(b"python") | Some(b"python3") | Some(b"pip") | Some(b"pip3")) { return false; }
    let mut args: [&[u8]; 32] = [&[]; 32]; args[0] = b"/system/python";
    let mut count = 1;
    for word in words {
        if count == args.len() { console.output.write_line(b"python: too many arguments (maximum 31)."); return true; }
        args[count] = word; count += 1;
    }
    let pip = matches!(name, Some(b"pip") | Some(b"pip3"));
    if pip || count >= 3 && args[1] == b"-m" && args[2] == b"pip" {
        let result = start_pip(console, if pip { &args[1..count] } else { &args[3..count] });
        console.output.write_line(match result { Ok(()) => b"pip started: offline wheels, private per-user packages.", Err(error) => error });
        return true;
    }
    if count >= 3 && args[1] == b"-m" || count == 2 && matches!(args[1], b"--version" | b"-V") {
        let job = &mut *(&raw mut JOB);
        let result = if job.task.is_some() { Err(b"python: a script is already running; use python-cancel.".as_slice()) }
            else if !active(console) { Err(b"python: an authenticated session is required.".as_slice()) }
            else { prepare_packages(console, job, false).and_then(|_| start_task(console, job, &args[..count])) };
        console.output.write_line(match result { Ok(()) => b"Python module started.", Err(error) => error });
        return true;
    }
    if count < 2 || args[1] == b"--help" {
        console.output.write_line(b"python /home/default/documents/hello.py [arguments]");
        console.output.write_line(b"python-cancel stops your running script. Paths must be absolute.");
        console.output.write_line(b"pip install /absolute/package.whl; pip list; pip show NAME; pip uninstall -y NAME");
        return true;
    }
    match start(console, args[1], &args[..count], None) {
        Ok(()) => console.output.write_line(b"Python script started."),
        Err(error) => console.output.write_line(error),
    }
    true
}}

// ------------------------=
// FUNC: poll
// DESC: Executes one native slice, reports output only to its owner, and reclaims all process and extension memory.
// ------------------=
pub(super) fn poll(console: &mut ConsoleRuntime) { unsafe {
    let job = &mut *(&raw mut JOB);
    let Some(task) = job.task.as_mut() else { return; };
    let authorized = job.owner == SecurityIdentity(console.current_session.0) && job.package_owner == console.current_user.0 && active(console)
        && job.generated.is_none_or(|generated| generated_current(console, generated));
    let mut files: [File<'_>; 8] = core::array::from_fn(|_| File { path: b"", bytes: b"" });
    for (file, source) in files.iter_mut().zip(job.files.iter()).take(job.count) {
        *file = File { path: &source.path[..source.path_len], bytes: &source.bytes[..source.len] };
    }
    let result = task.poll_with_filesystem(crate::runtime::ai::qwen::workers::clock_ns(), authorized, memory_mut(),
        b"", &files[..job.count], &mut job.output, &mut job.errors,
        |tables, operation, arguments| job.filesystem.syscall(tables, operation, arguments));
    if matches!(result, State::Runnable) { return; }
    let execution = task.execution_id();
    let lengths = task.result_lengths(b"", &mut job.output, &mut job.errors);
    if authorized {
        if let Some((length, errors, truncated)) = lengths {
            for text in [&job.output[..length], &job.errors[..errors]] {
                for line in text.split(|b| *b == b'\n').filter(|line| !line.is_empty()) { console.output.write_line(line); }
            }
            if truncated { console.output.write_line(b"python: output exceeded capture limit."); }
        }
        if !job.pip { console.output.write_line(match result {
            State::Exited(0) => b"Python completed successfully.",
            State::Exited(_) => b"Python exited with an error.",
            State::Cancelled => b"Python cancelled.",
            State::Deadline => b"Python exceeded its two-minute execution limit.",
            _ => b"Python process faulted; native runtime stopped it.",
        }); }
    }
    let cleanup = job.task.take().unwrap().release(memory_mut()).is_ok();
    if !cleanup && authorized {
        console.output.write_line(b"python: process cleanup failed.");
    }
    let mut packages_committed = true;
    if job.pip && authorized && cleanup && result == State::Exited(0) && job.filesystem.dirty() {
        packages_committed = job.filesystem.encode(&mut job.archive).ok().is_some_and(|length|
            crate::storage::python_packages_save(job.package_owner, console.current_session.0,
                job.package_revision, &job.archive[..length]).is_ok());
        console.output.write_line(if packages_committed { b"pip: package changes saved for this user." }
            else { b"pip: package transaction could not be saved (session, version or storage changed). Previous packages remain unchanged." });
    } else if job.pip && job.filesystem.dirty() && authorized {
        console.output.write_line(b"pip: unsuccessful operation discarded; installed packages remain unchanged.");
    }
    if job.pip && authorized {
        console.output.write_line(match result {
            State::Exited(0) if cleanup && packages_committed => b"pip completed successfully.",
            State::Cancelled => b"pip cancelled; no package changes were committed.",
            State::Deadline => b"pip exceeded its two-minute limit; no package changes were committed.",
            _ => b"pip did not complete successfully; previous installed packages remain unchanged.",
        });
    }
    let state = match result { State::Exited(_) if cleanup && packages_committed => 2, State::Cancelled => 3, _ => 4 };
    let exit = match result { State::Exited(code) if packages_committed => code, _ => -1 };
    let (stdout, stderr, _) = lengths.unwrap_or((0, 0, false));
    publish(state, exit, stdout, stderr, job.initial_pages, memory_mut().available_pages() as u64);
    if let Some(generated) = job.generated {
        if authorized {
            if finish_generated(console, job, generated, result, cleanup, execution, stderr) {
                console.redraw();
                return;
            }
        } else {
            // A stale result may retire only its old goal; it never rewrites the current editor or panel.
            generated_event(generated, Event::Failed { code: 4 });
        }
    }
    job.generated = None;
    job.count = 0;
    job.pip = false; job.archive.fill(0); job.filesystem.reset(false);
    console.redraw();
}}
