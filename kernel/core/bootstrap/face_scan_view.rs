//! Rigid canonical face geometry, decorative only, never recognition evidence.
use super::DisplayDevice;
#[cfg(target_os="none")]
use super::CONSOLE;
use crate::ui::{geometry::Rect,face_setup_layout::Layout};
const MESH:&[u8]=include_bytes!("../../../assets/ui-design-kit/default/face-canonical-v1.mesh");
const LIMIT:usize=816;
static mut BASE:[u32;LIMIT*LIMIT]=[0;LIMIT*LIMIT];
static mut DEPTH:[f32;256*256]=[0.0;256*256];
static mut DRAWN:[u64;3423]=[0;3423];
static mut BOUNDS:Option<Rect>=None;
static mut LAST_MS:u64=0;
static mut START_MS:Option<u64>=None;
static mut BASE_DIRTY:bool=false;
static mut POSE_MS:u64=0;
static mut POSE_REDUCED:bool=true;

// ------------------------=
// FUNC: advance_pose
// DESC: Bounds visible angular movement after scheduler stalls instead of skipping to a distant wall-clock pose.
// ------------------=
fn advance_pose(pose:u64,previous:u64,now:u64)->u64 {
    pose.wrapping_add(now.saturating_sub(previous).min(33))%96000
}

// ------------------------=
// FUNC: wave
// DESC: Produces periodic yaw with smooth zero-velocity reversals.
// ------------------=
fn wave(ms:u64,period:u64)->f32 {
    let t=(ms%period) as f32/period as f32;
    let x=if t<0.5 {t*2.0}else{(1.0-t)*2.0};
    let positive=16.0*x*(1.0-x)/(5.0-4.0*x*(1.0-x));
    if t<0.5 {positive}else{-positive}
}
// ------------------------=
// FUNC: triangle
// DESC: Reads immutable triangle indices from the checked-in mesh.
// ------------------=
fn triangle(i:usize)->[usize;3] {
    let at=8+468*12+i*6;
    core::array::from_fn(|c|u16::from_le_bytes([MESH[at+c*2],MESH[at+c*2+1]]) as usize)
}
// ------------------------=
// FUNC: project
// DESC: Applies one rigid orthonormal yaw to all vertices, then perspective around a fixed camera.
// ------------------=
fn project(i:usize,s:f32,c:f32)->[f32;3] {
    let v:[f32;3]=core::array::from_fn(|k| {
        let a=8+i*12+k*4;f32::from_le_bytes([MESH[a],MESH[a+1],MESH[a+2],MESH[a+3]])
    });
    let z=v[2]-3.0;
    let x=v[0]*c+z*s;
    let depth=1.0/(35.0-(z*c-v[0]*s));
    [128.0+x*420.0*depth,128.0-(v[1]+0.5)*420.0*depth,depth]
}
// ------------------------=
// FUNC: edge
// DESC: Computes signed screen-space triangle area for depth interpolation.
// ------------------=
fn edge(a:[f32;3],b:[f32;3],x:f32,y:f32)->f32 {(b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0])}

