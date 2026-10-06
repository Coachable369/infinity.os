//! Authenticated workplace commands and shared clipboard integration.
use super::*;
use infinity_enterprise_core::{self as work,Files,Error};
use crate::ui::clipboard::{self,ClipboardKind,ClipboardError};

// ------------------------=
// FUNC: now_ms
// DESC: Reads the same native monotonic time domain used by the browser worker.
// ------------------=
fn now_ms()->u64 {crate::runtime::ai::qwen::workers::clock_ns()/1_000_000}

struct NativeFiles;
// ------------------------=
// FUNC: storage_error
// DESC: Preserves capacity and collision failures instead of reporting every native storage rejection as generic I/O.
// ------------------=
fn storage_error(error:crate::storage::object::ObjectError)->Error {
    use crate::storage::object::ObjectError as E;
    match error {E::InsufficientCapacity=>Error::Capacity,E::NameConflict=>Error::Conflict,_=>Error::Storage}
}
impl Files for NativeFiles {
    // ------------------------=
    // FUNC: read
    // DESC: Reads exact native regular-file content, rejecting truncation and system objects.
    // ------------------=
    fn read(&mut self,path:&[u8],out:&mut [u8])->Result<usize,Error> {
        let (metadata,_)=crate::storage::object_inspect_path(path).map_err(|_|Error::Storage)?;
        if metadata.space!=crate::storage::object::Space::Personal || matches!(metadata.content_type,
            crate::storage::object::ContentType::Namespace|crate::storage::object::ContentType::System) {return Err(Error::Invalid);}
        if metadata.logical_size as usize>out.len() {return Err(Error::Capacity);}
        let (_,n)=crate::storage::object_read_path(path,None,out).map_err(|_|Error::Storage)?;
        if n!=metadata.logical_size as usize {return Err(Error::Integrity);}Ok(n)
    }
    // ------------------------=
    // FUNC: copy
    // DESC: Creates an independent native backup and refuses existing destination names.
    // ------------------=
    fn copy(&mut self,source:&[u8],destination:&[u8])->Result<(),Error> {
        personal_destination(destination)?;
        crate::storage::object_copy_path(source,destination).map(|_|()).map_err(storage_error)
    }
    // ------------------------=
    // FUNC: write
    // DESC: Creates one native content version; failed commits keep the old version authoritative.
    // ------------------=
    fn write(&mut self,path:&[u8],bytes:&[u8])->Result<u32,Error> {
        crate::storage::object_write_path(path,bytes).map_err(storage_error)
    }
}
// ------------------------=
// FUNC: personal_destination
// DESC: Restricts newly created workplace files to an existing Personal Space parent with no overwrite.
// ------------------=
fn personal_destination(path:&[u8])->Result<(),Error> {
    let normalized=crate::runtime::object_navigation::normalize_absolute_path(path).map_err(|_|Error::Invalid)?;
    if normalized.as_bytes()!=path || crate::storage::namespace_resolve(path).is_ok() {return Err(Error::Conflict);}
    let at=path.iter().rposition(|b|*b==b'/').ok_or(Error::Invalid)?;
    let (parent,_)=crate::storage::object_inspect_path(&path[..at.max(1)]).map_err(|_|Error::Storage)?;
    if parent.space!=crate::storage::object::Space::Personal {return Err(Error::Invalid);}Ok(())
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: workplace_authorized
    // DESC: Checks the full active session and native Personal Space capability before workplace operations.
    // ------------------=
    pub(super) fn workplace_authorized(&self,write:bool)->bool {
        use crate::runtime::identity::{SessionState,SESSION_PERSONAL_READ,SESSION_PERSONAL_WRITE};
        let required=if write {SESSION_PERSONAL_WRITE}else{SESSION_PERSONAL_READ};
        crate::runtime::with_runtime(|r|(0..crate::runtime::identity::MAX_SESSIONS).filter_map(|i|r.identity.session_nth(i))
            .any(|s|s.id==self.current_session && s.user==self.current_user && s.state==SessionState::Active && s.capabilities&required!=0)).unwrap_or(false)
    }
    // ------------------------=
    // FUNC: synchronize_workplace
    // DESC: Revokes clipboard and history on every observed lock, logout or identity transition, including console-issued locks.
    // ------------------=
    pub(super) fn synchronize_workplace(&mut self) {
        let active=self.workplace_authorized(false);
        if self.workplace_session!=self.current_session.0 {
            self.command_history=work::History::new();self.workplace_session=self.current_session.0;
            self.workplace_result=[0;8];
        }
        if !active {self.command_history.clear();}
        let now=now_ms();
        clipboard::with_shared(|c|{c.session(self.current_session.0,active);c.expire(now);});
    }
    // ------------------------=
    // FUNC: copy_workplace_text
    // DESC: Places complete selected text in the session clipboard without partially replacing prior contents.
    // ------------------=
    pub(super) fn copy_workplace_text(&self,bytes:&[u8])->bool {
        if bytes.is_empty() {return false;}
        clipboard::with_shared(|c|c.write(self.current_session.0,ClipboardKind::Utf8Text,bytes,now_ms())).is_ok()
    }
    // ------------------------=
    // FUNC: read_workplace_text
    // DESC: Reads shared text only for the current unlocked session and never coerces file references into text.
    // ------------------=
    pub(super) fn read_workplace_text(&self,out:&mut [u8])->Result<usize,ClipboardError> {
        clipboard::with_shared(|c|c.read(self.current_session.0,ClipboardKind::Utf8Text,out,now_ms()))
    }
    // ------------------------=
    // FUNC: workplace_console_key
    // DESC: Implements native command history, completion and atomic single-line paste without auto-execution.
    // ------------------=
    pub(super) fn workplace_console_key(&mut self,key:ConsoleKey)->bool {
        if !self.workplace_authorized(false) {return false;}
        match key {
            ConsoleKey::Shortcut(b'r')=>{
                if let Some(entry)=self.command_history.reverse_search(&self.command[..self.command_length]) {self.set_workplace_draft(entry.text());}
            },
            ConsoleKey::Up|ConsoleKey::Down=>{
                if let Some(entry)=self.command_history.recall(matches!(key,ConsoleKey::Up),&self.command[..self.command_length]) {
                    self.set_workplace_draft(entry.text());
                }
            },
            ConsoleKey::Tab(false)=>{
                let mut c=work::Completion::new(&self.command[..self.command_length]);
                for command in work::COMMANDS {c.offer(command);}
                for op in crate::runtime::console_language::OPERATIONS {
                    let mut name=[0;work::INPUT];let n=op.domain.len();let m=op.action.len();
                    if n+m+1>name.len() {continue;}
                    name[..n].copy_from_slice(op.domain);name[n]=b' ';name[n+1..n+1+m].copy_from_slice(op.action);c.offer(&name[..n+1+m]);
                }
                if let Some(entry)=c.result() {self.set_workplace_draft(entry.text());}
                if c.matches>1 {self.output.write_number(b"Matching commands: ",c.matches as u64);}
            },
            ConsoleKey::Shortcut(b'c'|b'x')=>{
                if self.copy_workplace_text(&self.command[..self.command_length]) && matches!(key,ConsoleKey::Shortcut(b'x')) {self.reset_input();}
            },
            ConsoleKey::Shortcut(b'v')=>{
                let mut bytes=[0;clipboard::MAX_CLIPBOARD_BYTES];
                match self.read_workplace_text(&mut bytes) {
                    Ok(n)=>if work::paste_line(&mut self.command,&mut self.command_length,&mut self.command_cursor,&bytes[..n]).is_err() {
                        self.output.write_line(b"Paste rejected: use one printable line within the input limit.");
                    },
                    Err(_)=>self.output.write_line(b"Clipboard empty, expired, or unavailable."),
                }
            },
            _=>return false,
        }true
    }
    // ------------------------=
    // FUNC: set_workplace_draft
    // DESC: Replaces the command draft without invoking any command handler.
    // ------------------=
    fn set_workplace_draft(&mut self,text:&[u8]) {
        if text.len()>self.command.len() {return;}
        self.command.fill(0);self.command[..text.len()].copy_from_slice(text);self.command_length=text.len();self.command_cursor=text.len();
    }
    // ------------------------=
    // FUNC: execute_workplace
    // DESC: Parses explicit workplace commands, rejects excess arguments, and reports actual operation failures.
    // ------------------=
    pub(super) fn execute_workplace(&mut self,input:&[u8])->bool {
        if input!=b"work" && !input.starts_with(b"work ") {return false;}
        if !self.workplace_authorized(false) {self.output.write_line(b"An active personal session is required.");return true;}
        let sequence=self.workplace_result[0].wrapping_add(1);self.workplace_result=[0;8];self.workplace_result[0]=sequence;
        let result=self.workplace_command(input);
        self.workplace_result[2]=if result.is_ok() {1}else{2};
        if let Err(error)=result {
            self.output.write_line(match error {Error::Integrity=>b"Integrity verification failed; do not trust this backup.",
                Error::Conflict=>b"Destination exists or source and destination conflict.",Error::Capacity=>b"Operation exceeds native capacity limits.",
                Error::Storage=>b"Storage operation failed; check the path, permissions, and capacity.",
                _=>b"Invalid arguments or unsupported object. Run work help."});
        }true
    }
    // ------------------------=
    // FUNC: workplace_command
    // DESC: Executes bounded clipboard, history, integrity, backup, restore and redacted support workflows.
    // ------------------=
    fn workplace_command(&mut self,input:&[u8])->Result<(),Error> {
        let w=work::Words::parse(input)?;let action=w.get(1).unwrap_or(b"help");let count=w.len();
        self.workplace_result[1]=match action {b"help"=>1,b"clipboard"=>2,b"private"=>3,b"history"=>4,b"checksum"=>5,
            b"compare"=>6,b"backup"=>7,b"restore"=>8,b"report"=>9,_=>0};
        match action {
            b"help" if count<=2=>{
                self.output.write_line(b"work clipboard clear|status|ttl SECONDS|browser-read on/off|browser-write on/off");
                self.output.write_line(b"work history find QUERY | work history clear | work private on/off");
                self.output.write_line(b"work checksum PATH | work compare LEFT RIGHT | work backup SOURCE NEW_PATH");
                self.output.write_line(b"work restore BACKUP TARGET SHA256 | work report NEW_PATH");
                self.output.write_line(b"Quote paths with spaces. Clipboard policy and history are session-local.");
                self.output.write_line(b"Up/Down recalls drafts; Tab completes; paste never executes commands.");
            },
            b"clipboard"=>{
                let option=w.get(2).ok_or(Error::Invalid)?;
                let (mut ttl,mut read,mut write)=clipboard::with_shared(|c|c.policy());
                match option {
                    b"clear" if count==3=>clipboard::with_shared(|c|c.clear()),
                    b"status" if count==3=>{
                        self.output.write_number(b"Expiry seconds (0 disables): ",ttl as u64);
                        self.output.write_number(b"Browser paste enabled: ",read as u64);
                        self.output.write_number(b"Browser copy/cut enabled: ",write as u64);return Ok(());
                    },
                    b"ttl" if count==4=>{
                        let value=core::str::from_utf8(w.get(3).unwrap()).map_err(|_|Error::Invalid)?;
                        ttl=value.parse::<u32>().map_err(|_|Error::Invalid)?;
                    },
                    b"browser-read"|b"browser-write" if count==4=>{
                        let enabled=match w.get(3) {Some(b"on")=>true,Some(b"off")=>false,_=>return Err(Error::Invalid)};
                        if option==b"browser-read" {read=enabled;}else{write=enabled;}
                    },
                    _=>return Err(Error::Invalid),
                }
                if !clipboard::with_shared(|c|c.configure(self.current_session.0,ttl,read,write,now_ms())) {return Err(Error::Invalid);}
                self.output.write_line(b"Clipboard updated for this session.");
            },
            b"private" if count==3=>{
                let enabled=match w.get(2) {Some(b"on")=>true,Some(b"off")=>false,_=>return Err(Error::Invalid)};
                self.command_history.set_private(enabled);
                self.output.write_line(if enabled {b"Private command mode on; history erased and command echo disabled."}else{b"Private command mode off."});
            },
            b"history"=>match w.get(2) {
                Some(b"clear") if count==3=>{self.command_history.clear();self.output.write_line(b"Command history erased.");},
                Some(b"find") if count==4=>{
                    let mut before=work::HISTORY;let mut found=0;
                    while found<OUTPUT_ROWS {
                        let Some((index,entry))=self.command_history.search(w.get(3).unwrap(),before) else {break;};
                        self.output.write_line(entry.text());before=index;found+=1;
                    }
                    if found==0 {self.output.write_line(b"No matching retained commands.");}
                },_=>return Err(Error::Invalid),
            },
            b"checksum" if count==3=>{
                let mut bytes=[0;work::CONTENT];let n=NativeFiles.read(w.get(2).unwrap(),&mut bytes)?;
                let hash=work::checksum(&bytes[..n]);self.workplace_digest(hash);
                self.output.write_hex(b"SHA256: ",&hash);
            },
            b"compare" if count==4=>{
                let mut left=[0;work::CONTENT];let mut right=[0;work::CONTENT];
                let a=NativeFiles.read(w.get(2).unwrap(),&mut left)?;let b=NativeFiles.read(w.get(3).unwrap(),&mut right)?;
                let result=work::compare(&left[..a],&right[..b]);
                self.workplace_result[3]=result.same as u64;self.workplace_result[4]=a as u64;self.workplace_result[5]=b as u64;
                self.workplace_result[6]=result.first_difference.map_or(u64::MAX,|v|v as u64);
                self.output.write_line(if result.same {b"Files are byte-identical."}else{b"Files differ."});
                self.output.write_number(b"Left bytes: ",a as u64);self.output.write_number(b"Right bytes: ",b as u64);
                if let Some(at)=result.first_difference {self.output.write_number(b"First difference (byte offset): ",at as u64);}
            },
            b"backup" if count==4=>{
                if !self.workplace_authorized(true) {return Err(Error::Invalid);}
                let digest=work::backup(&mut NativeFiles,w.get(2).unwrap(),w.get(3).unwrap())?;
                self.workplace_digest(digest);
                self.output.write_line(b"Independent backup verified. Retain this digest for restore.");self.output.write_hex(b"SHA256: ",&digest);
            },
            b"restore" if count==5=>{
                if !self.workplace_authorized(true) {return Err(Error::Invalid);}
                let digest=work::parse_digest(w.get(4).unwrap())?;
                let version=work::restore(&mut NativeFiles,w.get(2).unwrap(),w.get(3).unwrap(),digest)?;
                self.workplace_result[3]=version as u64;
                self.output.write_number(b"Restored and verified version: ",version as u64);
                self.output.write_line(b"Previous target content remains in native object history.");
            },
            b"report" if count==3=>{
                if !self.workplace_authorized(true) {return Err(Error::Invalid);}
                let path=w.get(2).unwrap();personal_destination(path)?;
                let storage=crate::storage::storage_usage().ok();
                let tasks=crate::runtime::with_runtime(|r|r.task_manager.task_count(&r.execution) as u32);
                #[cfg(feature="native-browser")]
                let browser=Some(crate::runtime::browser::status());
                #[cfg(not(feature="native-browser"))]
                let browser:Option<(u32,u32,u64,u64)>=None;
                let report=work::SupportReport{architecture:if cfg!(target_arch="aarch64") {1}else{2},installed:!self.system.live_profile,
                    width:self.system.framebuffer_width as u32,height:self.system.framebuffer_height as u32,
                    ready_devices:self.system.ready_device_count() as u32,storage_used:storage.map(|s|s.0),storage_total:storage.map(|s|s.1),tasks,
                    browser_state:browser.map(|v|v.0),browser_error:browser.map(|v|v.1)};
                let mut bytes=[0;512];let n=report.json(&mut bytes)?;
                crate::storage::object_create_note_at(crate::runtime::object_navigation::namespace_basename(path),&bytes[..n],path).map_err(storage_error)?;
                let mut verified=[0;512];let actual=NativeFiles.read(path,&mut verified)?;
                if actual!=n || verified[..n]!=bytes[..n] {return Err(Error::Integrity);}
                self.workplace_result[3]=n as u64;self.workplace_result[4]=crate::storage::object::crc32(&verified[..n]) as u64;
                self.output.write_line(b"Redacted JSON support report saved and verified.");
            },
            _=>return Err(Error::Invalid),
        }Ok(())
    }
    // ------------------------=
    // FUNC: workplace_digest
    // DESC: Publishes the digest explicitly requested by an authenticated integrity command to read-only diagnostics.
    // ------------------=
    fn workplace_digest(&mut self,digest:[u8;32]) {
        for (i,word) in digest.chunks_exact(8).enumerate() {self.workplace_result[4+i]=u64::from_le_bytes(word.try_into().unwrap());}
    }
    // ------------------------=
    // FUNC: file_clipboard
    // DESC: Copies or stages a move using a full object identity; paste refuses stale sources and never overwrites collisions.
    // ------------------=
    pub(super) fn file_clipboard(&mut self,key:u8,state:crate::runtime::object_navigation::FileNavigatorState) {
        let result=(||->Result<(),Error>{
            if !self.workplace_authorized(key!=b'c') {return Err(Error::Invalid);}
            if key==b'c' || key==b'x' {
                if (state.selected_index as usize)<crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT {return Err(Error::Invalid);}
                let entry=navigator_child_nth(state.active_namespace_ref.as_bytes(),state.selected_index as usize).ok_or(Error::Invalid)?;
                let path=&entry.path[..entry.path_len as usize];let id=crate::storage::namespace_resolve(path).map_err(|_|Error::Storage)?;
                if crate::storage::object_inspect_path(path).map_err(|_|Error::Storage)?.0.space!=crate::storage::object::Space::Personal {return Err(Error::Invalid);}
                let mut value=[0;512];if path.len()+17>value.len() {return Err(Error::Capacity);}
                value[0]=(key==b'x') as u8;value[1..17].copy_from_slice(&id.0);value[17..17+path.len()].copy_from_slice(path);
                clipboard::with_shared(|c|c.write(self.current_session.0,ClipboardKind::ObjectRefs,&value[..17+path.len()],now_ms())).map_err(|_|Error::Invalid)?;
            }else if key==b'v' {
                let mut value=[0;512];let n=clipboard::with_shared(|c|c.read(self.current_session.0,ClipboardKind::ObjectRefs,&mut value,now_ms())).map_err(|_|Error::Invalid)?;
                if n<=17 || value[0]>1 {return Err(Error::Invalid);}
                let source=&value[17..n];let id=crate::storage::namespace_resolve(source).map_err(|_|Error::Storage)?;
                if id.0!=value[1..17] {return Err(Error::Conflict);}
                let destination=crate::runtime::object_navigation::namespace_child_path(state.active_namespace_ref.as_bytes(),
                    crate::runtime::object_navigation::namespace_basename(source)).map_err(|_|Error::Invalid)?;
                personal_destination(destination.as_bytes())?;
                if value[0]==1 {
                    crate::storage::namespace_move(source,destination.as_bytes()).map_err(|_|Error::Storage)?;
                    clipboard::with_shared(|c|c.clear());
                }else{crate::storage::object_copy_path(source,destination.as_bytes()).map_err(|_|Error::Storage)?;}
            }else{return Err(Error::Invalid);}Ok(())
        })();
        let notice=if result.is_ok() {if key==b'v' {1}else if key==b'x' {2}else{3}} else {4};
        let _=crate::runtime::with_runtime(|r|{if let Some(n)=r.file_navigator.as_mut() {n.workplace_notice=notice;n.context_menu_open=false;}});
    }
}
