//! Shared drag overlay, rendered unchanged by the guest and pixel-proof harness.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: desktop_drag_overlay
    // DESC: Reuses the themed icon with a finite cyan lift/drop halo above retained windows.
    // ------------------=
    pub(super) fn desktop_drag_overlay(&mut self) {
        let Some(view) = crate::ui::desktop_drag::current() else { return; };
        let x = self.width * view.position[0] as usize / 1000;
        let y = self.height * view.position[1] as usize / 1000;
        let size = (self.height / 21).max(44);
        for pad in (3..=9).rev().step_by(3) {
            self.fill_rounded_rect_alpha(x.saturating_sub(pad), y.saturating_sub(pad),
                size + 2*pad, size + 2*pad, 12, 24, 158, 215,
                (u16::from(view.emphasis) * 18 / 255) as u8);
        }
        // Redraw the foreground icon over the highlight; artwork never scales
        // or lags behind the pointer. The persistent desktop owns the drop.
        let (label,kind): (&[u8],usize) = match view.item {
            0=>(b"Documents",0),1=>(b"Downloads",1),2=>(b"Pictures",0),
            3=>(b"Music",0),4=>(b"Videos",0),5=>(b"Projects",2),_ =>(b"notes.txt",3),
        };
        let role=[4,5,6,7,8,9,49][view.item.min(6)];
        let _=kind;
        self.themed_icon(x+size/2,y+size/2,role,size);
        let label_width=self.ui_text_width(label,1);
        self.ui_text(x+size.saturating_sub(label_width)/2,y+size+8,label,221,235,243,1);
    }
}
