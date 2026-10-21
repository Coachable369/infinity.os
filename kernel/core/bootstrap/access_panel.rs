//! UI-thread-only scratch composition; no captured credentials survive a paint.
use super::{access_scale, access_style, DisplayDevice};
const CAPACITY: usize = 2 * 1024 * 1024;
static mut PANEL_PIXELS: [u32; CAPACITY] = [0; CAPACITY];

impl DisplayDevice {
    // ------------------------=
    // FUNC: access_panel_entrance
    // DESC: Scales freshly painted panel pixels over restored wallpaper, preserving the native panel shape.
    // ------------------=
    pub(super) fn access_panel_entrance(&mut self, rect: (usize,usize,usize,usize), configuration: bool) {
        let motion=access_style::panel_motion();
        if motion.extent==1024 { return; }
        let (left,top,width,height)=rect;
        let Some(length)=width.checked_mul(height) else { return; };
        if length==0 || length>CAPACITY || left.saturating_add(width)>self.width
            || top.saturating_add(height)>self.height { return; }
        // A partial caret repaint cannot supply a complete source panel.
        let Some(region)=self.clipped_render_region(left,top,width,height) else { return; };
        if (region.left,region.top,region.right,region.bottom)!=(left,top,left+width,top+height) { return; }
        unsafe {
            let pixels=core::slice::from_raw_parts_mut((&raw mut PANEL_PIXELS).cast::<u32>(),length);
            for row in 0..height {
                core::ptr::copy_nonoverlapping(self.buffer.add((top+row)*self.stride+left),
                    pixels.as_mut_ptr().add(row*width),width);
            }
            let previous=self.render_clip;
            self.intersect_render_clip(left,top,width,height);
            #[cfg(studio_native_preview)]
            self.paint_bitmap_cover_rect(crate::studio_native_render::background(),0,0,self.width,self.height);
            #[cfg(not(studio_native_preview))]
            {if configuration { self.paint_first_boot_background(); }
            else { self.paint_authentication_background_rect(left,top,width,height); }}
            let (x,y,w,h)=motion.rectangle(rect);
            for row in 0..h {
                for col in 0..w {
                    *self.buffer.add((y+row)*self.stride+x+col)=
                        access_scale::sample(pixels,width,height,col,row,w,h);
                }
            }
            pixels.fill(0);
            self.render_clip=previous;
        }
        self.mark_dirty_rect(left,top,width,height);
    }
}
