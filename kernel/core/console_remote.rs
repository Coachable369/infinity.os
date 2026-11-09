//! Authenticated SSH shells use the same command runtime with private output and navigation.
use super::*;
use crate::runtime::console_execution::{Diagnostic, Outcome};
use crate::runtime::console_remote::{self as transport, Binding, Job, JobKind, Reply};
use crate::ui::terminal_edit as edit;
#[path="console_remote_keys.rs"]mod keys;
#[path="console_remote_screen.rs"]mod screen;

struct Shell {
    binding: Binding,
    console: ConsoleRuntime,
    sequence: u64,
    foreground: bool,
    pending: Option<Reply>,
    columns:usize,
    rows:usize,
    editor_screen:bool,
    keys:keys::Decoder,
    read_input:bool,
    deferred:Option<Job>,
}
static mut SHELLS: [Option<Shell>; 4] = [const { None }; 4];
static mut OWNERS: [[u8;16];4] = [[0;16];4];

// ------------------------=
// FUNC: foreign_owner
// DESC: Routes an existing native job only to its owning shell while preserving local logout cancellation.
// ------------------=
pub(super) fn foreign_owner(owner: [u8;16], console: &ConsoleRuntime) -> bool {
    if owner == console.current_session.0 { return false; }
    console.output.remote.is_some() || unsafe { (&*(&raw const OWNERS)).contains(&owner) }
}

