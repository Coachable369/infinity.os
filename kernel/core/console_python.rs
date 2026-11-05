//! Native Python command execution. Object snapshots are private to one authenticated session.
use super::ConsoleRuntime;
use crate::runtime::{execution::SecurityIdentity, native_compiler_task::Task,
    native_job_files::File, native_memory::memory_mut, native_process::State,
    native_python_packages::{Vfs, MAX_ARCHIVE, admit_pip, PipMode, PolicyError}};
#[path = "../runtime/native_python_workflow.rs"]
mod workflow;
#[path = "../runtime/generated_app_recovery.rs"]
mod recovery;
use super::native_network as network;
#[path = "console_python_extensions.rs"]
mod extensions;
#[path = "console_python_child.rs"]
mod child;
#[path = "console_python_data.rs"]
mod app_data;
use crate::runtime::ai::intelligence::software::Event;
use crate::runtime::{native_extensions, native_job_files, native_python_packages};

const LIMIT: usize = 1024 * 1024;
struct Snapshot { path: [u8; 95], path_len: usize, bytes: [u8; LIMIT], len: usize, object: [u8; 16], version: u32 }
impl Snapshot {
    const EMPTY: Self = Self { path: [0; 95], path_len: 0, bytes: [0; LIMIT], len: 0, object: [0; 16], version: 0 };
}
struct Job {
    tool_export: Option<crate::runtime::object_navigation::ByteText<96>>,
    monitor:Option<crate::runtime::execution::ContextHandle>,
    task: Option<Task>, owner: SecurityIdentity,
    files: [Snapshot; 8], count: usize,
    output: [u8; 32768], errors: [u8; 8192],
    initial_pages: u64,
    generated: Option<Generated>,
    recovery: Option<recovery::Snapshot>,
    exception: Option<crate::runtime::native_python_exception::Evidence>,
    filesystem: Vfs,
    data: app_data::Data,
    archive: [u8; MAX_ARCHIVE],
    package_revision: Option<crate::storage::object::PythonPackageRevision>,
    package_owner: [u8; 16],
    pip: bool,
    input: crate::runtime::native_console_input::Input,
    shown_output: usize, shown_errors: usize, drained_output: usize, drained_errors: usize,
    application: Option<crate::runtime::ai::application_host::Context>, ui_revision: u64,
    interaction_objective:[u8;512], interaction_length:usize,
    network: network::Network,
    extensions: extensions::Snapshots,
    child: child::Child,
    gui: crate::runtime::native_gui::Gui,
    gui_revision: u64,
    gui_keyboard: bool,
}
#[derive(Clone, Copy)]
struct Generated {
    goal: Option<u64>, user: [u8; 16], source: [u8; 16], revision: u64,
    request: u64, phase: workflow::Phase, launch: bool, launched: bool, context: u64,
    workflow_revision: u64, staged:bool,
}
static mut MONITOR_SAMPLE_NS:u64=0;
static mut MONITOR_RESUME_NS:u64=0;
static mut JOB: Job = Job { tool_export:None, monitor:None, task: None, owner: SecurityIdentity([0; 16]),
    files: [const { Snapshot::EMPTY }; 8], count: 0, output: [0; 32768], errors: [0; 8192], initial_pages: 0, generated: None,
    recovery: None, exception: None,
    filesystem: Vfs::new(), data: app_data::Data::new(), archive: [0; MAX_ARCHIVE], package_revision: None, package_owner: [0; 16], pip: false,
    input: crate::runtime::native_console_input::Input::new(), shown_output: 0, shown_errors: 0, drained_output: 0, drained_errors: 0,
    application: None, ui_revision: 0,
    interaction_objective:[0;512], interaction_length:0,
    network: network::Network::new(), extensions: extensions::Snapshots::new(),
    child: child::Child::new(),
    gui: crate::runtime::native_gui::Gui::new(), gui_revision: 0, gui_keyboard: false };
static mut NEXT_TASK_ID: u32 = 0x50590001;
static mut RUNTIME_DIAGNOSTICS: crate::ui::app_assistant::compiler_diagnostics::Receipt = crate::ui::app_assistant::compiler_diagnostics::Receipt::new();

// ------------------------=
// FUNC: repair_diagnostics
// DESC: Supplies native Python feedback only to the same authenticated session and unchanged editor revision.
// ------------------=
pub(super) fn repair_diagnostics(console: &ConsoleRuntime, output: &mut [u8]) -> usize {
    if !active(console)||!console.ai_chat_allowed(){return 0;}
    unsafe { (&*(&raw const RUNTIME_DIAGNOSTICS)).copy_for(console.current_session.0, console.assistant_revision(), output) }
}

#[no_mangle]
pub static mut INFINITY_PYTHON_SNAPSHOT: [u64; 10] = [0x494e46505954484e, 1, 0, 0, 0, 0, 0, 0, 0, 0];

#[no_mangle]
pub static mut INFINITY_NATIVE_GUI_SNAPSHOT: [u64; 80] = [0;80];

// ------------------------=
// FUNC: publish_gui
// DESC: Publishes seqlocked native window geometry and lifecycle facts for read-only installed interaction diagnostics, never authority or application data.
// ------------------=
fn publish_gui(runtime:&crate::runtime::InfinityRuntime,gui:&crate::runtime::native_gui::Gui) {
    let mut values=[0u64;80];
    values[0]=0x494e464e47554931; values[1]=1;
    values[4]=runtime.ui.windows.focused().map_or(0,|id|id.0 as u64);
    values[5]=gui.context().map_or(0,|owner|owner.0 as u64); values[6]=gui.revision();
    if let Some(owner)=gui.context() {
        for index in 0..runtime.ui.windows.count() {
            let Some(window)=runtime.ui.windows.ordered_window(index).filter(|w|w.owner==owner) else {continue;};
            if values[3]>=8 {break;}
            let at=8+values[3] as usize*8; values[3]+=1;
            values[at..at+8].copy_from_slice(&[window.id.0 as u64,window.bounds.x as i64 as u64,window.bounds.y as i64 as u64,
                window.bounds.width as u64,window.bounds.height as u64,window.state as u64,window.visible as u64,window.surface.0 as u64]);
        }
    }
    unsafe {
        let target=core::ptr::addr_of_mut!(INFINITY_NATIVE_GUI_SNAPSHOT).cast::<u64>();
        let sequence=core::ptr::read_volatile(target.add(2)).wrapping_add(2)&!1;
        core::ptr::write_volatile(target.add(2),sequence|1);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        for (index,value) in values.iter().enumerate() {if index!=2&&index!=79 {core::ptr::write_volatile(target.add(index),*value);}}
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        core::ptr::write_volatile(target.add(79),sequence); core::ptr::write_volatile(target.add(2),sequence);
    }
}

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
    target.object = id.0; target.version = metadata.current_version;
    Ok(())
}

