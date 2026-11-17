//! Native desktop widget header capture and chooser dispatch.
use super::*;
use crate::ui::{
    desktop_widgets as widgets,
    geometry::{Point, Rect},
};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: surface_desktop
    // DESC: Minimizes existing application surfaces without closing tasks or discarding documents.
    // ------------------=
    fn surface_desktop(&mut self) {
        use crate::ui::app_launcher::minimized_shelf as shelf;
        self.store_active_app_window();
        self.checkpoint_active_file_navigator();
        for (window, id) in [(&mut self.editor_window,shelf::EDITOR),
            (&mut self.command_window,shelf::COMMAND),(&mut self.task_manager_window,shelf::TASKS),
            (&mut self.browser_window,shelf::BROWSER)] {
            if window.visible { shelf::set(id,true); window.visible=false; }
        }
        self.command_window_suspended = shelf::current().mask & (1 << shelf::COMMAND) != 0;
        if self.settings_open { shelf::set(shelf::SETTINGS,true); self.settings_open=false; }
        crate::runtime::with_runtime(|runtime| {
            while let Some(index)=runtime.file_navigators.active_index() {
                shelf::set(index,true);
                runtime.file_navigators.minimize_active();
            }
            let display=Rect{x:0,y:0,width:self.system.framebuffer_width as u32,height:self.system.framebuffer_height as u32};
            let scale=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).scale();
            let work=crate::runtime::native_gui::client_work_area(display,scale);
            let mut windows=[None;crate::ui::window::MAX_WINDOWS];
            for (index,slot) in windows.iter_mut().enumerate() {
                *slot=runtime.ui.windows.ordered_window(index).copied().filter(|w|w.visible && matches!(w.z_class,crate::ui::window::ZOrderClass::Normal|crate::ui::window::ZOrderClass::Floating));
            }
            for window in windows.into_iter().flatten() {
                let _=runtime.ui.windows.set_state(window.owner,window.id,crate::ui::window::WindowState::Minimized,work,display);
            }
        });
        self.home_window_visible=false;
        self.desktop_app=DesktopAppKind::None;
        self.mode=ConsoleMode::Desktop;
        self.shell_menu=0;
        let _=self.checkpoint_desktop_layout();
        self.redraw();
    }
    // ------------------------=
    // FUNC: desktop_widgets_pointer
    // DESC: Moves exposed widgets without capturing covered application input; persists only completed edits.
    // ------------------=
    pub(super) fn desktop_widgets_pointer(&mut self, clicked: bool, secondary: bool) -> bool {
        if !matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings)
            || self.editor_dialog != EditorDialog::None
            || self.app_window_dragging
            || self.app_window_resizing.is_some()
            || self.home_window_dragging
            || self.home_window_resizing.is_some()
            || self.settings_window_dragging
            || self.settings_window_resizing.is_some()
        {
            return false;
        }
        let mut state=widgets::current();
        // An idle widget has no hover action. Avoid reconstructing every window's
        // geometry and reading AI state for every mouse packet above a client.
        if !clicked && !secondary && state.drag.is_none() && state.menu.is_none(){return false;}
        let (width, height) = (
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        let layout = SystemLayout::new(width, height);
        let scale = layout.scale();
        let point = Point {
            x: self.pointer_x * width as i32 / 1000,
            y: self.pointer_y * height as i32 / 1000,
        };
        let minimized = crate::runtime::ai::with_ai_runtime(|r| r.chat.minimized());
        if state.drag.is_some() {
            let old = state;
            if self.pointer_pressed {
                state.move_pointer(point,true,width,height,scale,minimized);
            } else {
                let targets=crate::runtime::with_runtime(|runtime|core::array::from_fn::<_,8,_>(|i|
                    runtime.ui.windows.ordered_window(i).filter(|w|w.visible && w.z_class==crate::ui::window::ZOrderClass::Desktop)
                        .map_or(Rect::default(),|w|w.bounds))).unwrap_or([Rect::default();8]);
                state.move_with_neighbours(point,false,width,height,scale,minimized,&targets);
            }
            widgets::publish(state);
            if state.drag.is_none() {
                let _ = self.persist_desktop_layout();
            }
            if state != old {
                self.redraw();
            } else {
                crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            }
            return true;
        }
        if secondary && state.menu.is_some() {
            state.menu=None;state.menu_widget=None;state.menu_hover=None;
            state.adjusting=false;state.visibility_menu=false;state.transparency_menu=false;
            widgets::publish(state);
        }
        if let Some(menu) = state.menu_rect(width, height, scale) {
            if let Some(id)=state.menu_widget.filter(|_|state.transparency_menu) {
                let inset=20*scale as i32;
                let length=(menu.width as i32-2*inset).max(1);
                if state.adjusting || (clicked && menu.contains(point) && point.y>=menu.y+36*scale as i32) {
                    let value=((point.x-menu.x-inset).clamp(0,length)*100+length/2)/length;
                    state.set_percent(id,value as u32);
                    let release=state.adjusting && !self.pointer_pressed;
                    state.adjusting=self.pointer_pressed;widgets::publish(state);
                    if release || !state.adjusting { let _=self.persist_desktop_layout(); }
                    self.redraw();return true;
                }
            }
            if clicked {
                let previous=state;
                if menu.contains(point) {
                    let row = (point.y - menu.y) as usize / (36 * scale);
                    if let Some(id)=state.menu_widget.filter(|_|!state.transparency_menu) {
                        match row{
                            1 => match id {
                                2 => {
                                    #[cfg(any(target_arch="aarch64",target_arch="x86_64"))]
                                    {
                                        self.surface_desktop();
                                        super::weather::configure();
                                    }
                                }
                                1 => self.open_settings(3),
                                _ => self.open_settings(0),
                            },
                            2=>{state.transparency_menu=true;state.menu_hover=None;widgets::publish(state);self.redraw();return true;},
                            3=>{state.visible &= !(1<<id);if id==1{self.set_ai_chat_enabled(false);}},
                            _=>{}
                        }
                    }else if !state.visibility_menu && row==1 {
                        state.visibility_menu=true;state.menu_hover=None;widgets::publish(state);self.redraw();return true;
                    }else if state.visibility_menu && (1..=3).contains(&row) {
                        state.visible ^= 1 << (row - 1);
                        if row == 2 {
                            self.set_ai_chat_enabled(state.visible & 2 != 0);
                        }
                        self.ai_chat_focus = 0;
                    } else if row == 4 {
                        state.positions = [[0; 2]; 2];state.weather_position=[0;2];
                    }
                }
                state.menu = None;state.menu_hover=None;state.menu_widget=None;state.visibility_menu=false;state.transparency_menu=false;
                widgets::publish(state);
                if previous.visible!=state.visible || previous.transparency!=state.transparency
                    || previous.positions!=state.positions || previous.weather_position!=state.weather_position {
                    let _ = self.persist_desktop_layout();
                }
                self.redraw();
            } else {
                let row=(point.y-menu.y).max(0) as usize/(36*scale);
                let hover=(menu.contains(point) && row>0 && !state.transparency_menu).then_some(row);
                if state.menu_hover!=hover{
                    state.menu_hover=hover;widgets::publish(state);self.redraw();
                }else{crate::bootstrap::system_ui_cursor(self.pointer_x,self.pointer_y);}
            }
            return true;
        }
        // Desktop widgets are below windows: never drag through an application.
        let (e, c, t) = self.desktop_app_windows();
        let app_rect = |s: DesktopAppWindowState| {
            layout
                .desktop_app_window_geometry(s.x, s.y, s.width, s.height, s.maximized)
                .window
        };
        let (x, y, w, h) = layout.home_window_geometry_sized(
            self.home_window_x,
            self.home_window_y,
            self.home_window_width,
            self.home_window_height,
            self.home_window_maximized,
        );
        let home = Rect {
            x: x as i32,
            y: y as i32,
            width: w as u32,
            height: h as u32,
        };
        if self.inactive_file_navigator_at_pointer().is_some()
            || crate::ui::desktop_stack::current()
                .hit(
                    [
                        home,
                        app_rect(c),
                        app_rect(e),
                        app_rect(t),
                        layout.settings_window_geometry(self.settings_window).window,
                        app_rect(self.browser_window_state()),
                    ],
                    point,
                )
                .is_some()
        {
            return false;
        }
        for id in [1,0,2] {
            if !state.shown(id,self.browser_window_state().visible){continue}
            let r=state.rect(id,width,height,scale,minimized && id==1);
            if !r.contains(point){continue}
            if secondary{
                state.menu_widget=Some(id);state.menu=Some(point);
                widgets::publish(state);self.redraw();return true;
            }
            if clicked{
                if id==1{
                    let g=layout.ai_chat_geometry(minimized);
                    if [g.minimize,g.close].iter().any(|r|r.contains(point)) || !minimized && [g.status,g.model,g.timeline,g.composer,g.send].iter().any(|r|r.contains(point)){return false}
                }
                #[cfg(any(target_arch="aarch64",target_arch="x86_64"))]
                if id==2 && super::weather::click((point.x-r.x)*100/r.width.max(1) as i32,(point.y-r.y)*100/r.height.max(1) as i32){
                    if self.mode!=ConsoleMode::Desktop || self.desktop_app!=DesktopAppKind::None { self.surface_desktop(); }
                    self.ai_chat_focus=0;self.redraw();return true;
                }
                self.surface_desktop();
                state.drag=Some((id,point.x-r.x,point.y-r.y));self.ai_chat_focus=0;widgets::publish(state);return true;
            }
        }
        if point.y < (height * 7 / 100) as i32
            || layout.desktop_foreground_geometry().dock.contains(point)
        {
            return false;
        }
        if crate::ui::desktop_widgets::current().position_trash(crate::ui::desktop_polish::trash_rect(width,height,scale),width,height).contains(point) {
            if secondary {
                let mut menu = super::clipboard_menu::Menu::new();
                menu.open = true;
                menu.target = super::clipboard_menu::Target::Trash;
                menu.place(point.x,point.y,width,height,scale);
                super::clipboard_menu::publish(menu);
            } else if clicked {
                let tile=state.position_trash(crate::ui::desktop_polish::trash_rect(width,height,scale),width,height);
                self.trash_grab=Some((point.x-tile.x,point.y-tile.y,self.pointer_x,self.pointer_y,false));
            }
            self.redraw();
            return true;
        }
        if clicked && self.mode == ConsoleMode::Desktop {
            let files = crate::ui::file_preview::desktop_files();
            if let Some(file) = files.iter().enumerate().find_map(|(i,file)|{
                let rect = crate::ui::file_preview::desktop_rect(i,width,height);
                (file.length > 0 && rect.bottom() < height.saturating_sub(100*scale) as i32 && rect.contains(point)).then_some(*file)
            }) {
                self.desktop_file_drag = true;
                self.trash_drag_source = Some(file);
                let source = self.trash_drag_source.unwrap_or(crate::ui::file_preview::EMPTY_FILE);
                crate::ui::desktop_drag::set_name(crate::runtime::object_navigation::namespace_basename(&source.path[..source.length]));
                self.home_dragging_item = Some(7);
                self.home_drag_from_desktop = false;
                self.home_drag_moved = false;
                self.home_drag_origin_x = self.pointer_x;
                self.home_drag_origin_y = self.pointer_y;
                return true;
            }
        }
        if secondary {
            state.menu_widget=None;state.menu = Some(point);
            self.shell_menu = 0;
            widgets::publish(state);
            self.redraw();
            return true;
        }
        if clicked {
            self.surface_desktop();
        }
        false
    }
}