impl Shell {
    // ------------------------=
    // FUNC: authorized
    // DESC: Revalidates the authenticated account, credential and session before every dispatch and output delivery.
    // ------------------=
    fn authorized(&self)->bool {
        transport::active(self.binding) && crate::runtime::with_runtime(|runtime|
            runtime.identity.console_session_authorized(self.binding.session,self.binding.user)).unwrap_or(false)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Creates a separate command state bound to the SSH account's own home and authenticated session.
    // ------------------=
    fn new(binding: Binding, system: SystemSnapshot) -> Option<Self> {
        if !transport::active(binding) { return None; }
        let home = crate::storage::provision_user_home(binding.user.0).ok()?;
        let mut console = ConsoleRuntime::new(system);
        console.current_user = binding.user;
        console.current_session = binding.session;
        console.workplace_session = binding.session.0;
        console.mode = ConsoleMode::Console;
        console.navigation_context = crate::runtime::object_navigation::ConsoleNavigationContext::new(
            binding.session.short(), binding.user.short(), home.as_bytes()).ok()?;
        console.output.remote = Some(Outcome::new());
        Some(Self { binding, console, sequence: 0, foreground: false, pending: None,
            columns:80,rows:24,editor_screen:false,keys:keys::Decoder::new(),read_input:false,deferred:None })
    }

    // ------------------------=
    // FUNC: busy
    // DESC: Keeps exec channels open for actual asynchronous work and private continuation prompts.
    // ------------------=
    fn busy(&self) -> bool {
        if self.console.log_reader.active{return true;}
        if edit::inspect(|state|state.active) || self.console.account_password.active() || self.console.ssh_terminal.active
            || self.console.remote_llm_waiting() || geturl::pending(&self.console) { return true; }
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if python_commands::editor_running(&self.console) { return true; }
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if compiler_commands::remote_busy(&self.console) { return true; }
        false
    }

    // ------------------------=
    // FUNC: waiting_input
    // DESC: Reports native stdin readiness independently of whether a program printed a prompt.
    // ------------------=
    fn waiting_input(&self)->bool {
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if python_commands::waiting_input(&self.console){return true;}
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if compiler_commands::waiting_input(&self.console){return true;}
        false
    }
    // ------------------------=
    // FUNC: native_busy
    // DESC: Identifies a foreground process so pasted stdin is retained until its single-line input buffer is ready.
    // ------------------=
    fn native_busy(&self)->bool {
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if python_commands::editor_running(&self.console){return true;}
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if compiler_commands::remote_busy(&self.console){return true;}
        false
    }

    // ------------------------=
    // FUNC: interrupt
    // DESC: Cancels only work owned by this shell, erasing password drafts before releasing a connection.
    // ------------------=
    fn interrupt(&mut self) {
        self.console.log_reader.clear();
        self.deferred=None;
        self.keys=keys::Decoder::new();
        self.console.cancel_account_password();
        self.console.cancel_remote_llm();
        if self.console.ssh_terminal.active { self.console.ssh_terminal_cancel(); }
        if edit::inspect(|state|state.active) { self.console.terminal_edit_key(ConsoleKey::Escape); }
        geturl::cancel(&self.console);
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        { python_commands::input(&mut self.console, ConsoleKey::Shortcut(b'c')); }
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        { compiler_commands::remote_cancel(&mut self.console); }
        self.console.reset_input();
    }

    // ------------------------=
    // FUNC: command
    // DESC: Dispatches complete command or private prompt input without exposing secrets to shell history.
    // ------------------=
    fn command(&mut self, bytes: &[u8]) {
        if self.console.account_password.active() {
            for byte in bytes { self.console.account_password_input(ConsoleKey::Character(*byte)); }
            self.console.account_password_input(ConsoleKey::Enter);
            self.console.output.write_line(b"");
            return;
        }
        if self.console.ssh_terminal.active {
            for byte in bytes { self.console.ssh_terminal_input(ConsoleKey::Character(*byte)); }
            self.console.ssh_terminal_input(ConsoleKey::Enter);
            return;
        }
        self.console.reset_input();
        if bytes.len() > self.console.command.len() {
            self.console.output.fail(Diagnostic::Capacity);
            self.console.output.write_line(b"Command input exceeds 512 bytes.");
            return;
        }
        self.console.command[..bytes.len()].copy_from_slice(bytes);
        self.console.command_length = bytes.len();
        self.console.command_cursor = bytes.len();
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if python_commands::input(&mut self.console, ConsoleKey::Enter) { return; }
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if compiler_commands::input(&mut self.console, ConsoleKey::Enter) { return; }
        self.console.execute_input();
    }

    // ------------------------=
    // FUNC: raw_input
    // DESC: Decodes bounded standard terminal keys for the same native editor and outbound SSH controllers.
    // ------------------=
    fn raw_input(&mut self,bytes:&[u8]) {
        if self.console.remote_ssh_connected(){let _=self.console.remote_ssh_input(bytes);return;}
        let now=crate::runtime::ai::qwen::workers::clock_ns()/1_000_000;
        for &byte in bytes {for key in self.keys.feed(byte,now).0.into_iter().flatten(){
            if self.console.log_reader.active{self.console.log_reader_key(key);}
            else if edit::inspect(|state|state.active){self.console.terminal_edit_key(key);}
            else if self.console.ssh_terminal.active{self.console.ssh_terminal_input(key);}
        }}
    }

    // ------------------------=
    // FUNC: render_editor
    // DESC: Projects the private native document into a bounded ANSI terminal without changing the desktop editor.
    // ------------------=
    fn render_editor(&mut self) {
        if !edit::inspect(|state|state.active) {
            if self.editor_screen {self.console.output.remote.as_mut().unwrap().stdout.write(b"\x1b[?1049l\x1b[?25h\n");self.editor_screen=false;}
            return;
        }
        let columns=self.columns.clamp(20,160);let rows=self.rows.clamp(6,48);let body=rows-4;
        let output=&mut self.console.output.remote.as_mut().unwrap().stdout;
        if !self.editor_screen{output.write(b"\x1b[?1049h");self.editor_screen=true;}
        output.write(b"\x1b[?25l\x1b[H\x1b[1;36mInfinity Editor  ");
        edit::mutate(|state|{
            let(cursor_row,cursor_column)=screen::position(state.document.bytes(),state.document.cursor(),columns);
            if cursor_row<state.top{state.top=cursor_row;}else if cursor_row>=state.top+body{state.top=cursor_row+1-body;}
            output.write(&state.path[..state.path_len.min(columns.saturating_sub(18))]);
            output.write(b"\x1b[0m\x1b[K\r\n\x1b[K\r\n");
            let bytes=state.document.bytes();
            let mut cells=[b' ';160*44];let _=screen::cells(bytes,state.top,columns,body,&mut cells);
            for row in 0..body {
                output.write(&cells[row*columns..(row+1)*columns]);
                output.write(b"\x1b[K\r\n");
            }
            let footer=b"^S Save  ^X Exit  ^W Find  ^G Line";
            output.write(b"\x1b[1;36m");output.write(&footer[..footer.len().min(columns)]);output.write(b"\x1b[0m\x1b[K\r\n");
            output.write(&state.notice[..state.notice.len().min(columns)]);output.write(b"\x1b[K");
            if matches!(state.prompt,2|3){
                cursor(output,rows,1);output.write(if state.prompt==2{b"Find: "}else{b"Line: "});
                output.write(&state.query[..state.query_len.min(columns-6)]);
            }else{
                cursor(output,3+cursor_row.saturating_sub(state.top),cursor_column+1);
            }
        });
        output.write(b"\x1b[?25h");
    }

    // ------------------------=
    // FUNC: process
    // DESC: Executes one ordered authenticated terminal request and returns its exact completion sequence.
    // ------------------=
    fn process(&mut self,job:Job) {
        if matches!(job.kind,JobKind::Command)&&self.native_busy()&&!self.waiting_input(){self.deferred=Some(job);return;}
        self.sequence=job.sequence;self.columns=usize::from(job.columns);self.rows=usize::from(job.rows);
        self.console.remote_dimensions=Some((self.columns,self.rows));
        match job.kind {
            JobKind::Command=>self.command(&job.input[..job.length]),
            JobKind::Input=>self.raw_input(&job.input[..job.length]),
            JobKind::Interrupt=>{self.interrupt();self.console.output.fail(Diagnostic::Cancelled);self.console.output.write_line(b"Command interrupted.");},
            JobKind::Closed=>self.interrupt(),
            JobKind::Eof=>{
                self.console.log_reader.clear();
                if self.console.remote_ssh_connected(){let _=self.console.remote_ssh_eof();}
                if self.console.account_password.active(){self.console.cancel_account_password();self.console.output.fail(Diagnostic::Cancelled);}
                if edit::inspect(|state|state.active){edit::session([0;16]);self.console.output.fail(Diagnostic::Cancelled);self.console.output.write_line(b"Input closed; unsaved editor changes were discarded.");}
                #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {python_commands::input(&mut self.console,ConsoleKey::Shortcut(b'd'));}
                #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {compiler_commands::input(&mut self.console,ConsoleKey::Shortcut(b'd'));}
            },
        }
        if self.console.remote_exit{self.interrupt();}
        self.render_editor();
        if !matches!(job.kind,JobKind::Closed){self.flush(true);}
    }

    // ------------------------=
    // FUNC: poll_work
    // DESC: Advances bounded owned native work and collects output without painting the local desktop.
    // ------------------=
    fn poll_work(&mut self) {
        if let Some(key)=self.keys.expire(crate::runtime::ai::qwen::workers::clock_ns()/1_000_000){
            if self.console.log_reader.active{self.console.log_reader_key(key);}
            else if edit::inspect(|state|state.active){self.console.terminal_edit_key(key);}
            else if self.console.ssh_terminal.active{self.console.ssh_terminal_input(key);}
            self.render_editor();
        }
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        python_commands::poll(&mut self.console);
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        compiler_commands::poll_remote(&mut self.console);
        geturl::poll(&mut self.console);
        self.console.ssh_terminal_poll();
        self.console.poll_log_reader();
        if !edit::inspect(|state|state.active) { self.console.poll_remote_llm(); }
        if self.deferred.is_some() && (!self.native_busy()||self.waiting_input()) {
            let job=self.deferred.take().unwrap();self.process(job);
        }
    }

    // ------------------------=
    // FUNC: flush
    // DESC: Returns typed completion and private prompt state through bounded SSH backpressure.
    // ------------------=
    fn flush(&mut self, force: bool) {
        if let Some(reply) = self.pending.take() {
            if let Err(reply) = transport::reply(self.binding, reply) { self.pending = Some(reply); return; }
        }
        let more = !self.console.remote_exit&&self.busy();
        let read_input=self.waiting_input();
        let output = self.console.output.remote.as_ref().unwrap();
        if !force && output.stdout.bytes().is_empty() && output.stderr.bytes().is_empty()
            && !(self.foreground && !more) && self.read_input==read_input { return; }
        let mut outcome = self.console.output.remote.replace(Outcome::new()).unwrap();
        if outcome.stdout.truncated() || outcome.stderr.truncated() {
            outcome.status = 75; outcome.diagnostic = Diagnostic::Capacity;
            outcome.stderr.write_line(b"Output exceeded the bounded channel buffer; narrow the query.");
        }
        if self.foreground && !more && !force { outcome.stdout.write_line(b""); }
        let reply = Reply { sequence: self.sequence, outcome, more,
            secret: self.console.account_password.active() || self.console.ssh_terminal.active,
            private: self.console.command_history.private(),
            clear_history:core::mem::take(&mut self.console.remote_history_clear),
            close:self.console.remote_exit,
            read_input,
            raw:edit::inspect(|state|state.active)||self.console.remote_ssh_connected()||self.console.log_reader.active };
        self.foreground = more;
        self.read_input=read_input;
        if let Err(reply) = transport::reply(self.binding, reply) { self.pending = Some(reply); }
    }
}

// ------------------------=
// FUNC: poll
// DESC: Runs SSH commands outside network/runtime borrows and revalidates every shell before work or delivery.
// ------------------=
pub(super) fn poll() {
    let Some(system) = (unsafe { (&*(&raw const RUNTIME)).as_ref().map(|runtime|runtime.system) }) else { return; };
    unsafe {
        let shells = &mut *(&raw mut SHELLS);
        let ready=core::array::from_fn(|slot|shells[slot].as_ref().is_none_or(|shell|shell.pending.is_none()&&shell.deferred.is_none()
            &&(!shell.console.remote_ssh_connected()||shell.console.remote_ssh_input_capacity()>=64)));
        if let Some(job) = transport::take_ready_job(ready) {
            let slot = job.binding.slot as usize;
            if slot < shells.len() {
                let _surface=edit::select_remote(slot);
                let changed = shells[slot].as_ref().is_some_and(|shell| shell.binding != job.binding);
                if changed { if let Some(shell) = shells[slot].as_mut() { shell.interrupt(); shell.poll_work(); } shells[slot] = None; }
                if !matches!(job.kind, JobKind::Closed) && shells[slot].is_none() {
                    shells[slot] = Shell::new(job.binding, system);
                    if shells[slot].is_some() { (&mut *(&raw mut OWNERS))[slot] = job.binding.session.0; }
                }
                if let Some(shell) = shells[slot].as_mut() {
                    if matches!(job.kind,JobKind::Interrupt|JobKind::Closed){
                        shell.pending=None;shell.console.output.remote=Some(Outcome::new());
                    }
                    if matches!(job.kind,JobKind::Closed){shell.interrupt();edit::session([0;16]);}
                    else if !shell.authorized(){shell.interrupt();}
                    else{shell.process(job);}
                } else if !matches!(job.kind, JobKind::Closed) {
                    let _ = transport::reply(job.binding, Reply { sequence:job.sequence,
                        outcome:Outcome::failure(Diagnostic::Unavailable),more:false,secret:false,private:false,raw:false,clear_history:false,close:false,read_input:false });
                }
            }
        }
        for (slot, shell) in shells.iter_mut().enumerate() {
            let Some(current) = shell.as_mut() else { continue; };
            let _surface=edit::select_remote(slot);
            if !current.authorized() {
                current.interrupt();edit::session([0;16]);current.poll_work();
                if !current.busy() { *shell = None; (&mut *(&raw mut OWNERS))[slot] = [0;16]; }
                continue;
            }
            current.flush(false);
            if current.pending.is_none() { current.poll_work(); }
            current.flush(false);
        }
    }
}

// ------------------------=
// FUNC: cursor
// DESC: Encodes an ANSI cursor position as bounded decimal fields without allocation.
// ------------------=
fn cursor(output:&mut crate::runtime::console_execution::Bytes<{crate::runtime::console_execution::OUTPUT_BYTES}>,row:usize,column:usize){
    let mut digits=[0;20];output.write(b"\x1b[");let n=write_decimal(&mut digits,row);output.write(&digits[..n]);
    output.write(b";");let n=write_decimal(&mut digits,column);output.write(&digits[..n]);output.write(b"H");
}

// ------------------------=
// FUNC: poll_ssh_commands
// DESC: Exposes the serialized shared shell pump to both native architecture input loops.
// ------------------=
#[inline(never)]
pub fn poll_ssh_commands() { poll(); }
