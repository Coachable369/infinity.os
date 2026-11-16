//! Shared native glass chooser, also exercised by the host pixel harness.
use super::*;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: desktop_widget_menu
    // DESC: Draws the glass desktop chooser with checked visibility rows above application layers.
    // ------------------=
    pub(super) fn desktop_widget_menu(&mut self, scale: usize) {
        let state = crate::ui::desktop_widgets::current();
        let Some(rect) = state.menu_rect(self.width, self.height, scale) else {
            return;
        };
        let (x, y) = (rect.x.max(0) as usize, rect.y.max(0) as usize);
        self.glass_panel(x, y, rect.width as usize, rect.height as usize, true);
        if let Some(id)=state.menu_widget{
            if state.transparency_menu {
                self.ui_text_strong(x+16*scale,y+10*scale,b"TRANSPARENCY",120,209,245,1);
                for (index,label) in [b"0%".as_slice(),b"25%",b"50%",b"75%"].iter().enumerate(){
                    let top=y+(46+36*index)*scale;
                    if usize::from((state.transparency>>(2*id))&3)==index {self.ui_text(x+14*scale,top,b"*",120,209,245,1);}
                    self.ui_text(x+40*scale,top,label,222,237,249,1);
                }
                return;
            }
            let (title,action):(&[u8],&[u8])=match id{2=>(b"WEATHER",b"Set location..."),1=>(b"INFINITY AI",b"AI & Voice settings"),_=>(b"SYSTEM OVERVIEW",b"System settings")};
            self.ui_text_strong(x+16*scale,y+10*scale,title,120,209,245,1);
            self.ui_text(x+16*scale,y+46*scale,action,222,237,249,1);
            self.ui_text(x+16*scale,y+82*scale,b"Set transparency...",222,237,249,1);
            self.ui_text(x+16*scale,y+118*scale,b"Hide widget",222,237,249,1);return;
        }
        if !state.visibility_menu {
            self.ui_text_strong(x+16*scale,y+10*scale,b"DESKTOP",120,209,245,1);
            self.ui_text(x+16*scale,y+46*scale,b"Widgets  >",222,237,249,1);
            return;
        }
        self.ui_text_strong(x + 16 * scale, y + 10 * scale, b"WIDGETS", 120, 209, 245, 1);
        for (i, label) in [
            b"System Overview".as_slice(),
            b"AI Chat",
            b"Weather",
            b"Reset positions",
        ]
        .iter()
        .enumerate()
        {
            let top = y + (46 + i * 36) * scale;
            self.ui_text(x + 40 * scale, top, label, 222, 237, 249, 1);
            if i < 3 && state.visible & (1 << i) != 0 {
                let (a, b) = ((x + 15 * scale) as i32, (top + 7 * scale) as i32);
                self.icon_line(
                    a,
                    b,
                    a + 4 * scale as i32,
                    b + 4 * scale as i32,
                    (105, 219, 255),
                    12 * scale,
                );
                self.icon_line(
                    a + 4 * scale as i32,
                    b + 4 * scale as i32,
                    a + 12 * scale as i32,
                    b - 4 * scale as i32,
                    (105, 219, 255),
                    12 * scale,
                );
            }
        }
    }
}
