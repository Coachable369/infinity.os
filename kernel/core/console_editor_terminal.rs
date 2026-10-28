//! Editor terminal controller reusing the native shell, process and compiler services.
use super::*;
use crate::ui::editor_chrome::terminal::{self, Layout, Preferences};
#[no_mangle]
pub static mut INFINITY_EDITOR_TERMINAL_SNAPSHOT:[u64;72]=[0;72];
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: editor_terminal_pointer_shape
    // DESC: Shows the shared vertical-resize and text cursors over the dock's real drag and input targets.
    // ------------------=
    pub(super) fn editor_terminal_pointer_shape(&self)->Option<crate::ui::text_input::PointerShape>{
        if self.mode!=ConsoleMode::Desktop||self.desktop_app!=DesktopAppKind::TextEditor||self.editor_dialog!=EditorDialog::None||!terminal::preferences().open{return None;}
        let p=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).point(self.pointer_x,self.pointer_y);
        let g=self.editor_terminal_layout();
        if terminal::inspect(|state|state.dragging)||g.splitter.contains(p){return Some(crate::ui::text_input::PointerShape::ResizeVertical);}
        if g.input.contains(p){return Some(crate::ui::text_input::PointerShape::Text);}None
    }
    // ------------------------=
    // FUNC: editor_terminal_layout
    // DESC: Resolves terminal hit targets using the same bounded geometry as the renderer.
    // ------------------=
    fn editor_terminal_layout(&self)->Layout {
        let g=self.editor_layout();
        let cell=crate::ui::editor_tools::preferences::Preferences{size:3,..crate::ui::editor_tools::preferences::current()}.cell()*g.scale;
        terminal::layout(crate::ui::geometry::Rect{height:(g.status.y-g.body.y).max(0) as u32,..g.body},g.scale,cell)
    }
    // ------------------------=
    // FUNC: save_editor_terminal
    // DESC: Atomically persists dock visibility and height under the authenticated user's private preference object.
    // ------------------=
    fn save_editor_terminal(&mut self) {
        if crate::storage::editor_terminal_preferences(self.current_user.0,self.current_session.0,Some(terminal::preferences())).is_err(){
            self.editor_tools.notice=b"Terminal layout could not be saved.";
        }
    }
    // ------------------------=
    // FUNC: restore_editor_terminal
    // DESC: Restores layout but never another user's commands, transcript, pending restart or process association.
    // ------------------=
    pub(super) fn restore_editor_terminal(&mut self) {
        let preferences=crate::storage::editor_terminal_preferences(self.current_user.0,self.current_session.0,None).unwrap_or(Preferences::new());
        terminal::mutate(|state|{*state=terminal::State::new();state.preferences=preferences;});
    }
    // ------------------------=
    // FUNC: toggle_editor_terminal
    // DESC: Collapses the dock without killing a running task or losing its scrollback.
    // ------------------=
    pub(super) fn toggle_editor_terminal(&mut self) {
        terminal::mutate(|state|{state.preferences.open=!state.preferences.open;state.focused=state.preferences.open;});
        self.save_editor_terminal();
    }
    // ------------------------=
    // FUNC: open_execution_console
    // DESC: Keeps editor-owned execution in its dock while preserving standalone Command behavior.
    // ------------------=
    pub(super) fn open_execution_console(&mut self) {
        if terminal::inspect(|state|state.attached) && self.assistant_owner_alive(2) {
            terminal::mutate(|state|{state.preferences.open=true;state.scroll=0;});
        } else {self.open_command_window();}
    }
    // ------------------------=
    // FUNC: editor_terminal_running
    // DESC: Reads real native execution state instead of inferring completion from console text.
    // ------------------=
    fn editor_terminal_running(&self)->bool {
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if compiler_commands::editor_running(self){return true;}
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        if python_commands::editor_running(self){return true;}
        false
    }
    // ------------------------=
    // FUNC: editor_terminal_stop
    // DESC: Stops the session-owned compiler or script through existing cancellable native lifecycles.
    // ------------------=
    fn editor_terminal_stop(&mut self) {
        self.reset_input();
        #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        compiler_commands::editor_stop(self);
        #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
        {let _=python_commands::command(self,b"python-cancel");}
    }
    // ------------------------=
    // FUNC: preserve_editor_execution
    // DESC: Moves live dock execution into Command when its editor closes, without changing task or session authority.
    // ------------------=
    pub(super) fn preserve_editor_execution(&mut self){
        if matches!(self.mode,ConsoleMode::Desktop|ConsoleMode::Settings|ConsoleMode::SystemMenu|ConsoleMode::AppLauncher)
            &&terminal::inspect(|state|state.attached)&&!self.assistant_owner_alive(2)&&self.editor_terminal_running(){
            terminal::mutate(|state|{state.attached=false;state.focused=false;state.restart=false;});
            if !self.assistant_owner_alive(1){self.open_command_window();self.redraw();}
        }
    }
    // ------------------------=
    // FUNC: editor_terminal_run
    // DESC: Saves the actual current document before selecting native Python or C execution.
    // ------------------=
    fn editor_terminal_run(&mut self) {
        if self.editor_terminal_running(){self.editor_tools.notice=b"Stop the current execution before running again.";return;}
        if self.editor_document_path_length==0 {self.open_editor_save_as_dialog();self.editor_tools.notice=b"Choose a .c or .py filename, then Run.";return;}
        if !self.save_editor_document(){self.editor_tools.notice=b"Save failed; nothing was executed.";return;}
        self.reset_input();
        terminal::mutate(|state|{state.preferences.open=true;state.attached=true;state.focused=true;state.scroll=0;});
        self.save_editor_terminal();
        let path=self.editor_document_path;let n=self.editor_document_path_length;
        if n>95 {self.output.write_line(b"Native execution paths must be at most 95 bytes.");return;}
        if path[..n].ends_with(b".py") {
            #[cfg(all(feature="native-python",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
            {let mut command=[0;104];command[..7].copy_from_slice(b"python ");command[7..7+n].copy_from_slice(&path[..n]);
                self.output.write_segments(&[b"$ ",&command[..7+n]]);let _=python_commands::command(self,&command[..7+n]);return;}
        }else if path[..n].ends_with(b".c") {
            #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
            {if let Err(_)=compiler_commands::editor_run(self){self.output.write_line(b"Build could not start; check compiler availability and session authority.");}return;}
        }
        self.output.write_line(b"Run supports native .c and .py documents on this build.");
    }
    // ------------------------=
    // FUNC: poll_editor_terminal
    // DESC: Restarts only after cancellation has released the previous native task and its mappings.
    // ------------------=
    pub(super) fn poll_editor_terminal(&mut self) {
        let running=self.editor_terminal_running();let (restart,was_running)=terminal::inspect(|state|(state.restart,state.running));
        terminal::mutate(|state|state.running=running);
        if restart&&!running {
            terminal::mutate(|state|state.restart=false);
            if self.assistant_owner_alive(2){self.editor_terminal_run();}
        }
        if was_running!=running {self.redraw();}
    }
    // ------------------------=
    // FUNC: input_editor_terminal
    // DESC: Gives the focused dock native shell or process input without mutating the code document.
    // ------------------=
    pub(super) fn input_editor_terminal(&mut self,key:ConsoleKey)->bool {
        if matches!(key,ConsoleKey::Shortcut(b'`')) {self.toggle_editor_terminal();return true;}
        if !terminal::inspect(|state|state.preferences.open&&state.focused){return false;}
        if matches!(key,ConsoleKey::Escape){terminal::mutate(|state|state.focused=false);return true;}
        terminal::mutate(|state|{state.attached=true;state.scroll=0;});
        if matches!(key,ConsoleKey::Shortcut(b'c'))&&self.editor_terminal_running(){self.editor_terminal_stop();return true;}
        self.input_console(key);true
    }
    // ------------------------=
    // FUNC: pointer_editor_terminal
    // DESC: Handles split dragging, run controls, input focus and bounded source diagnostic navigation.
    // ------------------=
    pub(super) fn pointer_editor_terminal(&mut self,clicked:bool,released:bool)->bool {
        if self.mode!=ConsoleMode::Desktop||self.desktop_app!=DesktopAppKind::TextEditor||self.editor_dialog!=EditorDialog::None{return false;}
        let g=self.editor_terminal_layout();let state=terminal::read();let layout=self.editor_layout();
        let count=state.visual_count(state.viewport_columns);
        let p=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).point(self.pointer_x,self.pointer_y);
        if state.dragging {
            if released {terminal::mutate(|state|state.dragging=false);self.save_editor_terminal();}
            else {let height=((layout.status.y-p.y).max(0) as usize/layout.scale.max(1)).clamp(112,1024) as u16;
                terminal::mutate(|state|state.preferences.height=height);}
            self.redraw();return true;
        }
        if state.scroll_dragging {
            if released{terminal::mutate(|state|state.scroll_dragging=false);}
            else{terminal::mutate(|next|next.scroll=g.drag_scroll(p.y,state.scroll_drag_offset,layout.scale,count));}
            self.redraw();return true;
        }
        if self.editor_tools.menu!=crate::ui::editor_chrome::Menu::None{return false;}
        if state.preferences.open&&g.splitter.contains(p)&&clicked{terminal::mutate(|state|state.dragging=true);return true;}
        if !g.panel.contains(p){if clicked {terminal::mutate(|state|state.focused=false);}return false;}
        if !clicked{return true;}
        self.ai_chat_focus=0;
        let mut assistant=crate::ui::app_assistant::read(2);assistant.focused=false;crate::ui::app_assistant::write(2,assistant);
        if g.toggle.contains(p){self.toggle_editor_terminal();}
        else if g.run.contains(p){self.editor_terminal_run();}
        else if g.stop.contains(p){terminal::mutate(|state|state.restart=false);self.editor_terminal_stop();}
        else if g.restart.contains(p){terminal::mutate(|state|state.restart=true);self.editor_terminal_stop();self.poll_editor_terminal();}
        else if g.input.contains(p){
            terminal::mutate(|state|state.focused=true);
            let appearance=crate::ui::editor_tools::preferences::current();
            let cell=crate::ui::editor_tools::preferences::Preferences{size:3,..appearance}.cell()*layout.scale;
            let visible=(g.input.width as usize).saturating_sub(28*layout.scale)/cell.max(1);
            let start=self.command_cursor.saturating_sub(visible.saturating_sub(1));
            self.command_cursor=(start+((p.x-g.input.x-12*layout.scale as i32).max(0)as usize+cell/2)/cell.max(1)).min(self.command_length);
        }
        else if g.track.contains(p){let grab=if g.thumb.contains(p){p.y-g.thumb.y}else{g.thumb.height as i32/2};terminal::mutate(|next|{next.scroll_dragging=true;next.scroll_drag_offset=grab;next.scroll=g.drag_scroll(p.y,grab,layout.scale,count);});}
        else if let Some((row,_,_))=g.row_at(p,layout.scale,count,state.scroll).and_then(|index|state.visual_line(index,state.viewport_columns)) {
            if let Some(location)=terminal::diagnostic(&state.lines[row][..state.lengths[row]],&self.editor_document_path[..self.editor_document_path_length]) {
                let source=self.editor_document.bytes();let mut at=0;let mut line=1;
                while at<source.len()&&line<location.line {if source[at]==b'\n'{line+=1;}at+=1;}
                let end=source[at..].iter().position(|b|*b==b'\n').map_or(source.len(),|n|at+n);
                self.editor_document.set_cursor((at+location.column.saturating_sub(1)).min(end));
                self.reveal_editor_cursor();terminal::mutate(|state|state.focused=false);
            }
        }
        self.redraw();true
    }
    // ------------------------=
    // FUNC: scroll_editor_terminal
    // DESC: Scrolls native transcript rows without moving the source document or losing tail-follow state.
    // ------------------=
    pub(super) fn scroll_editor_terminal(&mut self,vertical:i8)->bool {
        if self.mode!=ConsoleMode::Desktop||self.desktop_app!=DesktopAppKind::TextEditor||!terminal::preferences().open{return false;}
        let g=self.editor_terminal_layout();let layout=self.editor_layout();
        let p=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).point(self.pointer_x,self.pointer_y);
        if !g.output.contains(p)&&!g.track.contains(p){return false;}
        terminal::mutate(|state|{state.scroll=(state.scroll as isize+vertical as isize*3).clamp(0,state.visual_count(state.viewport_columns).saturating_sub(g.rows(layout.scale)) as isize)as usize;});
        self.redraw();true
    }
    // ------------------------=
    // FUNC: publish_editor_terminal
    // DESC: Publishes only the visible shell composer while retaining the code editor's independent buffer.
    // ------------------=
    pub(super) fn publish_editor_terminal(&self) {
        terminal::mutate(|state|{state.input_length=self.command_length.min(state.input.len());state.input[..state.input_length].copy_from_slice(&self.command[..state.input_length]);state.cursor=self.command_cursor.min(state.input_length);});
        let geometry=self.editor_terminal_layout();let editor=self.editor_layout();
        let mut values=[0u64;72];values[0]=0x494e464554455231;values[1]=1;
        terminal::inspect(|state|{
            let count=state.visual_count(state.viewport_columns);
            values[3..12].copy_from_slice(&[state.preferences.open as u64,state.preferences.height as u64,state.focused as u64,state.attached as u64,state.running as u64,count as u64,state.input_length as u64,state.cursor as u64,state.scroll as u64]);
            values[12]=self.editor_document.cursor() as u64;values[13]=self.editor_document.bytes().len() as u64;
            let start=count.saturating_sub(geometry.rows(editor.scale)).saturating_sub(state.scroll);
            for visual in start..(start+geometry.rows(editor.scale)).min(count){
                let Some((row,_,_))=state.visual_line(visual,state.viewport_columns)else{continue;};
                if let Some(location)=terminal::diagnostic(&state.lines[row][..state.lengths[row]],&self.editor_document_path[..self.editor_document_path_length]){
                    values[14]+=1;
                    if values[14]==1{values[15]=location.line as u64;values[16]=location.column as u64;values[17]=(geometry.output.x+8) as u64;values[18]=(geometry.output.y+((visual-start)*18*editor.scale+8) as i32) as u64;}
                }
            }
            values[21]=state.streamed_bytes;
        });
        let before=&self.editor_document.bytes()[..self.editor_document.cursor().min(self.editor_document.bytes().len())];
        values[19]=before.iter().filter(|b|**b==b'\n').count() as u64+1;
        values[20]=before.iter().rposition(|b|*b==b'\n').map_or(before.len(),|at|before.len()-at-1) as u64+1;
        for (index,rect) in [geometry.panel,geometry.splitter,geometry.output,geometry.input,geometry.toggle,geometry.run,geometry.stop,geometry.restart,geometry.track,geometry.thumb].iter().enumerate(){
            values[24+index*4..28+index*4].copy_from_slice(&[rect.x as u64,rect.y as u64,rect.width as u64,rect.height as u64]);
        }
        values[64]=geometry.rows(editor.scale) as u64;values[65]=editor.body.height as u64;values[66]=editor.scale as u64;values[67]=(self.desktop_app==DesktopAppKind::TextEditor) as u64;
        unsafe{let target=core::ptr::addr_of_mut!(INFINITY_EDITOR_TERMINAL_SNAPSHOT).cast::<u64>();let sequence=core::ptr::read_volatile(target.add(2)).wrapping_add(2)&!1;
            core::ptr::write_volatile(target.add(2),sequence|1);core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
            for(index,value)in values.iter().enumerate(){if index!=2&&index!=71{core::ptr::write_volatile(target.add(index),*value);}}
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);core::ptr::write_volatile(target.add(71),sequence);core::ptr::write_volatile(target.add(2),sequence);
        }
    }
}