// ------------------------=
// FUNC: prepare_packages
// DESC: Starts each invocation from the current user's committed archive with no inherited staged writes or previous-user files.
// ------------------=
fn prepare_packages(console: &ConsoleRuntime, job: &mut Job, writable: bool) -> Result<(), &'static [u8]> {
    job.tool_export=None;
    job.network.reset(SecurityIdentity(console.current_session.0), false);
    job.extensions.clear();
    job.child.clear();
    job.filesystem.reset(writable); job.archive.fill(0); job.count = 0; job.generated = None;
    job.recovery = None; job.exception = None;
    job.data = app_data::Data::new();
    job.interaction_objective.fill(0);job.interaction_length=0;
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
// FUNC: mount_package
// DESC: Snapshots an authorized wheel or source archive into the read-only package inputs without exposing its native directory to pip.
// ------------------=
fn mount_package(console: &ConsoleRuntime, job: &mut Job, path: &[u8]) -> Result<(), &'static [u8]> {
    if !path.starts_with(b"/") || path.len() > 95 || !native_python_packages::package_archive(path) {
        return Err(b"pip: supply an absolute ObjectStore path to a wheel or supported source archive.");
    }
    let name = path.rsplit(|b| *b == b'/').next().unwrap_or(b"");
    if name.is_empty() || name.len() > 95 { return Err(b"pip: invalid wheel filename."); }
    let mut mounted = [0; 101]; mounted[..6].copy_from_slice(b"/work/"); mounted[6..6 + name.len()].copy_from_slice(name);
    let mounted = &mounted[..6 + name.len()];
    let (metadata, _) = crate::storage::object_inspect_path(path).map_err(|_| b"pip: wheel object not found.".as_slice())?;
    if !crate::runtime::native_python_scope::readable(metadata.owner.0, console.current_user.0,
        metadata.space == crate::storage::object::Space::Personal, path) { return Err(b"pip: wheel belongs to another user or protected space."); }
    let (id, length) = crate::storage::object_read_path(path, Some(metadata.current_version), &mut job.archive[..native_python_packages::limits::FILE_BYTES])
        .map_err(|_| b"pip: cannot snapshot package archive (maximum 16 MiB).".as_slice())?;
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
// DESC: Admits bounded wheel operations into a staged per-user package transaction; online names use the authenticated native HTTPS service.
// ------------------=
unsafe fn start_pip(console: &ConsoleRuntime, arguments: &[&[u8]]) -> Result<(), &'static [u8]> {
    let job = &mut *(&raw mut JOB);
    if job.task.is_some() { return Err(b"pip: Python is already running; use python-cancel."); }
    if !active(console) { return Err(b"pip: an authenticated session is required."); }
    let mode = admit_pip(arguments).map_err(|error| match error {
        PolicyError::Command => b"pip: supported commands: install, uninstall -y, list, show, check, freeze, help, --version.".as_slice(),
        PolicyError::Confirmation => b"pip: use uninstall -y NAME to explicitly confirm removal.",
        PolicyError::Option => b"pip: this option is not supported by the confined native installer.",
        PolicyError::Input => b"pip: use a package name, a local wheel/source archive, or a verified HTTPS package archive URL.",
        PolicyError::SelfModification => b"pip: OS-managed pip and timezone data cannot be replaced or removed.",
    })?;
    let install = mode == PipMode::Install;
    let online = install && arguments.iter().skip(1).any(|value| !value.starts_with(b"-") && !value.starts_with(b"/"));
    prepare_packages(console, job, mode != PipMode::ReadOnly)?;
    job.network.reset(SecurityIdentity(console.current_session.0), online);
    job.pip = true;
    let mut owned = [[0u8; 256]; 24]; let mut lengths = [0usize; 24];
    if arguments.len() > owned.len() { return Err(b"pip: too many arguments."); }
    for (i, value) in arguments.iter().enumerate() {
        if install && value.starts_with(b"/") {
            mount_package(console, job, value)?;
            let slash = value.iter().rposition(|b| *b == b'/').ok_or(b"pip: invalid wheel path.".as_slice())?;
            let directory = if slash == 0 { b"/".as_slice() } else { &value[..slash] };
            for n in 0..crate::storage::object::NAMESPACE_CAPACITY {
                let Some(entry) = crate::storage::namespace_child_nth(directory, n)
                    .map_err(|_| b"pip: cannot inspect offline dependency wheels.".as_slice())? else { break; };
                let sibling = &entry.path[..entry.path_len as usize];
                if native_python_packages::package_archive(sibling) {
                    let (metadata, _) = crate::storage::object_inspect_path(sibling)
                        .map_err(|_| b"pip: dependency wheel changed.".as_slice())?;
                    if crate::runtime::native_python_scope::readable(metadata.owner.0, console.current_user.0,
                        metadata.space == crate::storage::object::Space::Personal, sibling) { mount_package(console, job, sibling)?; }
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
        for value in [if online { b"--index-url=https://pypi.org/simple".as_slice() } else { b"--no-index".as_slice() },
            b"--find-links=/work", b"--prefer-binary", b"--progress-bar=off", b"--no-compile"] { args[count] = value; count += 1; }
    }
    start_task(console, job, &args[..count])
}

// ------------------------=
// FUNC: start
// DESC: Captures script-local imports and launches one isolated, cancellable native CPython process.
// ------------------=
unsafe fn start(console: &ConsoleRuntime, path: &[u8], arguments: &[&[u8]], expected_source: Option<&[u8]>) -> Result<(), &'static [u8]> {
    start_snapshot(console,path,arguments,expected_source,None)
}
// ------------------------=
// FUNC: start_snapshot
// DESC: Mounts an authorized private source snapshot with an optional repair overlay for validation.
// ------------------=
unsafe fn start_snapshot(console:&ConsoleRuntime,path:&[u8],arguments:&[&[u8]],expected_source:Option<&[u8]>,candidate:Option<&[u8]>)->Result<(),&'static [u8]> {
    let job = &mut *(&raw mut JOB);
    if job.task.is_some() { return Err(b"python: a script is already running; use python-cancel."); }
    if !active(console) { return Err(b"python: an authenticated session is required."); }
    prepare_packages(console, job, false)?;
    snapshot(&mut job.files[0], path, console.current_user.0)?;
    if expected_source.is_some_and(|source| source != &job.files[0].bytes[..job.files[0].len]) {
        return Err(b"python: saved source changed before validation; no code was executed.");
    }
    if let Some(bytes)=candidate {if bytes.len()>LIMIT{return Err(b"Repair exceeds Python source limit.");}
        job.files[0].bytes[..bytes.len()].copy_from_slice(bytes);job.files[0].len=bytes.len();}
    job.count = 1;
    if crate::runtime::native_python_scope::system_script(path) {
        job.filesystem.mount_readonly(path,&job.files[0].bytes[..job.files[0].len]).map_err(|_|b"Cannot mount system tool.".as_slice())?;
        return start_system_tool(console,job,path,arguments);
    }
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
// FUNC: export_allowed
// DESC: Restricts new output to an active user's personal directory and never replaces existing objects.
// ------------------=
fn export_allowed(console:&ConsoleRuntime,path:&[u8])->bool {
    if !console.workplace_authorized(true)||path.len()>95||crate::storage::namespace_resolve(path).is_ok(){return false;}
    let Ok(parent)=crate::runtime::object_navigation::parent_path(path) else{return false;};
    crate::storage::object_inspect_path(parent.as_bytes()).is_ok_and(|(m,_)|
        m.kind==crate::storage::object::ObjectType::NamespaceNode && m.space==crate::storage::object::Space::Personal &&
        (m.owner.0==console.current_user.0 || m.owner.0==[0;16] && parent.as_bytes()==b"/home/default/documents"))
}

// ------------------------=
// FUNC: start_system_tool
// DESC: Mounts only named, authorized object snapshots and publishes FIGlet output only after successful execution.
// ------------------=
unsafe fn start_system_tool(console:&ConsoleRuntime,job:&mut Job,path:&[u8],arguments:&[&[u8]])->Result<(),&'static [u8]> {
    use crate::runtime::object_navigation::ByteText;
    let mut names=[ByteText::<96>::empty();32];let mut argv=[b"".as_slice();32];
    if arguments.len()>argv.len(){return Err(b"Too many arguments.");}
    argv[..arguments.len()].copy_from_slice(arguments);
    if path==b"/System/figlet.py" {
        for font in [b"/System/standard.flf".as_slice(),b"/System/small.flf"] {
            let (_,n)=crate::storage::object_read_path(font,None,&mut job.archive).map_err(|_|b"Missing FIGfont.".as_slice())?;
            job.filesystem.mount_readonly(font,&job.archive[..n]).map_err(|_|b"Cannot mount FIGfont.".as_slice())?;
        }
        let mut i=2;
        while i<arguments.len() {
            if arguments[i]==b"--"{break;}
            if matches!(arguments[i],b"-o"|b"--output"){
                let dest=arguments.get(i+1).and_then(|p|console.utility_path(p)).filter(|p|export_allowed(console,p.as_bytes()))
                    .ok_or(b"Use -o with a new file in your personal directory.".as_slice())?;
                job.tool_export=Some(dest);argv[i+1]=b"/tmp/figlet.txt";i+=2;
            } else if let Some(file)=arguments[i].strip_prefix(b"--output=").or_else(||arguments[i].strip_prefix(b"-o").filter(|p|!p.is_empty())){
                let dest=console.utility_path(file).filter(|p|export_allowed(console,p.as_bytes()))
                    .ok_or(b"Output must be a new file in your personal directory.".as_slice())?;
                job.tool_export=Some(dest);argv[i]=b"--output=/tmp/figlet.txt";i+=1;
            } else {i+=1;}
        }
    }else{
        let mut program=false;let mut skip_separator=false;let mut options=true;
        for (i,&word) in arguments.iter().enumerate().skip(2){
            if skip_separator{skip_separator=false;continue;}
            if options && word==b"--"{options=false;continue;}
            if options && path==b"/System/awk.py" && word==b"-F"{skip_separator=true;continue;}
            if options && word.starts_with(b"-") && word!=b"-"{continue;}
            if !program{program=true;continue;}
            if word==b"-"{continue;}
            let name=console.utility_path(word).ok_or(b"Invalid input filename.".as_slice())?;
            let (m,_)=crate::storage::object_inspect_path(name.as_bytes()).map_err(|_|b"Input file not found.".as_slice())?;
            if !crate::runtime::native_python_scope::readable(m.owner.0,console.current_user.0,
                m.space==crate::storage::object::Space::Personal,name.as_bytes()) && name.as_bytes()!=b"/System/motd" {return Err(b"Input file is not readable by this user.");}
            if job.filesystem.read(name.as_bytes()).is_err() {
                let (id,n)=crate::storage::object_read_path(name.as_bytes(),Some(m.current_version),&mut job.archive)
                    .map_err(|_|b"Input file exceeds snapshot limit or is unavailable.".as_slice())?;
                if id!=m.object.id{return Err(b"Input changed; retry.");}
                job.filesystem.mount_readonly(name.as_bytes(),&job.archive[..n]).map_err(|_|b"Input exceeds Python mount quota.".as_slice())?;
            }
            names[i]=name;
        }
        for (i,name) in names.iter().enumerate().take(arguments.len()){
            if !name.as_bytes().is_empty(){argv[i]=name.as_bytes();}
        }
    }
    start_task(console,job,&argv[..arguments.len()])
}

