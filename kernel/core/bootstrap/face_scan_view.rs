//! Illustrative scan mesh, not measured landmarks or proof of recognition.
use super::{DisplayDevice,CONSOLE};
use crate::ui::{geometry::Rect,face_setup_layout::Layout};
const MASK:&[u8]=include_bytes!("../../../assets/ui-design-kit/default/face-mask-v1.bmp");
const LIMIT:usize=480;
static mut BASE:[u32;LIMIT*LIMIT]=[0;LIMIT*LIMIT];
static mut BOUNDS:Option<Rect>=None;
static mut LAST_MS:u64=0;
static mut START_MS:Option<u64>=None;

// ------------------------=
// FUNC: active
// DESC: Animates only real in-flight capture/comparison commands, never idle or model loading.
// ------------------=
fn active()->bool {
    crate::runtime::face_setup_host::view(|s| s.pending() && matches!(s.stage(),
        crate::runtime::face_setup::Stage::Photos(_) | crate::runtime::face_setup::Stage::Verifying(_)))
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: face_mask_base
    // DESC: Paints the transparent authored mask and caches its clean substrate for bounded animation damage.
    // ------------------=
    pub(super) fn face_mask_base(&mut self,r:Rect) {
        self.paint_bitmap_alpha_fit_rect(MASK,r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        unsafe {
            BOUNDS=None;
            if r.width as usize>LIMIT || r.height as usize>LIMIT || r.x<0 || r.y<0
                || r.right()>self.width as i32 || r.bottom()>self.height as i32 {return;}
            let base=&mut *(&raw mut BASE);
            for y in 0..r.height as usize {for x in 0..r.width as usize {
                base[y*LIMIT+x]=self.framebuffer_pixel(r.x as usize+x,r.y as usize+y);
            }}
            BOUNDS=Some(r);
        }
    }

    // ------------------------=
    // FUNC: face_mesh_pixel
    // DESC: Clips soft mesh illumination to actual mask alpha, including its eye openings.
    // ------------------=
    fn face_mesh_pixel(&mut self,r:Rect,x:i32,y:i32,alpha:u8) {
        if x<0||y<0||x>=r.width as i32||y>=r.height as i32{return;}
        let sx=x as usize*480/r.width as usize;let sy=y as usize*480/r.height as usize;
        // Pillow writes bottom-up 32-bit BMP; validate the alpha against the authored silhouette.
        let offset=u32::from_le_bytes([MASK[10],MASK[11],MASK[12],MASK[13]]) as usize;
        let a=MASK[offset+((479-sy)*480+sx)*4+3];
        if a<160{return;}
        self.blend_color(r.x+x,r.y+y,74,190,255,alpha);
    }

    // ------------------------=
    // FUNC: face_mesh_edge
    // DESC: Draws a softly feathered connector whose intensity follows the traveling scan band.
    // ------------------=
    fn face_mesh_edge(&mut self,r:Rect,a:(i32,i32),b:(i32,i32),scan:i32) {
        let steps=(b.0-a.0).abs().max((b.1-a.1).abs()).max(1);
        for i in 0..=steps {
            let x=a.0+(b.0-a.0)*i/steps;let y=a.1+(b.1-a.1)*i/steps;
            let band=(r.height as i32/9).max(1);
            let light=(band-(y-scan).abs()).max(0)*190/band;
            let alpha=(30+light) as u8;
            self.face_mesh_pixel(r,x,y,alpha);
            self.face_mesh_pixel(r,x-1,y,alpha/5);
            self.face_mesh_pixel(r,x+1,y,alpha/5);
        }
    }

    // ------------------------=
    // FUNC: face_mask_scan
    // DESC: Lights a dense staggered 17-row face-conforming mesh with a continuous two-way sweep.
    // ------------------=
    fn face_mask_scan(&mut self,r:Rect,elapsed:u64,reduced:bool) {
        let phase=(elapsed%3200) as i32;
        let progress=if phase<1600 {phase}else{3200-phase};
        let scan=if reduced {r.height as i32/2}else{r.height as i32*progress/1600};
        // Widths trace the forehead, cheeks, jaw and tapered chin of the generated master.
        let widths=[17,25,30,33,34,35,35,34,33,31,29,27,24,20,16,11,5];
        let point=|row:usize,col:usize| {
            let half=widths[row]*r.width as i32/100;
            (r.width as i32/2-half+2*half*col as i32/10,
                r.height as i32*(6+row as i32*56/10)/100)
        };
        for row in 0..17 {for col in 0..11 {
            let p=point(row,col);
            if col<10 {self.face_mesh_edge(r,p,point(row,col+1),scan);}
            if row<16 {
                self.face_mesh_edge(r,p,point(row+1,col),scan);
                if col<10 {self.face_mesh_edge(r,p,point(row+1,col+1),scan);}
            }
            let hot=(p.1-scan).abs()<(r.height as i32/10).max(1);
            for dy in -3i32..=3 {for dx in -3i32..=3 {
                let d=dx*dx+dy*dy;
                if d<=9 {self.face_mesh_pixel(r,p.0+dx,p.1+dy,if hot {if d<=2 {255}else{85}}else{35});}
            }}
        }}
    }
}

// ------------------------=
// FUNC: animate
// DESC: Reuses the mask substrate at at most 30 Hz; stops on cancellation, failure, completion or page exit.
// ------------------=
pub fn animate(visible:bool,reduced:bool)->bool {
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
    unsafe {
        if !visible {START_MS=None;BOUNDS=None;return false;}
        let scanning=active();
        if !scanning && START_MS.is_none(){return false;}
        let first=START_MS.is_none();
        if scanning && !first && (reduced || now.saturating_sub(LAST_MS)<33) {return false;}
        let Some(c)=(*(&raw mut CONSOLE)).as_mut() else{return false;};
        let Some(r)=BOUNDS.filter(|r|*r==Layout::new(c.display.width,c.display.height).mask) else{return false;};
        if first {START_MS=Some(now);}
        c.restore_cursor();
        let d=&mut c.display;let clip=d.render_clip;
        d.set_render_clip(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let base=&*(&raw const BASE);
        for y in 0..r.height as usize {
            core::ptr::copy_nonoverlapping(base.as_ptr().add(y*LIMIT),
                d.buffer.add((r.y as usize+y)*d.stride+r.x as usize),r.width as usize);
        }
        if scanning {d.face_mask_scan(r,now.saturating_sub(START_MS.unwrap_or(now)),reduced);}
        else {START_MS=None;}
        LAST_MS=now;
        d.mark_dirty_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        d.render_clip=clip;
        c.save_and_draw_cursor(c.cursor_x,c.cursor_y);
        c.display.present_damage();
        true
    }
}
