//! Session-local shell lookup and immediate-child completion over the native object namespace.
use super::ConsoleRuntime;
use crate::runtime::object_navigation::ByteText;

// ------------------------=
// FUNC: default_path
// DESC: Supplies a fresh session's executable search list without inheriting another user's settings.
// ------------------=
pub(super) fn default_path()->ByteText<240> {ByteText::new(b"/System").unwrap_or(ByteText::empty())}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: print_motd
    // DESC: Reads the current version on every fresh terminal launch; an empty message is intentional.
    // ------------------=
    pub(super) fn print_motd(&mut self) {
        let mut buf=[0;4096];
        match crate::storage::object_read_path(b"/System/motd",None,&mut buf) {
            Ok((_,n))=>for line in buf[..n].split(|&b|b==b'\n') {self.output.write_line(line);},
            Err(_)=>self.output.write_line(b"Infinity Command Window"),
        }
    }

    // ------------------------=
    // FUNC: shell_environment
    // DESC: Expands complete $PATH components before atomically changing the bounded session search path.
    // ------------------=
    pub(super) fn shell_environment(&mut self, input:&[u8])->bool {
        if input==b"PATH" || input==b"echo $PATH" {self.output.write_line(self.shell_path.as_bytes());return true;}
        if input==b"motd" {self.print_motd();return true;}
        if let Some(message)=input.strip_prefix(b"motd set ") {
            let admin=self.workplace_authorized(true)&&crate::runtime::with_runtime(|r|r.identity.is_administrator(self.current_user)).unwrap_or(false);
            self.output.write_line(if !admin {b"Administrator session required."}else if crate::storage::object_write_path(b"/System/motd",message).is_ok(){b"Message of the day saved."}else{b"Cannot save /System/motd."});return true;
        }
        let Some(value)=input.strip_prefix(b"PATH=") else{return false;};
        let mut buf=[0;240];let mut used=0;
        for part in value.split(|&b|b==b';').filter(|part|!part.is_empty()) {
            let dir=if part==b"$PATH"{self.shell_path.as_bytes()}else{part};
            if dir.is_empty(){continue;}
            let sep=usize::from(used!=0);
            if used+sep+dir.len()>buf.len(){self.output.write_line(b"PATH too long; previous value retained.");return true;}
            if sep!=0 {buf[used]=b';';used+=1;}
            buf[used..used+dir.len()].copy_from_slice(dir);used+=dir.len();
        }
        if let Ok(path)=ByteText::new(&buf[..used]) {self.shell_path=path;}true
    }

    // ------------------------=
    // FUNC: shell_script
    // DESC: Looks up an exact script or its .py spelling in PATH order without consulting a host filesystem.
    // ------------------=
    pub(super) fn shell_script(&self, name:&[u8])->Option<ByteText<96>> {
        if name.is_empty()||name.contains(&b'/') {return None;}
        let mut filename=[0;96];
        if name.len()+3>filename.len(){return None;}
        filename[..name.len()].copy_from_slice(name);
        filename[name.len()..name.len()+3].copy_from_slice(b".py");
        let length=if name.ends_with(b".py"){name.len()}else{name.len()+3};
        for dir in self.shell_path.as_bytes().split(|&b|b==b';').filter(|d|!d.is_empty()) {
            let Some(dir)=self.utility_path(dir) else{continue;};
            if let Ok(path)=crate::runtime::object_navigation::join_path(dir.as_bytes(),&filename[..length]) {
                if crate::storage::namespace_resolve(path.as_bytes()).is_ok(){return Some(path);}
            }
        }
        None
    }

    // ------------------------=
    // FUNC: complete_shell_path
    // DESC: Completes visible object names, lists ambiguity and preserves text to the right of the cursor.
    // ------------------=
    pub(super) fn complete_shell_path(&mut self)->bool {
        let draft=self.command;let cursor=self.command_cursor.min(self.command_length);
        let Some(space)=draft[..cursor].iter().position(u8::is_ascii_whitespace) else{return false;};
        let cd=draft[..space].eq_ignore_ascii_case(b"cd") || &draft[..space]==b"idir";
        let start=if cd{(space..cursor).find(|&i|!draft[i].is_ascii_whitespace()).unwrap_or(cursor)}
            else{draft[..cursor].iter().rposition(u8::is_ascii_whitespace).unwrap_or(space)+1};
        let split=draft[start..cursor].iter().rposition(|&b|b==b'/').map_or(start,|i|start+i+1);
        let dir=if split==start {ByteText::new(self.navigation_context.path()).ok()}else{self.utility_path(&draft[start..split])};
        let Some(dir)=dir else{return true;};
        let _capture=crate::ui::command_terminal::Capture::new(self.command_terminal_visible());
        let mut common=[0;96];let mut n=0;let mut matches=0;let mut directory=false;
        for i in 0..crate::storage::object::PUBLIC_NAMESPACE_CAPACITY {
            let Ok(Some(entry))=crate::storage::namespace_list_nth(dir.as_bytes(),i) else{break;};
            let path=&entry.path[..entry.path_len as usize];
            if !crate::runtime::object_navigation::is_immediate_namespace_child(dir.as_bytes(),path){continue;}
            let name=path.rsplit(|&b|b==b'/').next().unwrap_or(&[]);
            if name.len()<cursor-split || !name[..cursor-split].eq_ignore_ascii_case(&draft[split..cursor]) {continue;}
            let Ok((meta,_))=crate::storage::object_inspect_path(path) else{continue;};
            if meta.owner.0!=[0;16] && meta.owner.0!=self.current_user.0 {continue;}
            let is_dir=meta.kind==crate::storage::object::ObjectType::NamespaceNode;
            if cd && !is_dir {continue;}
            directory=is_dir;
            if matches==0 {common[..name.len()].copy_from_slice(name);n=name.len();}
            else{n=common[..n].iter().zip(name).take_while(|(a,b)|a==b).count();}
            self.output.write_line(name);matches+=1;
        }
        if matches==0 {return cd;}
        if matches==1 && directory && n<common.len(){common[n]=b'/';n+=1;}
        if n>=cursor-split && self.command_length-(cursor-split)+n<=self.command.len(){
            self.command.copy_within(cursor..self.command_length,split+n);
            self.command[split..split+n].copy_from_slice(&common[..n]);
            self.command_length=self.command_length-(cursor-split)+n;self.command_cursor=split+n;
        }
        true
    }
}