// ------------------------=
// FUNC: path_command
// DESC: Invokes a PATH-resolved Python program with bounded quoted arguments without spawning a host shell.
// ------------------=
pub(super) fn path_command(console:&mut ConsoleRuntime,line:&[u8])->bool {
    let line=line.trim_ascii();
    let end=line.iter().position(u8::is_ascii_whitespace).unwrap_or(line.len());
    let Some(path)=console.shell_script(&line[..end]) else{return false;};
    let mut bytes=[0;super::COMMAND_CAPACITY];let mut spans=[(0,0);30];let mut count=0;
    let mut cursor=end;let mut n=0;
    while cursor<line.len(){
        if line[cursor].is_ascii_whitespace(){cursor+=1;continue;}
        if count==spans.len(){console.output.write_line(b"Too many arguments.");return true;}
        let first=n;let mut quote=0;
        while cursor<line.len(){
            let b=line[cursor];cursor+=1;
            if quote==0 && b.is_ascii_whitespace(){break;}
            if quote==0 && matches!(b,39|34){quote=b;continue;}
            if quote==b{quote=0;continue;}
            if b==92 && quote==0 && cursor<line.len(){bytes[n]=line[cursor];cursor+=1;}
            else{bytes[n]=b;}n+=1;
        }
        if quote!=0{console.output.write_line(b"Unclosed quote.");return true;}
        spans[count]=(first,n);count+=1;
    }
    let mut args=[b"".as_slice();32];args[0]=b"/system/python";args[1]=path.as_bytes();
    for (i,&(a,b)) in spans[..count].iter().enumerate(){args[i+2]=&bytes[a..b];}
    if let Err(e)=unsafe{start(console,path.as_bytes(),&args[..count+2],None)} {console.output.write_line(e);}
    true
}

