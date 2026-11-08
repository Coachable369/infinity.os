//! Nano-style local Console editor. File operations use the existing authorized editor adapter.
use super::*;
use crate::ui::terminal_edit as edit;
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: start_terminal_edit
    // DESC: Opens a new or existing bounded text file without changing its content until explicit save.
    // ------------------=
    pub(super) fn start_terminal_edit(&mut self,name:&[u8]) {
        let Some(path)=self.utility_path(name)else{self.output.write_line(b"Usage: edit FILE (new or existing)");return;};
        if path.as_bytes().len()>95{self.output.write_line(b"Path exceeds 95 bytes.");return;}
        edit::session(self.current_session.0);
        if edit::inspect(|s|s.active&&!s.document.is_saved()){self.output.write_line(b"Finish the current edit before opening another file.");return;}
        let mut bytes=[0;crate::ui::text_editor::DOCUMENT_CAPACITY];
        let loaded=crate::storage::editor_read_path(self.current_user.0,self.current_session.0,path.as_bytes(),&mut bytes);
        let (len,memory,new)=match loaded {
            Ok((n,m))=>(n,m,false),
            Err(crate::storage::object::ObjectError::NotFound|crate::storage::object::ObjectError::NamespaceNotFound)=>(0,None,true),
            Err(error)=>{self.storage_error(error);return;}
        };
        let revision=if new{None}else{crate::storage::object_inspect_path(path.as_bytes()).ok().map(|(m,_)|(m.object.id.0,m.current_version))};
        if !edit::mutate(|s|s.document.open(&bytes[..len])){bytes.fill(0);self.output.write_line(b"edit supports ASCII text up to 16 KiB; this file was not changed.");return;}
        bytes.fill(0);if self.output.remote.is_none(){self.open_command_window();crate::bootstrap::system_ui_settings_changed();}
        edit::mutate(|s|{s.active=true;s.closing_window=false;s.path.fill(0);s.path_len=path.as_bytes().len();s.path[..s.path_len].copy_from_slice(path.as_bytes());s.document.set_cursor(0);s.top=0;s.revision=revision;s.memory=memory.map(|(id,version)|(id.0,version));s.prompt=0;s.notice=if new{b"New file - Ctrl+S to create"}else{b"Ctrl+S Save   Ctrl+X Exit   Ctrl+W Find   Ctrl+G Go to line"};});
    }
    // ------------------------=
    // FUNC: save_terminal_edit
    // DESC: Rechecks live write authority and file revision, and publishes Saved only after a durable native commit.
    // ------------------=
    fn save_terminal_edit(&mut self)->bool {
        if !self.workplace_authorized(true){edit::mutate(|s|s.notice=b"Save denied: session is not writable.");return false;}
        edit::mutate(|s|{
            let path=&s.path[..s.path_len];
            let current=crate::storage::object_inspect_path(path).ok().map(|(m,_)|(m.object.id.0,m.current_version));
            if current!=s.revision {s.notice=b"File changed externally. Save cancelled; reopen to avoid overwriting.";return false;}
            let result=if current.is_some() {
                crate::storage::editor_write_path(self.current_user.0,self.current_session.0,path,s.document.bytes(),s.memory.map(|(id,version)|(crate::storage::object::ObjectId(id),version)))
            }else{
                let slash=path.iter().rposition(|b|*b==b'/').unwrap_or(0);
                let parent=if slash==0{b"/".as_slice()}else{&path[..slash]};
                let allowed=crate::storage::object_inspect_path(parent).is_ok_and(|(m,_)|m.space==crate::storage::object::Space::Personal&&(m.owner.0==self.current_user.0||m.owner.0==[0;16]));
                if !allowed{s.notice=b"Create denied: choose an existing writable Personal folder.";return false;}
                crate::storage::object_create_owned_note_at(&path[slash+1..],s.document.bytes(),path,crate::storage::object::ObjectId(self.current_user.0)).map(|_|None)
            };
            match result {Ok(memory)=>{s.memory=memory.map(|(id,version)|(id.0,version));s.revision=crate::storage::object_inspect_path(path).ok().map(|(m,_)|(m.object.id.0,m.current_version));s.document.save();s.notice=b"Saved";true},Err(_)=>{s.notice=b"Save failed: check permissions, available storage and file type.";false}}
        })
    }
    // ------------------------=
    // FUNC: terminal_edit_close
    // DESC: Guards both keyboard exit and window close against accidental loss of edits.
    // ------------------=
    pub(super) fn terminal_edit_close(&mut self)->bool {
        crate::bootstrap::system_ui_settings_changed();
        edit::mutate(|s|{if !s.active{return false;}if !s.document.is_saved(){s.prompt=1;s.notice=b"Save changes? Y = save and exit / N = discard / Esc = cancel";}else{s.active=false;s.document.erase_private();return false;}true})
    }
    // ------------------------=
    // FUNC: terminal_edit_key
    // DESC: Handles editing, search, line navigation, bounded cut/paste and explicit save/discard decisions independently of command input.
    // ------------------=
    pub(super) fn terminal_edit_key(&mut self,key:ConsoleKey)->bool {
        if !edit::inspect(|s|s.active&&s.owner==self.current_session.0){return false;}
        if !self.workplace_authorized(false){return false;}
        crate::bootstrap::system_ui_settings_changed();
        let prompt=edit::inspect(|s|s.prompt);
        if prompt==1 {
            match key {
                ConsoleKey::Character(b'y'|b'Y')=>{if self.save_terminal_edit(){edit::mutate(|s|{s.active=false;s.prompt=0;s.document.erase_private();});}},
                ConsoleKey::Character(b'n'|b'N')=>edit::mutate(|s|{s.active=false;s.prompt=0;s.document.erase_private();}),
                ConsoleKey::Escape=>edit::mutate(|s|{s.prompt=0;s.closing_window=false;s.notice=b"Exit cancelled";}),_=>{}
            }
            if edit::inspect(|s|!s.active&&s.closing_window){edit::mutate(|s|s.closing_window=false);self.close_desktop_app_unchecked();}
            return true;
        }
        if prompt==2||prompt==3 {
            edit::mutate(|s|match key {
                ConsoleKey::Escape=>{s.prompt=0;s.notice=b"Cancelled";},
                ConsoleKey::Backspace=>s.query_len=s.query_len.saturating_sub(1),
                ConsoleKey::Character(b) if (32..=126).contains(&b)&&s.query_len<s.query.len()=>{s.query[s.query_len]=b;s.query_len+=1;},
                ConsoleKey::Enter=>{s.prompt=0;if prompt==2{s.notice=if s.query_len>0&&s.document.find(&s.query[..s.query_len],false){b"Found"}else{b"Text not found"};}else{
                    let line=core::str::from_utf8(&s.query[..s.query_len]).ok().and_then(|v|v.parse::<usize>().ok());
                    if let Some(line)=line.filter(|n|*n>0){s.document.goto_line(line);s.notice=b"";}else{s.notice=b"Enter a positive line number";}
                }},_=>{}
            });return true;
        }
        match key {
            ConsoleKey::Shortcut(b's'|b'o')=>{self.save_terminal_edit();return true;},
            ConsoleKey::Shortcut(b'x')|ConsoleKey::Escape=>{self.terminal_edit_close();return true;},
            _=>{}
        }
        edit::mutate(|s|{
            s.notice=b"Ctrl+S Save   Ctrl+X Exit   Ctrl+W Find   Ctrl+G Go to line";
            let changed=match key {
                ConsoleKey::Character(b)=>s.document.insert(b),
                ConsoleKey::Enter=>s.document.insert(b'\n'),
                ConsoleKey::Tab(_)=>s.document.replace_selection(b"    "),
                ConsoleKey::Backspace=>s.document.backspace(),ConsoleKey::Delete=>s.document.delete(),
                ConsoleKey::Left=>s.document.move_cursor(-1),ConsoleKey::Right=>s.document.move_cursor(1),
                ConsoleKey::Up=>s.document.move_cursor_vertical(true),ConsoleKey::Down=>s.document.move_cursor_vertical(false),
                ConsoleKey::Home=>s.document.move_cursor_to_line_edge(false),ConsoleKey::End=>s.document.move_cursor_to_line_edge(true),
                ConsoleKey::Shortcut(b'a')=>{s.document.select(0,s.document.bytes().len());true},
                ConsoleKey::Shortcut(b'z')=>s.document.undo(),ConsoleKey::Shortcut(b'y')=>s.document.redo(),
                ConsoleKey::Shortcut(b'w'|b'f'|b'g')=>{s.prompt=if matches!(key,ConsoleKey::Shortcut(b'g')){3}else{2};s.query_len=0;true},
                ConsoleKey::Shortcut(b'k')=>{
                    let bytes=s.document.bytes();let at=s.document.cursor();let start=bytes[..at].iter().rposition(|b|*b==b'\n').map(|i|i+1).unwrap_or(0);
                    let end=bytes[at..].iter().position(|b|*b==b'\n').map(|i|at+i+1).unwrap_or(bytes.len());
                    if end-start>s.cut.len(){s.notice=b"Line exceeds the 4 KiB cut buffer";true}else{s.cut_len=end-start;s.cut[..s.cut_len].copy_from_slice(&bytes[start..end]);s.document.select(start,end);s.document.replace_selection(b"")}
                },
                ConsoleKey::Shortcut(b'u')=>s.document.replace_selection(&s.cut[..s.cut_len]),
                _=>return false,
            };
            if !changed&&matches!(key,ConsoleKey::Character(_)|ConsoleKey::Enter|ConsoleKey::Tab(_)){s.notice=b"Document is full (16 KiB) or input is unsupported";}
            true
        })
    }
}
