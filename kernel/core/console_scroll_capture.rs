//! Pointer capture for OS scrollbars, before unrelated hover and widget dispatch.
use super::*;
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: start_navigator_rail
    // DESC: Captures File Navigator's painted rail without enumerating the directory during pointer motion.
    // ------------------=
    pub(super) fn start_navigator_rail(&mut self,layout:SystemLayout)->bool {
        if !self.home_window_visible{return false;}
        let w=layout.home_window_geometry_sized(self.home_window_x,self.home_window_y,self.home_window_width,self.home_window_height,self.home_window_maximized);
        let pos=layout.point(self.pointer_x,self.pointer_y);let scale=layout.scale();
        if pos.x<(w.0+w.2.saturating_sub(18*scale))as i32{return false;}
        let Some(state)=crate::runtime::with_runtime(|r|r.file_navigator).flatten()else{return false;};
        if state.menu_open.is_some()||state.dialog_open.is_some()||state.context_menu_open{return false;}
        let list=state.view_mode==crate::runtime::object_navigation::ViewMode::List;
        let count=navigator_child_count(state.active_namespace_ref.as_bytes());
        let extent=if list{34*scale}else{(self.system.framebuffer_height/23).max(34)+40*scale};
        let Some(rail)=crate::ui::scroll_capture::file_rail(w,scale,if list{count}else{count.div_ceil(4)},extent,state.scroll_offset)else{return false;};
        if pos.x>=rail.x+rail.width as i32||pos.y<rail.y||pos.y>=rail.y+rail.height as i32{return false;}
        let grab=if pos.y>=rail.thumb_y && pos.y<rail.thumb_y+rail.thumb_height as i32{pos.y-rail.thumb_y}else{rail.thumb_height as i32/2};
        self.navigator_rail_drag=Some((rail,grab));self.continue_scroll_capture(true,false)
    }
    // ------------------------=
    // FUNC: continue_scroll_capture
    // DESC: Keeps held scrollbar gestures attached to their owner through pointer drift and consumes the final release.
    // ------------------=
    pub(super) fn continue_scroll_capture(&mut self,held:bool,released:bool)->bool {
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        if let Some((rail,grab))=self.navigator_rail_drag {
            if self.mode!=ConsoleMode::Desktop||!self.home_window_visible{self.navigator_rail_drag=None;return false;}
            let scroll=crate::ui::scroll_capture::offset(layout.point(self.pointer_x,self.pointer_y).y,rail.y,rail.height,rail.thumb_height,grab,rail.maximum);
            crate::runtime::with_runtime(|r|{if let Some(n)=r.file_navigator.as_mut(){n.scroll_offset=scroll;}});
            if !held{self.navigator_rail_drag=None;}self.present_continuous_motion(!held);return true;
        }
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
        if self.mode==ConsoleMode::Desktop&&self.desktop_app==DesktopAppKind::TextEditor&&crate::ui::editor_chrome::terminal::inspect(|s|s.scroll_dragging) {
            return self.pointer_editor_terminal(false,released||!held);
        }
        if self.assistant_owner().is_some_and(|(id,_)|crate::ui::app_assistant::dragging(id)) {
            return self.pointer_window_assistant(false);
        }
        false
    }
}