// ------------------------=
// FUNC: verify_rigid_geometry
// DESC: Reconstructs camera-space coordinates and verifies that rotations preserve every mesh edge length.
// ------------------=
#[cfg(not(target_os="none"))]
pub(crate) fn verify_rigid_geometry() {
    assert_eq!(advance_pose(700,1000,1033),733);
    assert_eq!(advance_pose(700,1000,5000),733);
    assert_eq!(advance_pose(700,1000,999),700);
    assert_eq!(advance_pose(95990,1000,1033),23);
    let camera=|p:[f32;3]|[(p[0]-128.0)/(420.0*p[2]),(128.0-p[1])/(420.0*p[2]),35.0-1.0/p[2]];
    let distance=|a:[f32;3],b:[f32;3]|(0..3).map(|k|(a[k]-b[k])*(a[k]-b[k])).sum::<f32>();
    for time in (0..12000).step_by(100) {
        let t=wave(time,12000)*0.58;let s=2.0*t/(1.0+t*t);let c=(1.0-t*t)/(1.0+t*t);
        for i in 0..(MESH.len()-8-468*12)/6 {
            let [a,b,_]=triangle(i);
            let expected=distance(camera(project(a,0.0,1.0)),camera(project(b,0.0,1.0)));
            let actual=distance(camera(project(a,s,c)),camera(project(b,s,c)));
            assert!((expected-actual).abs()<0.0002);
        }
    }
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: face_mask_base
    // DESC: Saves the clean scene beneath the illustration; animation never accumulates trails.
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
        self.face_mask_scan(r,POSE_MS,POSE_REDUCED);
    }}
    // ------------------------=
    // FUNC: face_mask_scan
    // DESC: Depth-tests rigid face triangles, then composites antialiased cyan edges and luminous nodes.
    // ------------------=
    pub(super) fn face_mask_scan(&mut self,r:Rect,elapsed:u64,reduced:bool) {unsafe {
        if BOUNDS!=Some(r) {return;}
        POSE_MS=elapsed;POSE_REDUCED=reduced;
        let base=&*core::ptr::addr_of!(BASE);
        for y in 0..r.height as usize {
            core::ptr::copy_nonoverlapping(base.as_ptr().add(y*LIMIT),
                self.buffer.add((r.y as usize+y)*self.stride+r.x as usize),r.width as usize);
        }
        // Rational half-angle rotation preserves lengths without transcendental dependencies.
        let t=if reduced {0.0}else{wave(elapsed,12000)*0.58};
        let s=2.0*t/(1.0+t*t);let c=(1.0-t*t)/(1.0+t*t);
        let points:[[f32;3];468]=core::array::from_fn(|i|project(i,s,c));
        let depth=&mut *core::ptr::addr_of_mut!(DEPTH);depth.fill(0.0);
        let triangles=(MESH.len()-8-468*12)/6;
        for i in 0..triangles {
            let [a,b,c]=triangle(i).map(|j|points[j]);
            let area=edge(a,b,c[0],c[1]);if area.abs()<0.001 {continue;}
            let minx=a[0].min(b[0]).min(c[0]).max(0.0) as usize;
            let maxx=(a[0].max(b[0]).max(c[0])+1.0).min(255.0) as usize;
            let miny=a[1].min(b[1]).min(c[1]).max(0.0) as usize;
            let maxy=(a[1].max(b[1]).max(c[1])+1.0).min(255.0) as usize;
            for y in miny..=maxy {for x in minx..=maxx {
                let u=edge(b,c,x as f32+0.5,y as f32+0.5)/area;
                let v=edge(c,a,x as f32+0.5,y as f32+0.5)/area;let w=1.0-u-v;
                if u>=0.0 && v>=0.0 && w>=0.0 {
                    depth[y*256+x]=depth[y*256+x].max(u*a[2]+v*b[2]+w*c[2]);
                }
            }}
        }
        let sweep=if reduced {0.5}else{(elapsed%3200) as f32/3200.0};
        // Each shared edge is painted once.
        let drawn=&mut *core::ptr::addr_of_mut!(DRAWN);drawn.fill(0);
        for i in 0..triangles {
            let f=triangle(i);
            for (a,b) in [(f[0],f[1]),(f[1],f[2]),(f[2],f[0])] {
                let key=a.min(b)*468+a.max(b);let bit=1u64<<(key%64);
                if drawn[key/64]&bit!=0 {continue;}drawn[key/64]|=bit;
                let a=points[a];let b=points[b];
                let steps=(((b[0]-a[0]).abs().max((b[1]-a[1]).abs())*r.width as f32/256.0) as usize+1).min(1024);
                for j in 0..=steps {
                    let t=j as f32/steps as f32;
                    let p=core::array::from_fn(|k|a[k]+(b[k]-a[k])*t);
                    self.face_mesh_sample(r,p,sweep,145,depth);
                }
            }
        }
        for p in points {self.face_mesh_sample(r,p,sweep,245,depth);}
    }}
    // ------------------------=
    // FUNC: face_mesh_sample
    // DESC: Occludes rear geometry and uses fractional pixel coverage for smooth subpixel motion.
    // ------------------=
    fn face_mesh_sample(&mut self,r:Rect,p:[f32;3],sweep:f32,alpha:u8,depth:&[f32]) {
        if p[0]<1.0 || p[1]<1.0 || p[0]>=255.0 || p[1]>=255.0 {return;}
        if p[2]+0.00035<depth[p[1] as usize*256+p[0] as usize] {return;}
        let x=r.x as f32+p[0]*r.width as f32/256.0;
        let y=r.y as f32+p[1]*r.height as f32/256.0;
        let ix=x as i32;let iy=y as i32;let fx=x-ix as f32;let fy=y-iy as f32;
        let glow=(1.0-(p[1]/256.0-sweep).abs()*24.0).max(0.0);
        let opacity=(alpha as f32+glow*100.0).min(255.0);
        if alpha==245 {
            for dy in -2..=2 {for dx in -2..=2 {
                let distance=(dx as f32-fx)*(dx as f32-fx)+(dy as f32-fy)*(dy as f32-fy);
                let coverage=(1.0-distance/4.0).max(0.0);
                self.blend_color(ix+dx,iy+dy,175,235,255,(opacity*coverage*coverage) as u8);
            }}
            return;
        }
        for (dx,dy,w) in [(0,0,(1.0-fx)*(1.0-fy)),(1,0,fx*(1.0-fy)),(0,1,(1.0-fx)*fy),(1,1,fx*fy)] {
            self.blend_color(ix+dx,iy+dy,145,225,255,(opacity*w) as u8);
        }
    }
}

// ------------------------=
// FUNC: animate
// DESC: Presents bounded mesh damage at 30 Hz with static reduced motion and cursor-coherent restoration.
// ------------------=
#[cfg(target_os="none")]
pub fn animate(visible:bool,reduced:bool)->bool {
    let now=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
    unsafe {
        if !visible {START_MS=None;BOUNDS=None;POSE_MS=0;POSE_REDUCED=true;return false;}
        let first=START_MS.is_none();
        if !first && !BASE_DIRTY && (reduced || now.saturating_sub(LAST_MS)<33) {return false;}
        let Some(c)=(*core::ptr::addr_of_mut!(CONSOLE)).as_mut() else{return false;};
        let Some(r)=BOUNDS.filter(|r|*r==Layout::new(c.display.width,c.display.height).mask) else{return false;};
        if first {START_MS=Some(now);}
        c.restore_cursor();
        let d=&mut c.display;let clip=d.render_clip;
        d.set_render_clip(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        let pose=if first || reduced {POSE_MS}else{advance_pose(POSE_MS,LAST_MS,now)};
        d.face_mask_scan(r,pose,reduced);
        LAST_MS=now;BASE_DIRTY=false;
        d.mark_dirty_rect(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        d.render_clip=clip;
        c.save_and_draw_cursor(c.cursor_x,c.cursor_y);c.display.present_damage();true
    }
}
