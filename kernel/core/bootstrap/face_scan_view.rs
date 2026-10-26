//! Illustrative scan mesh, not measured landmarks or proof of recognition.
use super::DisplayDevice;
#[cfg(target_os="none")]
use super::CONSOLE;
use crate::ui::{geometry::Rect,face_setup_layout::Layout};
const MASK:&[u8]=include_bytes!("../../../assets/ui-design-kit/default/face-mask-v1.bmp");
const LIMIT:usize=480;
static mut BASE:[u32;LIMIT*LIMIT]=[0;LIMIT*LIMIT];
static mut BOUNDS:Option<Rect>=None;
static mut LAST_MS:u64=0;
static mut START_MS:Option<u64>=None;
static mut INTENSITY:u16=255;
static mut BASE_DIRTY:bool=false;

// ------------------------=
// FUNC: active
// DESC: Distinguishes the active capture sweep from the slower decorative idle mesh.
// ------------------=
#[cfg(target_os="none")]
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
        #[cfg(target_os="none")]
        unsafe {
            BOUNDS=None;
            if r.width as usize>LIMIT || r.height as usize>LIMIT || r.x<0 || r.y<0
                || r.right()>self.width as i32 || r.bottom()>self.height as i32 {return;}
            let base=&mut *(&raw mut BASE);
            for y in 0..r.height as usize {for x in 0..r.width as usize {
                base[y*LIMIT+x]=self.framebuffer_pixel(r.x as usize+x,r.y as usize+y);
            }}
            BOUNDS=Some(r);
            BASE_DIRTY=true;
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
        let alpha=unsafe {(u16::from(alpha)*INTENSITY/255) as u8};
        let (red,green)=if alpha>210 {(173,228)}else{(74,190)};
        self.blend_color(r.x+x,r.y+y,red,green,255,alpha);
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
            let counter=(band-(y-(r.height as i32-scan)).abs()).max(0)*110/band;
            let alpha=(30+light.max(counter)).min(255) as u8;
            self.face_mesh_pixel(r,x,y,alpha);
            self.face_mesh_pixel(r,x-1,y,alpha/5);
            self.face_mesh_pixel(r,x+1,y,alpha/5);
            self.face_mesh_pixel(r,x,y-1,alpha/4);
            self.face_mesh_pixel(r,x,y+1,alpha/4);
        }
    }

    // ------------------------=
    // FUNC: face_mask_scan
    // DESC: Lights a dense staggered 17-row face-conforming mesh with a continuous two-way sweep.
    // ------------------=
    pub(super) fn face_mask_scan(&mut self,r:Rect,elapsed:u64,reduced:bool) {
        let phase=(elapsed%3200) as i32;
        let progress=if phase<1600 {phase}else{3200-phase};
        let scan=if reduced {r.height as i32/2}else{r.height as i32*progress/1600};
        let halo=(r.height as i32/24).max(1);
        for center in [scan,r.height as i32-scan] {
            for y in (center-halo).max(0)..(center+halo).min(r.height as i32) {
                let light=((halo-(y-center).abs())*60/halo) as u8;
                for x in 0..r.width as i32 {self.face_mesh_pixel(r,x,y,light);}
            }
        }
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
            let hot=(p.1-scan).abs().min((p.1-(r.height as i32-scan)).abs())<(r.height as i32/10).max(1);
            for dy in -5i32..=5 {for dx in -5i32..=5 {
                let d=dx*dx+dy*dy;
                if d<=25 {self.face_mesh_pixel(r,p.0+dx,p.1+dy,
                    if hot {if d<=2 {255}else{(200/(d/2+1)) as u8}}else{(70/(d+1)) as u8});}
            }}
        }}
    }
}

// ------------------------=
// FUNC: animate
// DESC: Reuses the cached mask at 15 Hz idle or 30 Hz capture; page exit stops animation and reduced motion is static.
// ------------------=
#[cfg(target_os="none")]
pub fn animate(visible:bool,reduced:bool)->bool {
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
    unsafe {
        if !visible {START_MS=None;BOUNDS=None;return false;}
        let scanning=active();
        let first=START_MS.is_none();
        if !first && !BASE_DIRTY && (reduced || now.saturating_sub(LAST_MS)<if scanning {33}else{66}) {return false;}
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
        let elapsed=now.saturating_sub(START_MS.unwrap_or(now));
        INTENSITY=if scanning {255}else{100};
        d.face_mask_scan(r,if scanning {elapsed}else{elapsed/4},reduced);
        LAST_MS=now;
        BASE_DIRTY=false;
        d.mark_dirty_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        d.render_clip=clip;
        c.save_and_draw_cursor(c.cursor_x,c.cursor_y);
        c.display.present_damage();
        true
    }
}
