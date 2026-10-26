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
    // DESC: Bilinearly samples generated angle sprites with moving blue-white scan illumination over a clean substrate.
    // ------------------=
    pub(super) fn face_mask_scan(&mut self,r:Rect,elapsed:u64,reduced:bool) {unsafe {
        if BOUNDS!=Some(r) {return;}
        const ANGLES:[usize;16]=[0,1,2,4,5,4,2,1,0,15,14,13,12,13,14,15];
        let phase=if reduced {0}else{elapsed%6400};
        let frame=ANGLES[(phase/400) as usize];
        let next=ANGLES[((phase/400+1)%16) as usize];
        let mix=if reduced {0}else{(phase%400)*256/400} as u32;
        let sweep=if reduced {r.height/2}else{((elapsed%2400)*u64::from(r.height)/2400) as u32};
        let base=&*core::ptr::addr_of!(BASE);
        for y in 0..r.height as usize {
            core::ptr::copy_nonoverlapping(base.as_ptr().add(y*LIMIT),
                self.buffer.add((r.y as usize+y)*self.stride+r.x as usize),r.width as usize);
            let fy=y*255*256/(r.height as usize).saturating_sub(1).max(1);
            let sy=fy/256;let wy=(fy%256) as u32;
            for x in 0..r.width as usize {
                let fx=x*255*256/(r.width as usize).saturating_sub(1).max(1);
                let sx=fx/256;let wx=(fx%256) as u32;
                let mut rgba=[0u32;4];
                for (cell,weight) in [(frame,256-mix),(next,mix)] {
                    if weight==0 {continue;}
                    for (px,py,w) in [(sx,sy,(256-wx)*(256-wy)),((sx+1).min(255),sy,wx*(256-wy)),
                        (sx,(sy+1).min(255),(256-wx)*wy),((sx+1).min(255),(sy+1).min(255),wx*wy)] {
                        let offset=(cell*256*256+py*256+px)*4;
                        for c in 0..4 {rgba[c]+=u32::from(SPRITES[offset+c])*w*weight/256;}
                    }
                }
                let alpha=(rgba[3]/65536) as u8;
                if alpha==0 {continue;}
                let distance=(y as i32-sweep as i32).unsigned_abs();
                let band=(r.height/28).max(1);
                let glow=band.saturating_sub(distance)*140/band;
                self.blend_color(r.x+x as i32,r.y+y as i32,
                    (rgba[0]/65536+glow).min(255) as u8,
                    (rgba[1]/65536+glow).min(255) as u8,
                    (rgba[2]/65536+glow).min(255) as u8,alpha);
            }
        }
    }}
}

// ------------------------=
// FUNC: animate
// DESC: Presents bounded sprite damage at 12 Hz with static reduced motion and cursor-coherent restoration.
// ------------------=
#[cfg(target_os="none")]
pub fn animate(visible:bool,reduced:bool)->bool {
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
    unsafe {
        if !visible {START_MS=None;BOUNDS=None;return false;}
        let first=START_MS.is_none();
        if !first && !BASE_DIRTY && (reduced || now.saturating_sub(LAST_MS)<83) {return false;}
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