// ------------------------=
// FUNC: start_task
// DESC: Admits one native CPython process with a unique process identity while retaining immutable input snapshots between validation and launch.
// ------------------=
unsafe fn start_task(console: &ConsoleRuntime, job: &mut Job, arguments: &[&[u8]]) -> Result<(), &'static [u8]> {
    let generated_run=arguments.get(1)==Some(&b"-m".as_slice())
        && arguments.get(2)==Some(&b"_infinity_generated_run".as_slice());
    let application=!job.pip&&!arguments.get(1).is_some_and(|p|crate::runtime::native_python_scope::system_script(p))&&!arguments.contains(&b"-c".as_slice())
        && !arguments.get(1).is_some_and(|arg|matches!(*arg,b"--version"|b"-V"|b"--help"|b"-h"));
    if application&&crate::runtime::ai::application_host::context().is_some(){return Err(b"python: another native application is running; stop it before launching this app.");}
    if application && job.count > 0 && arguments.get(if generated_run {3}else{1}).copied() == Some(&job.files[0].path[..job.files[0].path_len]) {
        job.data.bind(&mut job.filesystem, &mut job.archive, console.current_user.0, console.current_session.0,
            job.files[0].object, job.files[0].version)
            .map_err(|_| b"python: cannot grant durable data for this source; no application was launched.".as_slice())?;
    }
    let Some((kernel, controller)) = crate::runtime::native_compile_job::platform() else {
        return Err(b"python: native process runtime unavailable.");
    };
    let now = crate::runtime::ai::qwen::workers::clock_ns();
    if now == 0 { return Err(b"python: native clock is unavailable."); }
    job.owner = SecurityIdentity(console.current_session.0);
    job.extensions.capture(&job.filesystem, crate::runtime::native_python_package::PYTHON,
        crate::runtime::native_cpu::MACHINE)?;
    let secret = crate::runtime::native_python_entropy::next(&console.current_session.0)
        .ok_or(b"python: validated boot entropy is unavailable.".as_slice())?;
    job.output.fill(0); job.errors.fill(0);
    job.exception=None;
    job.input=crate::runtime::native_console_input::Input::new();job.shown_output=0;job.shown_errors=0;job.drained_output=0;job.drained_errors=0;
    if job.pip { job.input.close(); }
    job.application=None;job.ui_revision=0;
    job.initial_pages = memory_mut().available_pages() as u64;
    let id = NEXT_TASK_ID;
    NEXT_TASK_ID = id.checked_add(1).ok_or(b"python: process identifiers exhausted.".as_slice())?;
    job.task = Some(Task::start(crate::runtime::native_python_package::PYTHON, arguments,
        job.owner, id, 1, memory_mut(), kernel, controller,
        if job.pip { now.saturating_add(600_000_000_000) }
        else if arguments.contains(&b"-c".as_slice()) { now.saturating_add(120_000_000_000) }
        else { u64::MAX }).map_err(|_| b"python: cannot allocate native process.".as_slice())?);
    job.task.as_mut().unwrap().set_launch_secret(secret);
    if application {
        match crate::runtime::ai::application_host::attach(console.current_user.0,console.current_session.0){
            Ok(context)=>job.application=Some(context),
            Err(_)=>{let _=job.task.take().unwrap().release(memory_mut());return Err(b"python: application authority could not be attached.");}
        }
    }
    let path=if job.pip {b"pip".as_slice()}else if job.count>0{&job.files[0].path[..job.files[0].path_len]}else{b"Python"};
    let name=path.rsplit(|b|*b==b'/').next().unwrap_or(path);
    MONITOR_RESUME_NS=0;
    job.monitor=crate::runtime::with_runtime(|r|r.task_manager.native_start(&mut r.execution,crate::runtime::task_manager::IMAGE_NATIVE_PYTHON,name.strip_suffix(b".py").unwrap_or(name)).ok()).flatten();
    if job.monitor.is_none(){if job.application.take().is_some(){crate::runtime::ai::application_host::detach();}
        let _=job.task.take().unwrap().release(memory_mut());return Err(b"python: no process slot available.");}
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
    let staged=panel.repair_attempts>0;
    if staged && !panel.source_binding_matches(metadata.object.id.0,metadata.current_version){return Err(b"The saved source changed.");}
    let generated = Generated { staged,goal, user: console.current_user.0, source: metadata.object.id.0,
        revision: console.assistant_revision(),
        request: crate::ui::app_assistant::fingerprint(&panel.request[..panel.request_len], 0, None),
        phase: workflow::Phase::Validate, launch:launch&&panel.permits_generated_launch(), launched:false, context:0,
        workflow_revision:crate::runtime::ai::with_ai_runtime(|ai|ai.software_execution(console.current_user.0)
            .filter(|execution|Some(execution.goal)==goal).map_or(0,|execution|execution.source_revision)) };
    unsafe {
        start_snapshot(console, path, &[b"/system/python", b"-c", workflow::VALIDATE_GENERATED_APPLICATION, path], Some(console.editor_document.bytes()),if staged{Some(&panel.argument[..panel.argument_len])}else{None})?;
        (&mut *(&raw mut JOB)).generated = Some(generated);
        (&mut *(&raw mut JOB)).recovery = recovery::Snapshot::new(recovery::Binding {
            user:console.current_user.0, session:console.current_session.0,
            editor_revision:generated.revision, goal, source_object:generated.source,
            source_version:metadata.current_version, workflow_source_revision:generated.workflow_revision, context:0,
        }, console.editor_document.bytes(), &panel.request[..panel.request_len], path);
        let request=&panel.request[..panel.request_len];
        if launch&&request.len()<=512&&crate::ui::app_assistant::editor_compile_intent::requests_opponent(request){
            let job=&mut *(&raw mut JOB);job.interaction_objective[..request.len()].copy_from_slice(request);job.interaction_length=request.len();
        }
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
    let mut generated=generated;
    use crate::ui::app_assistant as assistant;
    let mut panel = assistant::read(2);
    let exit = match result { State::Exited(code) => Some(code), _ => None };
    if generated.phase==workflow::Phase::Validate&&exit==Some(0)&&reclaimed{
        let source=&job.files[0];let path=&source.path[..source.path_len];
        if generated.staged {
            if !super::compiler_commands::publish_verified_repair(console,&mut panel,generated.revision,assistant::fingerprint(&source.bytes[..source.len],0,None)){
                generated_event(generated,Event::Failed{code:2});panel.generation=assistant::GenerationStatus::Failed;
                panel.reply(b"Repair validation finished, but the source changed. Nothing was launched.");assistant::write(2,panel);return false;
            }
            generated.revision=panel.document_revision;generated.staged=false;
            generated.workflow_revision=crate::runtime::ai::with_ai_runtime(|ai|ai.software_execution(console.current_user.0).filter(|e|Some(e.goal)==generated.goal).map_or(0,|e|e.source_revision));
            if let Some((object,version))=panel.source_binding{
                job.recovery=recovery::Snapshot::new(recovery::Binding{user:console.current_user.0,session:console.current_session.0,editor_revision:generated.revision,goal:generated.goal,source_object:object,source_version:version,workflow_source_revision:generated.workflow_revision,context:0},&source.bytes[..source.len],&panel.request[..panel.request_len],path);
            }
            assistant::write(2,panel);
        }
        if let Err(error)=console.register_created_application(path,path,assistant::fingerprint(&source.bytes[..source.len],0,None)){console.output.write_line(error);}
    }
    match workflow::outcome(generated.phase, exit, reclaimed, generated.launch, execution) {
        workflow::Outcome::Validated => {
            generated_event(generated, Event::Compiled { artifact: generated.source });
            panel.reply(b"Python source saved and syntax validated by the native interpreter. No application was launched.");
        }
        workflow::Outcome::Launch => {
            generated_event(generated, Event::Compiled { artifact: generated.source });
            let path = job.files[0].path;
            let length = job.files[0].path_len;
            match start_task(console, job, &[b"/system/python", b"-m", b"_infinity_generated_run", &path[..length]]) {
                Ok(()) => {
                    let context=job.application.map_or(0,|context|context.instance);
                    job.generated = Some(Generated { phase: workflow::Phase::Run, context, ..generated });
                    if let Some(snapshot)=job.recovery.as_mut(){snapshot.binding.context=context;}
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
            let _=context;
            if panel.workflow_goal!=generated.goal || generated.revision!=console.assistant_revision()
                || !console.assistant_owner_alive(2)
                || generated.request!=assistant::fingerprint(&panel.request[..panel.request_len],0,None) {
                return false;
            }
            panel.reply(b"Python application exited normally. Its output is available in Command; this does not verify every application behavior.");
        }
        workflow::Outcome::Failed => {
            if generated.phase==workflow::Phase::Run {
                return finish_runtime_failure(console,job,generated,result,reclaimed,error_length);
            }
            let request = panel.request;
            let request_length = panel.request_len;
            let repairing = workflow::repairable(generated.phase, exit, reclaimed)
                && panel.queue_compiler_repair(&request[..request_length], generated.revision,
                    console.assistant_revision(), &job.errors[..error_length]);
            if repairing {
                panel.reply(b"Python source validation found syntax, dependency, or incomplete-implementation issues. Repairing this unchanged saved source using its diagnostics (maximum two attempts). Nothing has launched.");
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
// FUNC: finish_runtime_failure
// DESC: Retains owner-bound runtime evidence and repairs only an unchanged source exception after reclamation, without replaying the failed application.
// ------------------=
unsafe fn finish_runtime_failure(console:&mut ConsoleRuntime,job:&mut Job,generated:Generated,
    result:State,reclaimed:bool,error_length:usize)->bool {
    use crate::ui::app_assistant as assistant;
    if matches!(result,State::Cancelled) {
        if let Some(goal)=generated.goal {
            crate::runtime::ai::with_ai_runtime(|ai|ai.cancel_generated_execution(goal,generated.user,
                generated.source,generated.workflow_revision,if generated.launched {generated.context}else{0}));
        }
    } else {
        generated_event(generated,Event::RuntimeFailed {source:generated.source,
            revision:generated.workflow_revision,context:generated.context,code:3});
    }
    let failure=match result {
        State::Exited(code)=>recovery::Failure::Exit(code),
        State::Faulted(reason,address,pc)=>recovery::Failure::Fault(reason,address,pc),
        State::Cancelled=>recovery::Failure::Cancelled,State::Deadline=>recovery::Failure::Deadline,
        State::Budget=>recovery::Failure::Budget,_=>recovery::Failure::Platform,
    };
    let mut diagnostics=[0u8;1024];
    let stderr=job.recovery.as_ref().map_or(&job.errors[..error_length],|snapshot|snapshot.stderr());
    let length=match job.exception {
        Some(exception)=>exception.diagnostics(stderr,&mut diagnostics),
        None=>{
            let n=stderr.len().min(diagnostics.len());
            diagnostics[..n].copy_from_slice(&stderr[stderr.len()-n..]);n
        }
    };
    if length>0 {(&mut *(&raw mut RUNTIME_DIAGNOSTICS)).record(job.owner.0,generated.revision,&diagnostics[..length]);}
    let mut panel=assistant::read(2);
    let Some(snapshot)=job.recovery.as_ref() else{return false;};
    let stored=crate::storage::object_inspect_path(snapshot.path()).ok()
        .filter(|(metadata,_)|metadata.owner.0==generated.user)
        .map(|(metadata,_)|(metadata.object.id.0,metadata.current_version));
    let current=recovery::Current {
        user:console.current_user.0,session:console.current_session.0,
        editor_revision:console.assistant_revision(),goal:panel.workflow_goal,
        source_binding:panel.source_binding.filter(|binding|Some(*binding)==stored),
        source:console.editor_document.bytes(),request:&panel.request[..panel.request_len],
        path:&console.editor_document_path[..console.editor_document_path_length],
        authorized:active(console),editor_alive:console.assistant_owner_alive(2),ai_allowed:console.ai_chat_allowed(),
        idle:crate::runtime::ai::with_ai_runtime(|ai|ai.app_turn_owner().is_none())
            && !matches!(crate::runtime::native_compile_job::status(job.owner),
                crate::runtime::native_compile_job::Status::Compile|crate::runtime::native_compile_job::Status::Link),
    };
    // Only a terminating, separately reported source exception is eligible.
    // Nonzero sys.exit, missing input, resource failures and interpreter faults
    // are not evidence that rewriting the program would be appropriate.
    let repairing=reclaimed && matches!(result,State::Exited(code) if code!=0)
        && job.exception.is_some_and(|evidence|evidence.repairable())
        && snapshot.can_repair(current,failure)
        && panel.queue_runtime_repair(snapshot.request(),generated.revision,console.assistant_revision(),&diagnostics[..length]);
    if repairing {
        panel.reply(b"The application raised a runtime exception. Repairing this unchanged saved source using its actual traceback, then validating it. Choose Run explicitly to try the repaired application; no actions will be replayed automatically.");
        assistant::write(2,panel);
        super::compiler_commands::generation_progress(console,super::compiler_commands::VoiceStep::Repairing);
    } else if panel.workflow_goal==generated.goal
        && generated.revision==console.assistant_revision() && console.assistant_owner_alive(2)
        && generated.request==assistant::fingerprint(&panel.request[..panel.request_len],0,None)
        && !matches!(panel.generation,assistant::GenerationStatus::Queued|assistant::GenerationStatus::Running) {
        panel.generation=if matches!(result,State::Cancelled){assistant::GenerationStatus::Cancelled}else{assistant::GenerationStatus::Failed};
        panel.reply(if length>0 {&diagnostics[..length]}else{b"Python execution stopped. The saved source is preserved; no automatic repair or restart was authorized."});
        assistant::write(2,panel);
    }
    false
}

// ------------------------=
// FUNC: exception_call
// DESC: Copies one diagnostic-only exception packet from the exact generated run and validates its source path and line without granting new authority.
// ------------------=
fn exception_call<T:crate::runtime::native_memory::PageTable>(
    evidence:&mut Option<crate::runtime::native_python_exception::Evidence>,generated:Option<Generated>,
    source:&Snapshot,tables:&mut T,operation:u64,arguments:[u64;6])->Option<i64> {
    use crate::runtime::native_python_exception as exception;
    if operation!=exception::OPERATION{return None;}
    if !generated.is_some_and(|g|g.phase==workflow::Phase::Run&&g.context!=0)
        || evidence.is_some() {return Some(-13);}
    if arguments[1]!=exception::RECORD_BYTES as u64 || arguments[2..].iter().any(|word|*word!=0){return Some(-22);}
    let mut bytes=[0u8;exception::RECORD_BYTES];
    if crate::runtime::native_memory::copy_from_user(tables,arguments[0],&mut bytes).is_err(){return Some(-14);}
    let Some(decoded)=exception::decode(&bytes,&source.path[..source.path_len],&source.bytes[..source.len])else{return Some(-22);};
    *evidence=Some(decoded);Some(0)
}

// ------------------------=
// FUNC: launch_infinitycode
// DESC: Launches the bundled designer through the existing Python process with a private owned source and durable data grant.
// ------------------=
pub(super) fn launch_infinitycode(console: &mut ConsoleRuntime) {
    let result = (|| -> Result<(), &'static [u8]> {
        if !active(console) { return Err(b"InfinityCode: sign in first."); }
        let home = crate::storage::provision_user_home(console.current_user.0)
            .map_err(|_| b"InfinityCode: personal storage is unavailable.".as_slice())?;
        let base = home.as_bytes();
        let suffix = b"/documents/InfinityCode.py";
        let mut path = [0u8; 95];
        let length = base.len() + suffix.len();
        if length > path.len() { return Err(b"InfinityCode: personal path is too long."); }
        path[..base.len()].copy_from_slice(base);
        path[base.len()..length].copy_from_slice(suffix);
        let path = &path[..length];
        let source = include_bytes!("../../sdk/python/examples/infinitycode.py");
        match crate::storage::object_inspect_path(path) {
            Ok(_) => {}, // Snapshot below rechecks ownership and exact bootstrap bytes.
            Err(crate::storage::object::ObjectError::NotFound | crate::storage::object::ObjectError::NamespaceNotFound) => {
                crate::storage::object_create_owned_note_at(b"InfinityCode.py", source, path,
                    crate::storage::object::ObjectId(console.current_user.0))
                    .map_err(|_| b"InfinityCode: cannot save its launch document.".as_slice())?;
            },
            Err(_) => return Err(b"InfinityCode: cannot inspect its launch document."),
        }
        unsafe { start(console, path, &[b"/system/python", path], Some(source)) }
    })();
    console.output.write_line(match result {
        Ok(()) => b"Launching InfinityCode. Projects are saved in its private /data directory.",
        Err(error) => error,
    });
}

// ------------------------=
// FUNC: command
// DESC: Accepts native Python script commands and cancellation without running user code inside input dispatch.
// ------------------=
pub(super) fn command(console: &mut ConsoleRuntime, input: &[u8]) -> bool { unsafe {
    if input.trim_ascii() == b"infinitycode" {
        launch_infinitycode(console);
        return true;
    }
    if input == b"python-cancel" {
        let job = &mut *(&raw mut JOB);
        if job.owner == SecurityIdentity(console.current_session.0) {
            job.input.close();
            if let Some(task) = job.task.as_mut() { task.cancel(); }
            job.child.cancel();
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
    let allow_network = !pip && count > 2 && args[1] == b"--allow-network";
    if allow_network { args.copy_within(2..count, 1); count -= 1; }
    if pip || count >= 3 && args[1] == b"-m" && args[2] == b"pip" {
        let result = start_pip(console, if pip { &args[1..count] } else { &args[3..count] });
        console.output.write_line(match result { Ok(()) => b"pip started: offline wheels, private per-user packages.", Err(error) => error });
        return true;
    }
    if count >= 3 && args[1] == b"-m" || count == 2 && matches!(args[1], b"--version" | b"-V") {
        let job = &mut *(&raw mut JOB);
        let result = if job.task.is_some() { Err(b"python: a script is already running; use python-cancel.".as_slice()) }
            else if !active(console) { Err(b"python: an authenticated session is required.".as_slice()) }
            else { prepare_packages(console, job, false).and_then(|_| {
                job.network.reset(SecurityIdentity(console.current_session.0), allow_network);
                start_task(console, job, &args[..count])
            }) };
        console.output.write_line(match result { Ok(()) => b"Python module started.", Err(error) => error });
        return true;
    }
    if count < 2 || args[1] == b"--help" {
        console.output.write_line(b"python /home/default/documents/hello.py [arguments]");
        console.output.write_line(b"python --allow-network /absolute/script.py grants native network access to this invocation only.");
        console.output.write_line(b"python-cancel stops your running script. Paths must be absolute.");
        console.output.write_line(b"pip install /absolute/package.whl; pip list; pip show NAME; pip uninstall -y NAME");
        return true;
    }
    match start(console, args[1], &args[..count], None) {
        Ok(()) => {
            (&mut *(&raw mut JOB)).network.reset(SecurityIdentity(console.current_session.0), allow_network);
            console.output.write_line(b"Python script started.");
        }
        Err(error) => console.output.write_line(error),
    }
    true
}}

// ------------------------=
// FUNC: editor_running
// DESC: Exposes actual Python process activity only to its initiating session.
// ------------------=
pub(super) fn editor_running(console:&ConsoleRuntime)->bool {unsafe{let job=&*(&raw const JOB);job.task.is_some()&&job.owner==SecurityIdentity(console.current_session.0)}}
// ------------------------=
// FUNC: input
// DESC: Delivers complete native stdin lines, EOF and cancellation while leaving the source editor untouched.
// ------------------=
pub(super) fn input(console:&mut ConsoleRuntime,key:super::ConsoleKey)->bool {unsafe {
    let job=&mut *(&raw mut JOB);
    if job.task.is_none()||job.owner!=SecurityIdentity(console.current_session.0)||job.pip{return false;}
    match key {
        super::ConsoleKey::Shortcut(b'c')=>{job.input.close();if let Some(task)=job.task.as_mut(){task.cancel();}job.child.cancel();console.reset_input();},
        super::ConsoleKey::Shortcut(b'd')=>job.input.close(),
        super::ConsoleKey::Enter=>{if job.input.submit(&console.command[..console.command_length]){console.output.write_line(&console.command[..console.command_length]);console.reset_input();}},
        _=>{console.edit_input(key);},
    }true
}}

// ------------------------=
// FUNC: terminal_read
// DESC: Marshals line-oriented stdin through checked user-memory ranges and reports terminal descriptors without granting ambient input.
// ------------------=
fn terminal_read<T:crate::runtime::native_memory::PageTable>(input:&mut crate::runtime::native_console_input::Input,tables:&mut T,operation:u64,a:[u64;6])->Option<i64> {
    use crate::runtime::native_memory::{check_user_range,copy_to_user};
    if operation==0x121d {return Some(if a[0]>2{-9}else{copy_to_user(tables,a[1],&1i32.to_le_bytes()).map_or(-14,|_|0)});}
    if operation!=0x121b{return None;}
    if a[0]!=0{return Some(-9);}
    if check_user_range(tables,a[3],8,true).is_err(){return Some(-14);}
    let read=input.read(a[2].min(1024)as usize,|bytes|copy_to_user(tables,a[1],bytes).is_ok());
    Some(if read<0{read}else{copy_to_user(tables,a[3],&(read as u64).to_le_bytes()).map_or(-14,|_|0)})
}

// ------------------------=
// FUNC: report_stream
// DESC: Publishes newly produced bytes while the native Python process is still running, with no replay at completion.
// ------------------=
fn report_stream(console:&mut ConsoleRuntime,bytes:&[u8]) {
    crate::ui::command_terminal::mutate(|s|{if s.authorized{s.append(bytes);}});
    crate::ui::editor_chrome::terminal::mutate(|state|state.append(bytes));
    for line in bytes.split(|b|*b==b'\n').filter(|line|!line.is_empty()) {
        // Keep the standalone command viewport current without mirroring the same bytes twice into the dock.
        if console.output.count==super::OUTPUT_ROWS {
            console.output.lines.copy_within(1..super::OUTPUT_ROWS,0);console.output.lengths.copy_within(1..super::OUTPUT_ROWS,0);console.output.count-=1;
        }
        let n=line.len().min(super::LINE_CAPACITY);let row=console.output.count;
        console.output.lines[row][..n].copy_from_slice(&line[..n]);console.output.lengths[row]=n;console.output.count+=1;
    }
}

// ------------------------=
// FUNC: poll
// DESC: Executes one native slice, reports output only to its owner, and reclaims all process and extension memory.
// ------------------=
pub(super) fn poll(console: &mut ConsoleRuntime) { unsafe {
    let job = &mut *(&raw mut JOB);
    if job.task.is_none() { return; }
    let session_current=job.owner == SecurityIdentity(console.current_session.0) && job.package_owner == console.current_user.0 && active(console);
    let authorized = session_current&&job.generated.is_none_or(|generated|workflow::authorized(generated.phase,generated.launched,
        generated_current(console,generated),generated.user==console.current_user.0&&console.ai_chat_allowed()));
    let monitor=job.monitor.and_then(|h|crate::runtime::with_runtime(|r|r.execution.get(h).map(|c|(c.state,c.budget))).flatten());
    let state=monitor.map(|v|v.0);
    let authorized=authorized && state!=Some(crate::runtime::execution::ContextState::Stopped);
    if authorized && state==Some(crate::runtime::execution::ContextState::Waiting){return;}
    let now = crate::runtime::ai::qwen::workers::clock_ns();
    if let Some((_, budget))=monitor {
        if authorized && budget.cpu_weight<100 && now<MONITOR_RESUME_NS{return;}
        memory_mut().monitor_limit(true,job.task.as_ref().unwrap().context_id(),budget.memory_limit);
    }
    if job.child.running() {
        job.child.poll(now, authorized, memory_mut(), &mut job.filesystem, &mut job.network);
        return;
    }
    let task = job.task.as_mut().unwrap();
    if authorized&&job.input.waiting&&now<task.deadline(){return;}
    let mut files: [File<'_>; 16] = core::array::from_fn(|_| File { path: b"", bytes: b"" });
    for (file, source) in files.iter_mut().zip(job.files.iter()).take(job.count) {
        *file = File { path: &source.path[..source.path_len], bytes: &source.bytes[..source.len] };
    }
    for index in 0..job.extensions.count() { files[job.count + index] = job.extensions.file(index).unwrap(); }
    let file_count = job.count + job.extensions.count();
    let native_owner = crate::ui::window::ContextId(task.context_id());
    let parent_deadline = task.deadline();
    let display = crate::ui::geometry::Rect {x:0,y:0,width:console.system.framebuffer_width as u32,height:console.system.framebuffer_height as u32};
    let scale = crate::ui::system_layout::SystemLayout::new(display.width as usize, display.height as usize).scale();
    let work = crate::runtime::native_gui::client_work_area(display,scale);
    let gui_allowed = authorized && !job.pip && job.generated.is_none_or(|g|g.phase == workflow::Phase::Run);
    let result = task.poll_with_services(now, authorized, memory_mut(),
        b"", &files[..file_count], &mut job.output, &mut job.errors,
        |memory, tables, operation, arguments| crate::runtime::with_runtime(|runtime|job.gui.dispatch(runtime,memory,tables,job.owner,native_owner,gui_allowed,operation,arguments,work,display,now)).flatten()
            .or_else(||exception_call(&mut job.exception,job.generated,&job.files[0],tables,operation,arguments))
            .or_else(|| application_call(&mut job.application,job.pip,console.current_user.0,console.current_session.0,tables,operation,arguments))
            .or_else(||terminal_read(&mut job.input,tables,operation,arguments))
            .or_else(||job.child.syscall(memory, tables, operation, arguments, &mut job.filesystem,
                job.owner, job.pip, &mut *(&raw mut NEXT_TASK_ID), now, parent_deadline).map(|result| {
                    if operation == 0x1280 && result >= 0 { job.network.close(); }
                    result
                }))
            .or_else(||job.network.syscall(tables, operation, arguments))
            .or_else(|| job.data.syscall(&mut job.filesystem, &mut job.archive, tables, operation, arguments)));
    if let Some(h)=job.monitor {
        let timestamp=crate::runtime::ai::qwen::workers::clock_ns();
        let busy=timestamp.saturating_sub(now);
        MONITOR_RESUME_NS=timestamp.saturating_add(crate::runtime::task_manager::TaskManager::native_cooldown_ns(busy,monitor.map_or(100,|v|v.1.cpu_weight)));
        let sample=timestamp.saturating_sub(MONITOR_SAMPLE_NS)>=1_000_000_000;
        let bytes=if sample{MONITOR_SAMPLE_NS=timestamp;Some(memory_mut().resident_bytes(native_owner.0))}else{None};
        crate::runtime::with_runtime(|r|{if let Some(c)=r.execution.get_mut(h){c.usage.timed=true;c.usage.busy_ns=c.usage.busy_ns.saturating_add(busy);if let Some(n)=bytes{c.usage.memory_bytes=n;}}});
    }
    let progress=task.result_lengths(b"",&mut job.output,&mut job.errors);
    if job.generated.is_some_and(|g|g.phase==workflow::Phase::Run) {
        if let (Some((_,stderr,_)),Some(snapshot))=(progress,job.recovery.as_mut()) {
            snapshot.record_stderr(&job.errors[job.shown_errors.min(stderr)..stderr]);
        }
    }
    let mut changed=false;
    if job.gui_revision != job.gui.revision() {
        job.gui_revision = job.gui.revision(); job.gui_keyboard = true; changed = true;
    }
    if authorized&&task.execution_id().is_some(){
        if let Some(mut generated)=job.generated.filter(|g|g.phase==workflow::Phase::Run&&!g.launched){
            if let Some(context)=job.application{
                generated_event(generated,Event::Launched{context:context.instance});
                generated.launched=true;job.generated=Some(generated);
                let mut panel=crate::ui::app_assistant::read(2);
                panel.reply(b"Python application is running in its native process. Its source can now be edited or closed independently.");crate::ui::app_assistant::write(2,panel);
                changed=true;
            }
        }
    }
    if authorized && job.interaction_length>0 && matches!(result,State::Runnable)
        && job.application.is_some_and(crate::runtime::ai::application_host::published) {
        if crate::runtime::ai::with_ai_runtime(|ai|ai.start_application_agent(console.current_user.0,console.current_session.0,
            &job.interaction_objective[..job.interaction_length],true)).is_ok(){job.interaction_length=0;}
    }
    if job.application.is_some(){let ui=crate::ui::native_uikit::presentation();if ui.revision!=job.ui_revision{
        if ui.active&&!console.command_window.visible{console.open_command_window();}
        job.ui_revision=ui.revision;changed=true;
    }}
    if authorized {if let Some((stdout,stderr,_))=progress {
        if stdout>job.shown_output{report_stream(console,&job.output[job.shown_output..stdout]);job.shown_output=stdout;changed=true;}
        if stderr>job.shown_errors{report_stream(console,&job.errors[job.shown_errors..stderr]);job.shown_errors=stderr;changed=true;}
    }}
    if matches!(result, State::Runnable) {
        if job.generated.is_some_and(|g|g.phase==workflow::Phase::Run) && job.recovery.is_some(){
            job.drained_errors=job.drained_errors.saturating_add(job.shown_errors);
            task.drain_errors();job.shown_errors=0;
        }
        if !job.pip && job.shown_output>0 {job.drained_output=job.drained_output.saturating_add(job.shown_output);task.drain_output();job.shown_output=0;}
        if changed{console.redraw();}return;
    }
    let execution = task.execution_id();
    let lengths = task.result_lengths(b"", &mut job.output, &mut job.errors);
    if authorized {
        if let Some((_length, _errors, truncated)) = lengths {
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
    memory_mut().monitor_limit(true,native_owner.0,u64::MAX);
    if let Some(h)=job.monitor.take(){crate::runtime::with_runtime(|r|{let _=r.task_manager.end(&mut r.execution,h);if let Some(c)=r.execution.get_mut(h){c.usage.memory_bytes=0;}});}
    let cleanup = job.task.take().unwrap().release(memory_mut()).is_ok();
    if let Some(path)=job.tool_export.take() {
        if authorized && cleanup && result==State::Exited(0) {
            let saved=export_allowed(console,path.as_bytes()) && job.filesystem.read(b"/tmp/figlet.txt").ok().is_some_and(|bytes|{
                let name=path.as_bytes().rsplit(|b|*b==b'/').next().unwrap_or(&[]);
                crate::storage::object_create_owned_note_at(name,bytes,path.as_bytes(),crate::storage::object::ObjectId(console.current_user.0)).is_ok()
            });
            if saved {console.output.write_segments(&[b"Saved ",path.as_bytes()]);}
            else {console.output.write_line(b"Could not create output object; existing files are never overwritten.");}
        }
    }
    let _ = crate::runtime::with_runtime(|runtime|job.gui.close(runtime,memory_mut()));
    job.gui_keyboard = false;
    if let Some(owned)=job.application.take(){
        if crate::runtime::ai::application_host::context().is_some_and(|live|live.instance==owned.instance){
            crate::runtime::ai::application_host::detach();crate::ui::native_uikit::close();
        }
    }
    job.network.reset(job.owner, false);
    let data_committed = !(authorized && cleanup && result == State::Exited(0))
        || job.data.checkpoint(&mut job.filesystem, &mut job.archive).is_ok();
    if !data_committed { console.output.write_line(b"python: application data could not be saved; the previous durable checkpoint remains intact."); }
    if !cleanup && authorized {
        console.output.write_line(b"python: process cleanup failed.");
    }
    let mut packages_committed = data_committed;
    let mut package_incompatible = false;
    if job.pip && authorized && cleanup && result == State::Exited(0) && job.filesystem.dirty() {
        package_incompatible = !extensions::Snapshots::validate(&job.filesystem,
            crate::runtime::native_python_package::PYTHON, crate::runtime::native_cpu::MACHINE);
        packages_committed = !package_incompatible
            && job.filesystem.encode(&mut job.archive).ok().is_some_and(|length|
            crate::storage::python_packages_save(job.package_owner, console.current_session.0,
                job.package_revision, &job.archive[..length]).is_ok());
        console.output.write_line(if packages_committed { b"pip: package changes saved for this user." }
            else { b"pip: native extension incompatibility, quota, session, version or storage prevented publication. Previous packages remain unchanged." });
    } else if job.pip && job.filesystem.dirty() && authorized {
        console.output.write_line(b"pip: unsuccessful operation discarded; installed packages remain unchanged.");
    }
    if job.pip && authorized {
        console.output.write_line(match result {
            State::Exited(0) if cleanup && packages_committed => b"pip completed successfully.",
            State::Cancelled => b"pip cancelled; no package changes were committed.",
            State::Deadline => b"pip exceeded its ten-minute limit; no package changes were committed.",
            _ => b"pip did not complete successfully; previous installed packages remain unchanged.",
        });
    }
    let state = if package_incompatible { 5 } else { match result {
        State::Exited(_) if cleanup && packages_committed => 2, State::Cancelled => 3, _ => 4 } };
    let exit = if package_incompatible { 65 } else { match result { State::Exited(code) if packages_committed => code, _ => -1 } };
    let (stdout, stderr, _) = lengths.unwrap_or((0, 0, false));
    publish(state, exit, stdout.saturating_add(job.drained_output), stderr.saturating_add(job.drained_errors), job.initial_pages, memory_mut().available_pages() as u64);
    if let Some(generated) = job.generated {
        if authorized {
            if finish_generated(console, job, generated, result, cleanup&&data_committed, execution, stderr) {
                console.redraw();
                return;
            }
        } else {
            // A stale result may retire only its old goal; it never rewrites the current editor or panel.
            generated_event(generated, Event::Failed { code: 4 });
        }
    }
    job.generated = None;
    job.recovery=None;job.exception=None;
    job.count = 0;
    job.pip = false; job.archive.fill(0); job.filesystem.reset(false); job.extensions.clear(); job.child.clear();
    job.data = app_data::Data::new();
    console.redraw();
}}

// ------------------------=
// FUNC: application_call
// DESC: Binds genuine Python UIKit and action hooks to one session-owned app instance without stealing another application's registry.
// ------------------=
fn application_call<T:crate::runtime::native_memory::PageTable>(application:&mut Option<crate::runtime::ai::application_host::Context>,pip:bool,user:[u8;16],session:[u8;16],tables:&mut T,operation:u64,arguments:[u64;6])->Option<i64>{
    if !matches!(operation,0x1140..=0x1142|0x1150..=0x1156){return None;}
    if pip{return Some(-13);}
    #[cfg(feature="native-compiler")]
    {
        if application.is_none(){
            if crate::runtime::ai::application_host::context().is_some(){return Some(-16);}
            *application=match crate::runtime::ai::application_host::attach(user,session){Ok(context)=>Some(context),Err(_)=>return Some(-13)};
        }
        if crate::runtime::ai::application_host::context().is_none_or(|live|application.is_none_or(|owned|owned.instance!=live.instance||live.owner!=user||live.session!=session)){return Some(-13);}
        return Some(if operation<=0x1142{super::compiler_commands::native_uikit_syscall(tables,operation,arguments)}
            else{super::compiler_commands::native_interaction_syscall(tables,operation,arguments,session)});
    }
    #[cfg(not(feature="native-compiler"))]
    Some(-88)
}

// ------------------------=
// FUNC: release_gui_keyboard
// DESC: Returns keyboard routing to OS shortcuts and secure overlays without destroying the running native application.
// ------------------=
pub(super) fn release_gui_keyboard() { unsafe { (&mut *(&raw mut JOB)).gui_keyboard=false; } }

// ------------------------=
// FUNC: gui_frames
// DESC: Borrows committed context-owned pixels only during the serialized desktop render pass, in authoritative native window order.
// ------------------=
pub(super) fn gui_frames(mut render: impl FnMut(crate::ui::window::Window,crate::ui::compositor::SurfaceFrame<'_>,bool)) {
    unsafe {
        let job = &*(&raw const JOB);
        let _ = crate::runtime::with_runtime(|runtime| {
            publish_gui(runtime,&job.gui);
            let Some(owner)=job.gui.context() else{return;};
            if runtime.ui.windows.focused()!=job.gui.assistant_target() {
                let mut panel=crate::ui::app_assistant::read(crate::ui::app_assistant::NATIVE_OWNER);
                panel.expanded=false;panel.focused=false;panel.pending=crate::ui::app_assistant::Action::None;
                crate::ui::app_assistant::write(crate::ui::app_assistant::NATIVE_OWNER,panel);
            }
            for index in 0..runtime.ui.windows.count() {
                if let Some(window) = runtime.ui.windows.ordered_window(index).filter(|w|w.owner==owner) {
                    if let Some(frame) = job.gui.surfaces.frame(runtime,window.surface) {
                        if frame.visible || window.state==crate::ui::window::WindowState::Minimized {
                            render(*window,frame,runtime.ui.windows.focused()==Some(window.id));
                        }
                    }
                }
            }
        });
    }
}

// ------------------------=
// FUNC: gui_input
// DESC: Routes real normalized desktop input only to the currently authenticated GUI context; clicks outside release keyboard routing.
// ------------------=
pub(super) fn gui_input(console:&ConsoleRuntime,event:crate::ui::input::UiEvent<'_>,scroll_y:i32)->bool {
    unsafe {
        let job=&mut *(&raw mut JOB);
        if job.task.is_none() || job.owner!=SecurityIdentity(console.current_session.0) || !active(console) { return false; }
        let Some(owner)=job.gui.context() else {return false;};
        let now=crate::runtime::ai::qwen::workers::clock_ns();
        let previous_target=job.gui.assistant_target();
        let handled=crate::runtime::with_runtime(|runtime|{
            if let crate::ui::input::UiEvent::Pointer{position,phase,button}=event {
                let display=crate::ui::geometry::Rect{x:0,y:0,width:console.system.framebuffer_width as u32,height:console.system.framebuffer_height as u32};
                let scale=crate::ui::system_layout::SystemLayout::new(display.width as usize,display.height as usize).scale();
                let (chrome,target)=job.gui.desktop_pointer(runtime,position,phase,button,display,scale,now);
                if phase==crate::ui::input::PointerPhase::Down {
                    job.gui_keyboard=target.is_some();
                }
                if chrome{return true;}
                if target.is_none(){return false;}
            }else if !job.gui_keyboard || crate::ui::app_assistant::read(crate::ui::app_assistant::NATIVE_OWNER).focused{return false;}
            runtime.native_input(event,0,scroll_y,now)
        }).unwrap_or(false);
        if previous_target!=job.gui.assistant_target() {
            // Never deliver a former window's turn to a newly selected native window.
            crate::runtime::ai::with_ai_runtime(|ai| {
                if ai.app_turn_owner()==Some(crate::ui::app_assistant::NATIVE_OWNER) {ai.cancel_app_turn();}
            });
        }
        handled
    }
}

// ------------------------=
// FUNC: gui_assistant_owner
// DESC: Resolves the authenticated focused application window without exposing its client pixels or entering another app context.
// ------------------=
pub(super) fn gui_assistant_owner(console:&ConsoleRuntime)->Option<crate::ui::geometry::Rect> {
    unsafe {
        let job=&*(&raw const JOB);
        if job.task.is_none() || job.owner!=SecurityIdentity(console.current_session.0) || !active(console) {return None;}
        let owner=job.gui.context()?;
        let scale=crate::ui::system_layout::SystemLayout::new(console.system.framebuffer_width,console.system.framebuffer_height).scale();
        crate::runtime::with_runtime(|runtime| {
            let focused=runtime.ui.windows.focused()?;
            if job.gui.assistant_target()!=Some(focused) || !crate::ui::app_assistant::expanded(crate::ui::app_assistant::NATIVE_OWNER) {return None;}
            let window=*runtime.ui.windows.inspect(focused)?;
            if window.owner!=owner {return None;}
            crate::runtime::native_gui::assistant_bounds(window,scale)
        }).flatten()
    }
}

// ------------------------=
// FUNC: gui_control
// DESC: Resolves one owned native app window by title or active focus and delivers ordinary modal-safe lifecycle events, never killing its task.
// ------------------=
pub(super) fn gui_control(console: &ConsoleRuntime, name: Option<&[u8]>, action: crate::ui::window::WindowControl) -> Option<bool> {
    unsafe {
        let job = &mut *(&raw mut JOB);
        if job.task.is_none() || job.owner != SecurityIdentity(console.current_session.0) || !active(console) { return None; }
        let owner = job.gui.context()?;
        crate::runtime::with_runtime(|runtime| {
            let mut selected = None;
            for index in 0..runtime.ui.windows.count() {
                let Some(window) = runtime.ui.windows.ordered_window(index) else { continue; };
                if window.owner != owner { continue; }
                let matches = if let Some(name) = name {
                    let title = window.title();
                    title.as_bytes().eq_ignore_ascii_case(name)
                        || title.split(" — ").next().is_some_and(|base| base.as_bytes().eq_ignore_ascii_case(name))
                } else { runtime.ui.windows.focused() == Some(window.id) && (job.gui_keyboard
                    || (job.gui.assistant_target()==Some(window.id)
                        && crate::ui::app_assistant::expanded(crate::ui::app_assistant::NATIVE_OWNER))) };
                if matches {
                    if selected.is_some() { return Some(false); }
                    selected = Some(window.id);
                }
            }
            let id = selected?;
            let display = crate::ui::geometry::Rect { x: 0, y: 0,
                width: console.system.framebuffer_width as u32, height: console.system.framebuffer_height as u32 };
            let scale = crate::ui::system_layout::SystemLayout::new(display.width as usize, display.height as usize).scale();
            let work = crate::runtime::native_gui::client_work_area(display, scale);
            let success = runtime.ui.windows.control_owned(owner, id, action, work, display).is_ok();
            if success {
                runtime.flush_window_events(crate::runtime::ai::qwen::workers::clock_ns(), id.0 as u64, id.0 as u64);
                job.gui_keyboard = action != crate::ui::window::WindowControl::Minimize;
            }
            Some(success)
        }).flatten()
    }
}
