//! Pointer capture for OS scrollbars, before unrelated hover and widget dispatch.
use super::*;
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: continue_scroll_capture
    // DESC: Keeps held scrollbar gestures attached to their owner through pointer drift and consumes the final release.
    // ------------------=
    pub(super) fn continue_scroll_capture(&mut self,held:bool,released:bool)->bool {
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        if self.settings_scroll_dragging&&self.mode==ConsoleMode::Settings {
            if held||released {let offset=layout.settings_scroll_offset_for_thumb_in_section(self.pointer_y,self.settings_window,self.settings_scroll_grab_offset,self.system_focus);
                self.settings_window.scroll_offset=offset;self.settings_scroll_target=offset;}
            if !held{self.settings_scroll_dragging=false;}self.present_continuous_motion(!held);return true;
        }
        if self.editor_scroll_dragging&&self.mode==ConsoleMode::Desktop&&self.desktop_app==DesktopAppKind::TextEditor {
            if held||released{self.editor_scroll_row=layout.desktop_editor_scroll_offset_for_thumb(self.pointer_y,self.editor_scroll_geometry(),self.editor_scroll_grab_offset);}
            if !held{self.editor_scroll_dragging=false;}self.present_continuous_motion(!held);return true;
        }
        if let Some(grab)=self.ai_chat_scroll_grab {
            if self.mode!=ConsoleMode::Desktop{self.ai_chat_scroll_grab=None;return false;}
            let (enabled,minimized,offset,maximum)=crate::runtime::ai::with_ai_runtime(|r|(r.chat.enabled(),r.chat.minimized(),r.chat.timeline_scroll_offset(),r.chat.timeline_maximum_scroll()));
            if enabled&&!minimized&&(held||released){
                let viewport=layout.ai_chat_geometry(false).transcript_viewport(layout.scale());
                let g=layout.ai_chat_scroll_geometry(false,maximum.saturating_add(viewport.height as usize),offset);
                let next=crate::ui::scroll_capture::offset(layout.point(self.pointer_x,self.pointer_y).y,g.track.y,g.track.height,g.thumb.height,grab,maximum);
                crate::runtime::ai::with_ai_runtime(|r|r.chat.scroll_timeline(next as isize-offset as isize));
            }
            if !held||!enabled||minimized{self.ai_chat_scroll_grab=None;}
            self.present_continuous_motion(!held);return true;
        }
        if self.mode==ConsoleMode::Desktop&&self.desktop_app==DesktopAppKind::TextEditor&&crate::ui::editor_chrome::terminal::read().scroll_dragging {
            return self.pointer_editor_terminal(false,released||!held);
        }
        if self.assistant_owner().is_some_and(|(id,_)|crate::ui::app_assistant::dragging(id)) {
            return self.pointer_window_assistant(false);
        }
        false
    }
}
