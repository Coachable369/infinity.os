//! YuNet decoding and SFace alignment for a real decoded 640x480 RGB camera frame.
//! No camera simulation, host image API, or synthetic recognition path.
use crate::{Pose, CROP_SIDE};

pub const CAMERA_WIDTH:usize=640;
pub const CAMERA_HEIGHT:usize=480;
pub const RGB_BYTES:usize=CAMERA_WIDTH*CAMERA_HEIGHT*3;
const MAX_CANDIDATES:usize=64;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Shape, Numeric, Crowded, NoFace, MultipleFaces, Landmarks, Pose }
#[derive(Clone,Copy,Default,Debug)]
pub struct Point {pub x:f32,pub y:f32}
#[derive(Clone,Copy,Default,Debug)]
pub struct Face {pub x:f32,pub y:f32,pub width:f32,pub height:f32,pub score:f32,pub landmarks:[Point;5]}
pub struct Head<'a> {pub stride:usize,pub class:&'a [f32],pub object:&'a [f32],pub boxes:&'a [f32],pub landmarks:&'a [f32]}

// ------------------------=
// FUNC: detector_input
// DESC: Converts actual RGB pixels to letterboxed 320x320 NCHW BGR, matching the fixed YuNet preprocessing contract.
// ------------------=
pub fn detector_input(rgb:&[u8],out:&mut [f32])->Result<(),Error> {
    if rgb.len()!=RGB_BYTES || out.len()!=3*320*320 {return Err(Error::Shape);}
    out.fill(0.0);
    for y in 0..240 {for x in 0..320 {for c in 0..3 {
        let mut sum=0u32;
        for dy in 0..2 {for dx in 0..2 {sum+=u32::from(rgb[((y*2+dy)*640+x*2+dx)*3+2-c]);}}
        out[c*320*320+y*320+x]=sum as f32*0.25;
    }}}
    Ok(())
}

// ------------------------=
// FUNC: sqrt
// DESC: Computes a finite nonnegative square root without platform math dependencies.
// ------------------=
fn sqrt(value:f32)->Result<f32,Error> {
    if !value.is_finite() || value<0.0 {return Err(Error::Numeric);}
    if value==0.0 {return Ok(0.0);}
    let mut x=f32::from_bits((value.to_bits()>>1)+0x1fc00000);
    for _ in 0..6 {x=0.5*(x+value/x);}
    Ok(x)
}

// ------------------------=
// FUNC: overlap
// DESC: Measures intersection-over-union for bounded nonmaximum suppression.
// ------------------=
fn overlap(a:Face,b:Face)->f32 {
    let w=(a.x+a.width).min(b.x+b.width)-a.x.max(b.x);
    let h=(a.y+a.height).min(b.y+b.height)-a.y.max(b.y);
    let intersection=w.max(0.0)*h.max(0.0);
    let union=a.width*a.height+b.width*b.height-intersection;
    if union>0.0 {intersection/union}else{0.0}
}

