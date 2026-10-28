//! Dispatches authored desktop menus through existing session-authorized OS actions.
use super::*;
use crate::ui::installer_template::navigation::{self,Destination};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: authored_menu_key
    // DESC: Handles submenu traversal, type selection and paging without executing disabled entries.
    // ------------------=
    pub(super) fn authored_menu_key(&mut self,key:ConsoleKey)->bool {
        if self.mode!=ConsoleMode::SystemMenu||self.shell_menu!=0{return false;}
        let Some(menu)=crate::ui::desktop_template::menu()else{return false;};let mut view=navigation::view();
        match key {
            ConsoleKey::Escape|ConsoleKey::Left=>{if !view.back(&menu){self.close_shell_menu();}navigation::publish(view);}
            ConsoleKey::Up|ConsoleKey::Tab(true)=>{view.step(&menu,false);navigation::publish(view);}
            ConsoleKey::Down|ConsoleKey::Tab(false)=>{view.step(&menu,true);navigation::publish(view);}
            ConsoleKey::Home=>{view.selected=view.count(&menu).saturating_sub(1);view.step(&menu,true);navigation::publish(view);}
            ConsoleKey::End=>{view.selected=0;view.step(&menu,false);navigation::publish(view);}
            ConsoleKey::Enter=>self.activate_authored_menu(),
            ConsoleKey::Right=>{if view.item(&menu,view.selected).is_some_and(|i|matches!(menu.destination(i),Some(Destination::Submenu(_)))){self.activate_authored_menu();}}
            ConsoleKey::Character(b)=>{for row in 0..view.count(&menu){if view.item(&menu,row).is_some_and(|i|menu.items[i].label.first().is_some_and(|v|v.eq_ignore_ascii_case(&b))){view.select(&menu,row);break;}}navigation::publish(view);}
            _=>{}
        }
        if self.mode==ConsoleMode::SystemMenu&&self.shell_menu==0{self.system_focus=navigation::view().selected;}
        true
    }
    // ------------------------=
    // FUNC: activate_authored_menu
    // DESC: Routes validated destinations only after session authorization, preserving power confirmations.
    // ------------------=
    pub(super) fn activate_authored_menu(&mut self) {
        if !self.workplace_authorized(false){self.close_shell_menu();return;}
        let Some(menu)=crate::ui::desktop_template::menu()else{return;};let mut view=navigation::view();
        if view.parent.is_some()&&view.selected==0 {view.back(&menu);navigation::publish(view);return;}
        let Some(destination)=view.item(&menu,view.selected).and_then(|i|menu.destination(i))else{return;};
        match destination {
            Destination::Submenu(index)=>{view.enter(&menu,index);navigation::publish(view);}
            Destination::System(index)=>self.activate_default_infinity_item(index as usize),
            Destination::Settings(screen)=>self.open_settings(screen.saturating_sub(1)as usize),
            Destination::Application(app)=>{
                self.close_shell_menu();
                match app {0=>self.open_command_window(),1=>self.open_text_editor(),2=>{let _=self.open_file_navigator_window(home_location_path(0));},
                    3=>{#[cfg(feature="native-browser")]browser_controller::open_home(self);},4=>self.open_app_launcher(),_=>{}}
            }
            Destination::Screen(id)|Destination::Window(id)=>{
                if crate::ui::native_uikit::presentation().active {self.close_shell_menu();return;}
                if !self.ensure_app_task(crate::runtime::task_manager::IMAGE_COMMAND_WINDOW){return;}
                self.close_shell_menu();self.store_active_app_window();self.mode=ConsoleMode::Desktop;
                if self.authored_host.is_none() { self.authored_host=Some((self.command_window,self.command_window_suspended)); }
                self.desktop_app=DesktopAppKind::CommandWindow;self.command_window.visible=true;
                crate::ui::app_launcher::minimized_shelf::set(crate::ui::app_launcher::minimized_shelf::COMMAND,false);
                self.load_active_app_window();
                if !self.app_window_maximized {
                    self.app_window_restore_x=self.app_window_x;self.app_window_restore_y=self.app_window_y;
                    self.app_window_restore_width=self.app_window_width;self.app_window_restore_height=self.app_window_height;
                }
                self.app_window_maximized=matches!(destination,Destination::Screen(_));
                self.ai_chat_focus=0;navigation::set_scene(id);self.store_active_app_window();let _=self.checkpoint_desktop_layout();
            }
        }
    }
    // ------------------------=
    // FUNC: close_authored_destination
    // DESC: Restores the borrowed native host without resetting the terminal's command or output buffers.
    // ------------------=
    pub(super) fn close_authored_destination(&mut self)->bool {
        if self.desktop_app!=DesktopAppKind::CommandWindow || navigation::scene()==0 {return false;}
        let Some((window,suspended))=self.authored_host.take()else{return false;};
        navigation::set_scene(0);self.command_window=window;self.command_window_suspended=suspended;
        self.load_active_app_window();
        if !window.visible {self.desktop_app=DesktopAppKind::None;}
        self.app_window_dragging=false;self.app_window_resizing=None;
        crate::ui::app_launcher::minimized_shelf::set(crate::ui::app_launcher::minimized_shelf::COMMAND,suspended);
        let _=self.checkpoint_desktop_layout();true
    }
    // ------------------------=
    // FUNC: authored_menu_pointer
    // DESC: Shares renderer geometry for disabled rows, padding and click-away dismissal.
    // ------------------=
    pub(super) fn authored_menu_pointer(&mut self,clicked:bool)->bool {
        if self.mode!=ConsoleMode::SystemMenu||self.shell_menu!=0{return false;}
        let Some(menu)=crate::ui::desktop_template::menu()else{return false;};
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let p=layout.point(self.pointer_x,self.pointer_y);let (x,y,w,h,count)=layout.system_menu_geometry(0);
        let rect=crate::ui::geometry::Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32};
        if !rect.contains(p){if clicked{self.close_shell_menu();}return true;}
        let scale=layout.scale();let offset=(p.y-y as i32-11*scale as i32).max(0)as usize;let row=offset/(34*scale);
        if p.y>=y as i32+11*scale as i32&&row<count {
            let mut view=navigation::view();let index=view.offset+row;
            if view.selectable(&menu,index){view.select(&menu,index);navigation::publish(view);self.system_focus=view.selected;if clicked{self.activate_authored_menu();}}
        }true
    }
    // ------------------------=
    // FUNC: authored_menu_scroll
    // DESC: Makes long menus pointer-scrollable using the same enabled-row navigation as keyboard input.
    // ------------------=
    pub(super) fn authored_menu_scroll(&mut self,vertical:i8)->bool {
        if self.mode!=ConsoleMode::SystemMenu||self.shell_menu!=0{return false;}
        let Some(menu)=crate::ui::desktop_template::menu()else{return false;};let mut view=navigation::view();
        for _ in 0..vertical.unsigned_abs()as usize{view.step(&menu,vertical<0);}
        navigation::publish(view);self.system_focus=view.selected;self.redraw();true
    }
    // ------------------------=
    // FUNC: activate_default_infinity_item
    // DESC: Keeps native action identities independent of authored labels and ordering.
    // ------------------=
    fn activate_default_infinity_item(&mut self,index:usize) {
        match index {
            0=>self.open_settings(9),1=>self.open_settings(0),2=>self.open_settings(2),3=>self.open_settings(3),4=>self.open_settings(6),5=>self.open_settings(4),
            6=>{let _=self.lock_session_preserving_desktop(false);},
            7=>{let _=self.persist_desktop_layout();let _=crate::runtime::with_runtime(|r|r.identity.end_session(self.current_session,self.current_user));
                self.current_session=crate::runtime::identity::StableId::zero();self.reset_app_assistant_session();self.mode=ConsoleMode::Authentication;self.system_focus=0;self.reset_input();},
            8=>self.open_shell_menu(17),9=>self.open_shell_menu(18),_=>{}
        }
    }
}
