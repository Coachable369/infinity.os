//! Topmost-owner hover routing for shared window chrome and authored Settings controls.
use super::*;
use crate::ui::{geometry::{Point,Rect},app_launcher::window_controls as controls};
#[no_mangle]
pub static mut INFINITY_WINDOW_CONTROL_SNAPSHOT: [u64;81]=[0;81];

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: publish_window_control_snapshot
    // DESC: Exposes non-secret lifecycle and button centers for read-only installed input acceptance.
    // ------------------=
    pub(super) fn publish_window_control_snapshot(&self) {
        let l=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let (e,c,t)=self.desktop_app_windows();
        let b=self.browser_window_state();
        let (x,y,w,h)=l.home_window_geometry_sized(self.home_window_x,self.home_window_y,self.home_window_width,self.home_window_height,self.home_window_maximized);
        let home=Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32};
        let settings=l.settings_window_geometry(self.settings_window);
        let app=|s:DesktopAppWindowState|l.desktop_app_window_geometry(s.x,s.y,s.width,s.height,s.maximized);
        let windows=[home,app(c).window,app(e).window,app(t).window,settings.window,app(b).window];
        let maximized=[self.home_window_maximized,c.maximized,e.maximized,t.maximized,self.settings_window.maximized,b.maximized];
        let stack=crate::ui::desktop_stack::current();
        let shelf=crate::ui::app_launcher::minimized_shelf::current();
        let rail=crate::ui::app_launcher::minimized_shelf::Geometry::new(self.system.framebuffer_width,self.system.framebuffer_height,shelf);
        unsafe {
            let p=&mut *(&raw mut INFINITY_WINDOW_CONTROL_SNAPSHOT);
            p[0]=0x494e465743545231;p[1]=1;p[2]=p[2].wrapping_add(1)|1;
            p[3]=shelf.mask as u64;p[4]=stack.active as u64;
            p[5]=rail.rail.x as u64;p[6]=rail.rail.y as u64;p[7]=rail.tile as u64;
            for id in 0..6 {
                let r=windows[id];
                let mut buttons=controls::bounds(r,if id==0 {34*l.scale() as u32}else if id==4 {settings.title.height}else{48*l.scale() as u32},l.scale());
                #[cfg(feature="native-browser")]
                if id==5 {
                    let scale=l.scale().max(1).min((r.width as usize/760).max(1));
                    if let Some(b)=infinity_browser_core::layout::Layout::new(r.width,r.height,scale as u32) {
                        buttons=[b.minimize,b.maximize,b.close].map(|v|Rect{x:r.x+v.x,y:r.y+v.y,width:v.width,height:v.height});
                    }
                }
                let at=8+id*12;
                p[at]=stack.visible[id] as u64;p[at+1]=maximized[id] as u64;
                p[at+2]=r.x as u64;p[at+3]=r.y as u64;p[at+4]=r.width as u64;p[at+5]=r.height as u64;
                for (i,r) in buttons.into_iter().enumerate() {
                    p[at+6+i*2]=(r.x+r.width as i32/2) as u64;
                    p[at+7+i*2]=(r.y+r.height as i32/2) as u64;
                }
            }
            p[2]=p[2].wrapping_add(1)&!1;p[80]=p[2];
        }
    }
    // ------------------------=
    // FUNC: dispatch_window_control
    // DESC: Gives title controls priority over overlapping app menus while preserving modal guards.
    // ------------------=
    pub(super) fn dispatch_window_control(&mut self,layout:SystemLayout)->bool {
        if self.editor_dialog!=EditorDialog::None || self.spatial.open || self.app_window_dragging
            || self.home_window_dragging || self.settings_window_dragging
            || self.app_window_resizing.is_some() || self.home_window_resizing.is_some()
            || self.settings_window_resizing.is_some() {return false;}
        if self.mode==ConsoleMode::Settings {
            let Some(SettingsTarget::WindowControl(index))=layout.settings_target(self.pointer_x,self.pointer_y,self.settings_window) else {return false;};
            if index==1 {self.toggle_window_maximized(4);}
            else {
                crate::ui::app_launcher::minimized_shelf::set(crate::ui::app_launcher::minimized_shelf::SETTINGS,index==0);
                self.settings_open=false; self.enter_desktop();
                let _=self.checkpoint_desktop_layout();
            }
            return true;
        }
        if self.mode!=ConsoleMode::Desktop || self.desktop_app==DesktopAppKind::None {return false;}
        #[cfg(feature="native-browser")]
        if self.desktop_app==DesktopAppKind::Browser {
            let g=layout.desktop_app_window_geometry(self.app_window_x,self.app_window_y,self.app_window_width,self.app_window_height,self.app_window_maximized);
            let scale=layout.scale().max(1).min((g.window.width as usize/760).max(1));
            if let Some(b)=infinity_browser_core::layout::Layout::new(g.window.width,g.window.height,scale as u32) {
                let x=self.system.framebuffer_width as i32*self.pointer_x/1000-g.window.x;
                let y=self.system.framebuffer_height as i32*self.pointer_y/1000-g.window.y;
                use infinity_browser_core::layout::Control;
                match b.hit(x,y) {
                    Some(Control::Minimize)=>self.minimize_desktop_app(),
                    Some(Control::Maximize)=>self.toggle_window_maximized(5),
                    Some(Control::Close)=>self.close_desktop_app(),
                    _=>return false,
                }
                return true;
            }
            return false;
        }
        match layout.desktop_app_window_target(self.pointer_x,self.pointer_y,self.app_window_x,self.app_window_y,
            self.app_window_width,self.app_window_height,self.app_window_maximized,self.desktop_app==DesktopAppKind::TextEditor) {
            DesktopAppWindowTarget::Minimize=>self.minimize_desktop_app(),
            DesktopAppWindowTarget::Close=>self.close_desktop_app(),
            DesktopAppWindowTarget::Maximize=>self.toggle_window_maximized(1),
            _=>return false,
        }
        true
    }
    // ------------------------=
    // FUNC: publish_window_controls
    // DESC: Publishes only an uncovered title control or authored Settings action, respecting modal and drag capture.
    // ------------------=
    pub(super) fn publish_window_controls(&self)->bool {
        if !matches!(self.mode,ConsoleMode::Desktop|ConsoleMode::Settings) || self.spatial.open
            || self.editor_dialog!=EditorDialog::None || self.app_window_dragging
            || self.home_window_dragging || self.settings_window_dragging
            || self.app_window_resizing.is_some() || self.home_window_resizing.is_some()
            || self.settings_window_resizing.is_some() {
            return controls::publish(None);
        }
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let point=Point{x:self.system.framebuffer_width as i32*self.pointer_x/1000,
            y:self.system.framebuffer_height as i32*self.pointer_y/1000};
        let stack=crate::ui::desktop_stack::current();
        let (editor,command,tasks)=self.desktop_app_windows();
        let app=|state:DesktopAppWindowState|layout.desktop_app_window_geometry(state.x,state.y,state.width,state.height,state.maximized);
        for id in stack.order.into_iter().rev() {
            if !stack.visible[id] {continue;}
            let (window,buttons)=if id==0 {
                let target=crate::runtime::with_runtime(|r| {
                    for layer in (0..crate::runtime::object_navigation::MAX_FILE_NAVIGATOR_INSTANCES).rev() {
                        let Some((_,n))=r.file_navigators.back_to_front(layer) else {continue;};
                        if !n.visible {continue;}
                        let (x,y,w,h)=layout.home_window_geometry_sized(n.x,n.y,n.width,n.height,n.maximized);
                        let window=Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32};
                        if window.contains(point) {return Some(window);}
                    }
                    None
                }).flatten();
                let Some(window)=target else {continue;};
                (window,controls::bounds(window,(34*layout.scale()) as u32,layout.scale()))
            } else if id==4 {
                let g=layout.settings_window_geometry_for_section(self.settings_window,stack.settings_section);
                (g.window,controls::bounds(g.window,g.title.height,layout.scale()))
            } else {
                let g=app(match id {1=>command,2=>editor,5=>self.browser_window_state(),_=>tasks});
                let mut buttons=[g.minimize,g.maximize,g.close];
                #[cfg(feature="native-browser")]
                if id==5 {
                    let scale=layout.scale().max(1).min((g.window.width as usize/760).max(1));
                    if let Some(b)=infinity_browser_core::layout::Layout::new(g.window.width,g.window.height,scale as u32) {
                        buttons=[b.minimize,b.maximize,b.close].map(|r| {
                            let size=r.width.min(r.height);
                            Rect{x:g.window.x+r.x+((r.width-size)/2) as i32,
                                y:g.window.y+r.y+((r.height-size)/2) as i32,width:size,height:size}
                        });
                    }
                }
                (g.window,buttons)
            };
            if window.contains(point) {
                let highlight=controls::hit(buttons,point).map(|index|
                    controls::Highlight{rect:buttons[index],pressed:self.pointer_pressed})
                    .or_else(|| {
                        if id!=4 {return None;}
                        let row=self.settings_window.expanded_row?;
                        let action=layout.settings_action_geometry(self.settings_window,stack.settings_section,row)?;
                        action.hit(point).then_some(controls::Highlight{rect:action.bounds,pressed:self.pointer_pressed})
                    });
                return controls::publish(highlight);
            }
        }
        controls::publish(None)
    }
}