// ------------------------=
// FUNC: single_face
// DESC: Decodes the three real YuNet heads, performs bounded NMS, and fails closed on zero, multiple or excessive candidates.
// ------------------=
pub fn single_face(heads:[Head<'_>;3])->Result<Face,Error> {
    let mut candidates=[Face::default();MAX_CANDIDATES];let mut length=0;
    for (level,head) in heads.into_iter().enumerate() {
        let stride=8usize<<level;
        if head.stride!=stride {return Err(Error::Shape);}
        let side=320/stride;let count=side*side;
        if head.class.len()!=count || head.object.len()!=count || head.boxes.len()!=count*4 || head.landmarks.len()!=count*10 {return Err(Error::Shape);}
        for i in 0..count {
            let class=head.class[i];let object=head.object[i];
            if !class.is_finite() || !object.is_finite() || !(0.0..=1.0).contains(&class) || !(0.0..=1.0).contains(&object) {return Err(Error::Numeric);}
            let score=sqrt(class*object)?;
            if score<0.8 {continue;}
            let b=&head.boxes[i*4..i*4+4];
            if b.iter().any(|v|!v.is_finite()) || b[2].abs()>10.0 || b[3].abs()>10.0 {return Err(Error::Numeric);}
            let scale=stride as f32*2.0;
            let width=crate::inference::exp(b[2]).map_err(|_|Error::Numeric)?*scale;
            let height=crate::inference::exp(b[3]).map_err(|_|Error::Numeric)?*scale;
            let x=((i%side) as f32+b[0])*scale-width*0.5;
            let y=((i/side) as f32+b[1])*scale-height*0.5;
            if x+width<=0.0 || y+height<=0.0 || x>=640.0 || y>=480.0 {continue;}
            let mut face=Face {x,y,width,height,score,landmarks:[Point::default();5]};
            for j in 0..5 {
                let dx=head.landmarks[i*10+j*2];let dy=head.landmarks[i*10+j*2+1];
                if !dx.is_finite() || !dy.is_finite() {return Err(Error::Numeric);}
                face.landmarks[j]=Point {x:((i%side) as f32+dx)*scale,y:((i/side) as f32+dy)*scale};
            }
            if length==MAX_CANDIDATES {return Err(Error::Crowded);}
            let mut at=length;
            while at>0 && candidates[at-1].score<score {candidates[at]=candidates[at-1];at-=1;}
            candidates[at]=face;length+=1;
        }
    }
    let mut suppressed=[false;MAX_CANDIDATES];let mut selected=None;
    for i in 0..length {
        if suppressed[i] {continue;}
        if selected.is_some() {return Err(Error::MultipleFaces);}
        selected=Some(candidates[i]);
        for j in i+1..length {if overlap(candidates[i],candidates[j])>0.3 {suppressed[j]=true;}}
    }
    selected.ok_or(Error::NoFace)
}

// ------------------------=
// FUNC: pose
// DESC: Derives gentle horizontal pose guidance from detected landmarks; not liveness or anti-spoofing evidence.
// ------------------=
pub fn pose(face:&Face)->Result<Pose,Error> {
    let [left,right,nose,mouth_left,mouth_right]=face.landmarks;
    for p in face.landmarks {if !p.x.is_finite() || !p.y.is_finite() || p.x<0.0 || p.y<0.0 || p.x>=640.0 || p.y>=480.0 {return Err(Error::Landmarks);}}
    let dx=right.x-left.x;let dy=right.y-left.y;
    let distance=dx*dx+dy*dy;
    if dx<15.0 || distance<400.0 || dy.abs()>dx*0.35 || nose.y<left.y.min(right.y)
        || mouth_left.y<nose.y || mouth_right.y<nose.y {return Err(Error::Landmarks);}
    let nx=nose.x-(left.x+right.x)*0.5;let ny=nose.y-(left.y+right.y)*0.5;
    let yaw=(nx*dx+ny*dy)/distance;
    if yaw.abs()>0.5 {return Err(Error::Pose);}
    // Camera RGB is not mirrored. A person's left turn moves their nose
    // towards image-right; pose names follow the person, not the image axes.
    Ok(if yaw>0.12 {Pose::Left}else if yaw< -0.12 {Pose::Right}else{Pose::Front})
}

// ------------------------=
// FUNC: pose_uses_subject_coordinates
// DESC: Checks unmirrored camera landmarks against the wizard's person-relative front and side instructions.
// ------------------=
#[test]
fn pose_uses_subject_coordinates() {
    for (nose_x,expected) in [(320.0,Pose::Front),(340.0,Pose::Left),(300.0,Pose::Right)] {
        let face=Face {landmarks:[Point{x:270.0,y:180.0},Point{x:370.0,y:180.0},
            Point{x:nose_x,y:230.0},Point{x:285.0,y:280.0},Point{x:355.0,y:280.0}],..Face::default()};
        assert_eq!(pose(&face),Ok(expected));
    }
}

// ------------------------=
// FUNC: align
// DESC: Fits a five-landmark similarity transform and bilinearly samples actual RGB pixels into SFace input and the quality crop.
// ------------------=
pub fn align(rgb:&[u8],face:&Face,input:&mut [f32],grey:&mut [u8])->Result<(),Error> {
    if rgb.len()!=RGB_BYTES || input.len()!=3*CROP_SIDE*CROP_SIDE || grey.len()!=CROP_SIDE*CROP_SIDE {return Err(Error::Shape);}
    pose(face)?;
    let target=[Point{x:38.2946,y:51.6963},Point{x:73.5318,y:51.5014},Point{x:56.0252,y:71.7366},
        Point{x:41.5493,y:92.3655},Point{x:70.7299,y:92.2041}];
    let mut source_mean=Point::default();let mut target_mean=Point::default();
    for i in 0..5 {source_mean.x+=face.landmarks[i].x*0.2;source_mean.y+=face.landmarks[i].y*0.2;
        target_mean.x+=target[i].x*0.2;target_mean.y+=target[i].y*0.2;}
    let mut den=0.0;let mut dot=0.0;let mut cross=0.0;
    for i in 0..5 {
        let x=face.landmarks[i].x-source_mean.x;let y=face.landmarks[i].y-source_mean.y;
        let u=target[i].x-target_mean.x;let v=target[i].y-target_mean.y;
        den+=x*x+y*y;dot+=x*u+y*v;cross+=x*v-y*u;
    }
    if den<1.0 {return Err(Error::Landmarks);}
    let a=dot/den;let b=cross/den;let determinant=a*a+b*b;
    if !determinant.is_finite() || determinant<0.0001 || determinant>100.0 {return Err(Error::Landmarks);}
    let tx=target_mean.x-a*source_mean.x+b*source_mean.y;
    let ty=target_mean.y-b*source_mean.x-a*source_mean.y;
    for y in 0..112 {for x in 0..112 {
        let u=x as f32-tx;let v=y as f32-ty;
        let sx=(a*u+b*v)/determinant;let sy=(-b*u+a*v)/determinant;
        if !sx.is_finite() || !sy.is_finite() || sx<0.0 || sy<0.0 || sx>=639.0 || sy>=479.0 {return Err(Error::Landmarks);}
        let ix=sx as usize;let iy=sy as usize;let fx=sx-ix as f32;let fy=sy-iy as f32;
        let mut channels=[0.0;3];
        for c in 0..3 {
            let at=(iy*640+ix)*3+c;
            let top=rgb[at] as f32*(1.0-fx)+rgb[at+3] as f32*fx;
            let bottom=rgb[at+1920] as f32*(1.0-fx)+rgb[at+1923] as f32*fx;
            channels[c]=top*(1.0-fy)+bottom*fy;
            input[c*112*112+y*112+x]=channels[c];
        }
        grey[y*112+x]=(channels[0]*0.299+channels[1]*0.587+channels[2]*0.114+0.5) as u8;
    }}
    Ok(())
}
