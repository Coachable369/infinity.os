//! Unified standalone terminal controller. Native command authority is unchanged.
use super::*;
use crate::ui::command_terminal::{self as terminal,Menu,Layout};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: command_terminal_visible
    // DESC: Restricts terminal interaction to the authenticated standalone Command Window.
    // ------------------=
    pub(super) fn command_terminal_visible(&self)->bool {
        !self.current_session.is_zero() && (self.mode==ConsoleMode::Console || self.mode==ConsoleMode::Desktop&&self.desktop_app==DesktopAppKind::CommandWindow)
            && crate::ui::installer_template::navigation::scene()==0 && !crate::ui::native_uikit::presentation().active
    }
    // ------------------------=
    // FUNC: publish_command_terminal
    // DESC: Revokes private scrollback on lock or identity change and synchronizes the inline local prompt.
    // ------------------=
    pub(super) fn publish_command_terminal(&self) {
        let active=!self.current_session.is_zero()&&!matches!(self.mode,ConsoleMode::Locked|ConsoleMode::Authentication|ConsoleMode::Startup);
        let revoke=terminal::inspect(|s|s.authorized&&(!active||s.owner!=self.current_session.0));
        if revoke {crate::ui::terminal_vt::mutate(|t|t.reset());}
        let load=terminal::mutate(|s|s.session(self.current_session.0,active));
        if load {crate::ui::terminal_vt::mutate(|t|t.reset());if let Ok(bytes)=crate::storage::command_terminal_preferences(self.current_user.0,self.current_session.0,None){terminal::mutate(|s|s.restore_settings(&bytes));}}
        if active {terminal::mutate(|s|s.publish(self.navigation_context.path(),&self.command[..self.command_length],self.command_cursor));}
        terminal::publish_snapshot();
    }
    // ------------------------=
    // FUNC: command_terminal_layout
    // DESC: Shares measured geometry with the framebuffer painter and SSH PTY dimensions.
    // ------------------=
    fn command_terminal_layout(&self)->Layout {
        let l=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let g=l.desktop_app_window_geometry(self.app_window_x,self.app_window_y,self.app_window_width,self.app_window_height,self.app_window_maximized);
        terminal::inspect(|s|s.layout(g.content,g.toolbar,l.scale()))
    }
    // ------------------------=
    // FUNC: command_terminal_save
    // DESC: Atomically saves preferences under the active user's identity, reporting storage failures visibly.
    // ------------------=
    fn command_terminal_save(&self) {
        let bytes=terminal::inspect(|s|s.settings());
        let saved=crate::storage::command_terminal_preferences(self.current_user.0,self.current_session.0,Some(bytes)).is_ok();
        terminal::mutate(|s|{s.notice=if saved{b"Preferences saved"}else{b"Could not save preferences"};s.changed();});
    }
    // ------------------------=
    // FUNC: command_terminal_adjust
    // DESC: Changes one selected preference; row and column limits use zero for dynamic sizing.
    // ------------------=
    fn command_terminal_adjust(&self,delta:isize) {
        terminal::mutate(|s|{match s.menu_row {
            0=>s.appearance.font=(s.appearance.font as isize+delta).rem_euclid(4)as u8,
            1=>s.appearance.size=(s.appearance.size as isize+delta).clamp(0,8)as u8,
            2=>s.appearance.scheme=(s.appearance.scheme as isize+delta).rem_euclid(10)as u8,
            3=>s.row_override=if s.row_override==0&&delta>0{24}else{(s.row_override as isize+delta).clamp(0,96)as usize},
            4=>s.column_override=if s.column_override==0&&delta>0{80}else{(s.column_override as isize+delta).clamp(0,320)as usize},
            5=>s.opacity=(s.opacity as isize+delta*13).clamp(64,255)as u8,
            6=>s.background=(s.background as isize+delta).rem_euclid(terminal::BACKGROUNDS.len() as isize) as u8,
            _=>{s.background=0;s.appearance=crate::ui::editor_tools::preferences::Preferences{scheme:1,font:2,size:3};s.row_override=0;s.column_override=0;s.opacity=255;}
        }s.changed();});self.command_terminal_save();
    }
    // ------------------------=
    // FUNC: command_terminal_key
    // DESC: Handles preferences and keyboard help without injecting their navigation into a shell or remote PTY.
    // ------------------=
    pub(super) fn command_terminal_key(&mut self,key:ConsoleKey)->bool {
        if !self.command_terminal_visible()||crate::ui::terminal_edit::inspect(|s|s.active){return false;}
        if matches!(key,ConsoleKey::Shortcut(b',')){self.shell_menu=0;terminal::mutate(|s|{s.menu=Menu::Appearance;s.menu_row=0;s.setting_edit=false;s.changed();});return true;}
        if terminal::inspect(|s|s.menu!=Menu::None){
            if terminal::inspect(|s|matches!(s.menu_row,3|4)) {
                if let ConsoleKey::Character(b'0'..=b'9')=key {
                    if let ConsoleKey::Character(b)=key {terminal::mutate(|s|{if !s.setting_edit{s.setting_value=0;}s.setting_edit=true;s.setting_value=(s.setting_value*10+(b-b'0')as usize).min(if s.menu_row==3{96}else{320});s.changed();});}return true;
                }
                if terminal::inspect(|s|s.setting_edit){match key {
                    ConsoleKey::Backspace=>{terminal::mutate(|s|{s.setting_value/=10;s.changed();});return true;},
                    ConsoleKey::Enter=>{terminal::mutate(|s|{if s.menu_row==3{s.row_override=s.setting_value;}else{s.column_override=s.setting_value;}s.setting_edit=false;s.changed();});self.command_terminal_save();return true;},
                    _=>terminal::mutate(|s|s.setting_edit=false)
                }}
            }
            match key {
                ConsoleKey::Escape=>terminal::mutate(|s|s.menu=Menu::None),
                ConsoleKey::Up=>terminal::mutate(|s|s.menu_row=(s.menu_row+terminal::PREFERENCE_ROWS-1)%terminal::PREFERENCE_ROWS),
                ConsoleKey::Down|ConsoleKey::Tab(false)=>terminal::mutate(|s|s.menu_row=(s.menu_row+1)%terminal::PREFERENCE_ROWS),
                ConsoleKey::Left=>self.command_terminal_adjust(-1),
                ConsoleKey::Right|ConsoleKey::Enter=>self.command_terminal_adjust(1),
                ConsoleKey::Character(b'0')=>{terminal::mutate(|s|{if s.menu_row==3{s.row_override=0;}if s.menu_row==4{s.column_override=0;}});self.command_terminal_save();}
                _=>()
            }return true;
        }
        if terminal::inspect(|s|s.help){terminal::mutate(|s|{match key {
            ConsoleKey::Escape=>if s.help_detail{s.help_detail=false;}else{s.help=false;},
            ConsoleKey::Up=>s.help_selected=s.help_selected.saturating_sub(1),
            ConsoleKey::Down=>s.help_selected=(s.help_selected+1).min(crate::runtime::console_help::COMMANDS.len()-1),
            ConsoleKey::Enter=>s.help_detail=true,_=>()
        }s.changed();});return true;}
        if matches!(key,ConsoleKey::Shortcut(b'c'))&&!terminal::inspect(|s|s.remote){let mut bytes=[0;8192];if let Some(n)=terminal::inspect(|s|s.copy(&mut bytes)){if n>0{self.copy_workplace_text(&bytes[..n]);return true;}}}
        if let ConsoleKey::Shortcut(code)=key {if let Some(p)=terminal::inspect(|s|s.appearance.zoom_shortcut(code)) {
            terminal::mutate(|s|{s.appearance=p;s.changed();});self.command_terminal_save();return true;
        }}false
    }
    // ------------------------=
    // FUNC: scroll_command_terminal
    // DESC: Scrolls retained output independently of the editable command and remote process lifecycle.
    // ------------------=
    pub(super) fn scroll_command_terminal(&mut self,amount:i8)->bool {
        if !self.command_terminal_visible()||crate::ui::terminal_edit::inspect(|s|s.active){return false;}
        let l=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        if !self.command_terminal_layout().body.contains(l.point(self.pointer_x,self.pointer_y)){return false;}
        terminal::mutate(|s|s.scroll_by(amount as isize*3));self.redraw();true
    }
    // ------------------------=
    // FUNC: command_terminal_pointer
    // DESC: Opens the shared preferences menu, selects rows, and selects transcript text without intercepting window controls.
    // ------------------=
    pub(super) fn command_terminal_pointer(&mut self,clicked:bool,held:bool)->bool {
        if !self.command_terminal_visible()||crate::ui::terminal_edit::inspect(|s|s.active)||self.app_window_dragging{return false;}
        let l=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);let point=l.point(self.pointer_x,self.pointer_y);let g=self.command_terminal_layout();
        let headers=Layout::header_menus(g.toolbar,g.scale);
        if clicked&&headers[1].contains(point){self.shell_menu=0;terminal::mutate(|s|{s.menu=if s.menu==Menu::None{Menu::Appearance}else{Menu::None};s.menu_row=0;s.setting_edit=false;s.changed();});return true;}
        if terminal::inspect(|s|s.menu!=Menu::None)&&clicked {
            let (rect,row)=terminal::inspect(|s|g.menu_rect(s));
            if rect.contains(point){terminal::mutate(|s|{s.menu_row=((point.y-rect.y)as usize/row).min(terminal::PREFERENCE_ROWS-1);s.setting_edit=false;});self.command_terminal_adjust(1);}
            else{terminal::mutate(|s|{s.menu=Menu::None;s.setting_edit=false;s.changed();});return false;}return true;
        }
        if self.shell_menu==20{return false;}
        if clicked&&g.text.contains(point){terminal::mutate(|s|{let at=g.at(point,s);s.selection=Some((at,at));s.dragging=true;s.changed();});return true;}
        if terminal::inspect(|s|s.dragging){terminal::mutate(|s|{if held{let at=g.at(point,s);if let Some((start,_))=s.selection{s.selection=Some((start,at));}}else{s.dragging=false;}s.changed();});return true;}false
    }
}
