//! Authorized, bounded log viewing shared by local and SSH consoles.
use super::*;
use crate::ui::log_follow::{self, Follow, CAPACITY};

pub(super) struct LogReader {
    pub active: bool,
    path: [u8;96],
    length: usize,
    owner: [u8;16],
    session: [u8;16],
    next: u64,
    lines: Option<usize>,
    follow: Follow,
}
impl LogReader {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one bounded file follower per authenticated console, including independent SSH sessions.
    // ------------------=
    pub const fn new()->Self {Self{active:false,path:[0;96],length:0,owner:[0;16],session:[0;16],next:0,lines:None,follow:Follow::new()}}
    // ------------------------=
    // FUNC: clear
    // DESC: Erases retained contents and identity bindings when following ends or authorization changes.
    // ------------------=
    pub fn clear(&mut self){self.active=false;self.path.fill(0);self.length=0;self.owner.fill(0);self.session.fill(0);self.follow.clear();self.next=0;self.lines=None;}
}
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: log_command
    // DESC: Opens cat or tail through the existing authorized file adapter without altering files or bypassing private-object policy.
    // ------------------=
    pub(super) fn log_command(&mut self,command:&[u8]) {
        use crate::runtime::console_execution::Diagnostic;
        if self.current_session.is_zero(){self.output.fail(Diagnostic::Denied);self.output.write_line(b"Sign in before reading files.");return;}
        let Ok(options)=log_follow::options(command) else {
            self.output.fail(Diagnostic::Syntax);self.output.write_line(b"Usage: cat|tail [-f] [-n LINES] [--] FILE");return;
        };
        let Some(path)=self.utility_path(options.path) else {self.output.fail(Diagnostic::Syntax);self.output.write_line(b"Invalid file path.");return;};
        self.log_reader.clear();
        self.log_reader.length=path.as_bytes().len();self.log_reader.path[..self.log_reader.length].copy_from_slice(path.as_bytes());
        self.log_reader.owner=self.current_user.0;self.log_reader.session=self.current_session.0;
        self.log_reader.lines=options.lines;self.log_reader.active=true;
        self.poll_log_reader();
        if !options.follow {self.log_reader.clear();}
    }
    // ------------------------=
    // FUNC: log_reader_key
    // DESC: Reserves Ctrl-C and Escape to stop following without submitting typed text as another command.
    // ------------------=
    pub(super) fn log_reader_key(&mut self,key:ConsoleKey)->bool {
        if !self.log_reader.active{return false;}
        if matches!(key,ConsoleKey::Escape|ConsoleKey::Shortcut(b'c')) {self.log_reader.clear();self.output.write_line(b"");self.reset_input();}
        true
    }
    #[inline(never)]
    // ------------------------=
    // FUNC: poll_log_reader
    // DESC: Reauthorizes every one-second snapshot, streams only appended bytes, and handles replacement without blocking the event loop.
    // ------------------=
    pub(super) fn poll_log_reader(&mut self) {
        if !self.log_reader.active{return;}
        if self.current_session.is_zero()||self.log_reader.session!=self.current_session.0||self.log_reader.owner!=self.current_user.0 {
            self.log_reader.clear();return;
        }
        let now=crate::runtime::ai::qwen::workers::clock_ns();
        if now<self.log_reader.next{return;}self.log_reader.next=now.saturating_add(1_000_000_000);
        let mut bytes=[0u8;CAPACITY];
        let loaded=crate::storage::editor_read_path(self.current_user.0,self.current_session.0,&self.log_reader.path[..self.log_reader.length],&mut bytes);
        match loaded {
            Ok((length,_))=>{
                if let Ok((start,replaced))=self.log_reader.follow.update(&bytes[..length],self.log_reader.lines.take()) {
                    if replaced {self.output.write_line(b"[File replaced or truncated]");}
                    if start<length {
                        if let Some(output)=self.output.remote.as_mut(){output.stdout.write(&bytes[start..length]);}
                        else {
                            crate::ui::command_terminal::mutate(|terminal|{if terminal.authorized{terminal.append(&bytes[start..length]);}});
                            crate::bootstrap::system_ui_settings_changed();
                        }
                    }
                }
            },
            Err(error)=>{self.log_reader.clear();self.storage_error(error);},
        }
        bytes.fill(0);
    }
}
