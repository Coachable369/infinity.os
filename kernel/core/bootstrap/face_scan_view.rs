//! Generated connected-dot face, decorative only, never recognition evidence.
use super::DisplayDevice;
#[cfg(target_os="none")]
use super::CONSOLE;
use crate::ui::{geometry::Rect,face_setup_layout::Layout};
const SPRITES:&[u8]=include_bytes!("../../../assets/ui-design-kit/default/face-digital-turn-v1.rgba");
const LIMIT:usize=816;
static mut BASE:[u32;LIMIT*LIMIT]=[0;LIMIT*LIMIT];
static mut BOUNDS:Option<Rect>=None;
static mut LAST_MS:u64=0;
static mut START_MS:Option<u64>=None;
static mut BASE_DIRTY:bool=false;

// ------------------------=
// FUNC: wave
// DESC: Produces a continuous periodic sine approximation with zero-velocity yaw reversals.
// ------------------=
fn wave(ms:u64,period:u64)->f32 {
    let t=ms as f32/period as f32;
    let x=if t<0.5 {t*2.0}else{(1.0-t)*2.0};
    let positive=16.0*x*(1.0-x)/(5.0-4.0*x*(1.0-x));
    if t<0.5 {positive}else{-positive}
}
// ------------------------=
// FUNC: mesh_point
// DESC: Projects the generated face texture on a fixed convex depth surface around a stable pivot.
// ------------------=
fn mesh_point(sx:f32,y:f32,r:Rect,sine:f32,cosine:f32)->f32 {
    let x=(sx-128.0)/128.0;
    let dome=(1.0-x*x-y*y).max(0.0)*0.20;
    let nose=(1.0-x.abs()*4.0).max(0.0)*(1.0-(y-0.05).abs()*3.0).max(0.0)*0.10;
    r.x as f32+r.width as f32*(0.5+(x*cosine+(dome+nose)*sine)*0.43)
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: face_mask_base
    // DESC: Caches clean scene pixels before painting the first dot frame, preventing animation trails.
    // ------------------=
    pub(super) fn face_mask_base(&mut self,r:Rect) {unsafe {
        BOUNDS=None;
        if r.width as usize>LIMIT || r.height as usize>LIMIT || r.x<0 || r.y<0
            || r.right()>self.width as i32 || r.bottom()>self.height as i32 {return;}
        let base=&mut *core::ptr::addr_of_mut!(BASE);
        for y in 0..r.height as usize {for x in 0..r.width as usize {
            base[y*LIMIT+x]=*self.buffer.add((r.y as usize+y)*self.stride+r.x as usize+x);
        }}
        BOUNDS=Some(r);BASE_DIRTY=true;
        self.face_mask_scan(r,0,true);
    }}
    // ------------------------=
    // FUNC: face_mask_scan
    // DESC: Rotates a textured connected-dot depth surface with bilinear filtering and scan illumination.
    // ------------------=
    pub(super) fn face_mask_scan(&mut self,r:Rect,elapsed:u64,reduced:bool) {unsafe {
        if BOUNDS!=Some(r) {return;}
        let base=&*core::ptr::addr_of!(BASE);
        for y in 0..r.height as usize {
            core::ptr::copy_nonoverlapping(base.as_ptr().add(y*LIMIT),
                self.buffer.add((r.y as usize+y)*self.stride+r.x as usize),r.width as usize);
        }
        let yaw=if reduced {0.0}else{wave(elapsed%10000,10000)*0.60};
        let sine=yaw*(1.0-yaw*yaw/6.0+yaw*yaw*yaw*yaw/120.0);
        let cosine=1.0-yaw*yaw/2.0+yaw*yaw*yaw*yaw/24.0;
        let sweep=if reduced {0.5}else{(elapsed%3200) as f32/3200.0};
        for dy in 0..r.height {
            let ny=((dy as f32+0.5)/r.height as f32-0.5)/0.43;
            let sy=128.0+ny*128.0;
            if sy<0.0 || sy>=255.0 {continue;}
            let iy=sy as usize;let fy=sy-iy as f32;
            let glow=(1.0-(dy as f32/r.height as f32-sweep).abs()*28.0).max(0.0);
            let mut left=mesh_point(0.0,ny,r,sine,cosine);
            for sx in 0..255 {
                let right=mesh_point((sx+1) as f32,ny,r,sine,cosine);
                for dx in (left+0.5) as i32..(right+0.5) as i32 {
                    let fx=((dx as f32+0.5-left)/(right-left)).clamp(0.0,1.0);
                    let mut rgba=[0u8;4];
                    for c in 0..4 {
                        let at=(iy*256+sx)*4+c;
                        let top=SPRITES[at] as f32*(1.0-fx)+SPRITES[at+4] as f32*fx;
                        let bottom=SPRITES[at+1024] as f32*(1.0-fx)+SPRITES[at+1028] as f32*fx;
                        rgba[c]=(top*(1.0-fy)+bottom*fy) as u8;
                    }
                    self.blend_color(dx,r.y+dy as i32,rgba[0].saturating_add((glow*35.0) as u8),rgba[1],rgba[2],rgba[3]);
                }
                left=right;
            }
        }
    }}
}

// ------------------------=
// FUNC: animate
// DESC: Presents bounded mesh damage at 30 Hz with static reduced motion and cursor-coherent restoration.
// ------------------=
#[cfg(target_os="none")]
pub fn animate(visible:bool,reduced:bool)->bool {
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
    unsafe {
        if !visible {START_MS=None;BOUNDS=None;return false;}
        let first=START_MS.is_none();
        if !first && !BASE_DIRTY && (reduced || now.saturating_sub(LAST_MS)<33) {return false;}
        let Some(c)=(*core::ptr::addr_of_mut!(CONSOLE)).as_mut() else{return false;};
        let Some(r)=BOUNDS.filter(|r|*r==Layout::new(c.display.width,c.display.height).mask) else{return false;};
        if first {START_MS=Some(now);}
        c.restore_cursor();
        let d=&mut c.display;let clip=d.render_clip;
        d.set_render_clip(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        d.face_mask_scan(r,now.saturating_sub(START_MS.unwrap_or(now)),reduced);
        LAST_MS=now;BASE_DIRTY=false;
        d.mark_dirty_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        d.render_clip=clip;
        c.save_and_draw_cursor(c.cursor_x,c.cursor_y);c.display.present_damage();true
    }
}
