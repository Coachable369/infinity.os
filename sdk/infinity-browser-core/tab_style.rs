//! Image-backed horizontal sibling of the default kit's attached AI adornment.
const WIDTH: usize = 944;
const HEIGHT: usize = 192;
const IMAGE: &[u8; WIDTH * HEIGHT * 4] = include_bytes!("../../assets/ui-design-kit/default/browser-horizontal-tab-v1.rgba");

// ------------------------=
// FUNC: inset
// DESC: Returns smooth, symmetric shoulders in quarter-pixels for pointer ownership.
// ------------------=
fn inset(width: u32, height: u32, y: u32) -> u32 {
    let shoulder = (height * 8 / 3).min(width * 4 / 5);
    let remaining = (height * 4).saturating_sub(y);
    shoulder.saturating_mul(remaining).saturating_mul(remaining) / (height * 4).pow(2)
}

// ------------------------=
// FUNC: contains
// DESC: Shares the visible bevel silhouette with native tab hit testing.
// ------------------=
pub fn contains(width: u32, height: u32, x: u32, y: u32) -> bool {
    if width == 0 || height == 0 || x >= width || y >= height { return false; }
    let cut = inset(width, height, y * 4 + 2);
    x * 4 + 2 >= cut && (width - x) * 4 - 2 >= cut
}

// ------------------------=
// FUNC: pixel
// DESC: Samples the generated glass sprite inside its existing logical tab bounds.
// ------------------=
pub fn pixel(width: u32, height: u32, x: u32, y: u32, active: bool, hover: bool) -> (u8,u8,u8,u8) {
    if width == 0 || height == 0 || x >= width || y >= height { return (0,0,0,0); }
    sample(width,height,x as i32,y as i32,active,hover)
}

// ------------------------=
// FUNC: halo
// DESC: Uses only the master's bounded exterior glow, without adding a second analytic rim.
// ------------------=
pub fn halo(width:u32,height:u32,x:i32,y:i32,active:bool,hover:bool)->(u8,u8,u8,u8) {
    if width==0 || height==0 || (!active && !hover) {return (0,0,0,0);}
    if x>=0 && x<width as i32 && y>=0 {return (0,0,0,0);}
    sample(width,height,x,y,active,hover)
}

// ------------------------=
// FUNC: sample
// DESC: Alpha-weighted area sampling preserves the image's bevel and transparency at native scale without decoding or allocation.
// ------------------=
fn sample(width:u32,height:u32,x:i32,y:i32,active:bool,hover:bool)->(u8,u8,u8,u8) {
    if width==0 || height==0 {return (0,0,0,0);}
    let scale=(height/36).max(1) as i32;
    if x< -6*scale || x>=width as i32+6*scale || y< -6*scale || y>=height as i32 {return (0,0,0,0);}
    let w=width as usize+12*scale as usize;
    let h=height as usize+12*scale as usize;
    let px=(x+6*scale) as usize;
    let py=(y+8*scale) as usize;
    let x0=(px*WIDTH/w).min(WIDTH-1);
    let x1=((px+1)*WIDTH/w).max(x0+1).min(WIDTH);
    let y0=(py*HEIGHT/h).min(HEIGHT-1);
    let y1=((py+1)*HEIGHT/h).max(y0+1).min(HEIGHT);
    let mut channels=[0u32;3];let mut alpha=0u32;
    for sy in y0..y1 {for sx in x0..x1 {
        let at=(sy*WIDTH+sx)*4;let a=IMAGE[at+3] as u32;
        alpha+=a;
        for c in 0..3 {channels[c]+=IMAGE[at+c] as u32*a;}
    }}
    if alpha==0 {return (0,0,0,0);}
    let gain=if active {if hover {272}else{256}}else if hover {208}else{100};
    let color=|c:usize| ((channels[c]/alpha)*gain/256).min(255) as u8;
    (color(0),color(1),color(2),(alpha/((x1-x0)*(y1-y0)) as u32) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: bevel_pixels_and_hits_scale_together
    // DESC: Checks transparent shoulders, opaque readable centers, bounded samples and distinct active edges.
    // ------------------=
    #[test]
    fn bevel_pixels_and_hits_scale_together() {
        for scale in 1..=4 {
            let (w,h)=(216*scale,36*scale);
            assert!(!contains(w,h,0,0));
            assert!(pixel(w,h,0,0,true,false).3<64);
            assert!(contains(w,h,w/2,h/2));
            assert!(pixel(w,h,w/2,h/2,true,false).3>230);
            assert_ne!(pixel(w,h,w/2,0,true,false),pixel(w,h,w/2,0,false,false));
            assert_ne!(pixel(w,h,w/2,0,false,true),pixel(w,h,w/2,0,false,false));
            assert_eq!(pixel(w,h,w,h,true,false).3,0);
            for y in 0..h { for x in 0..w {
                assert_eq!(contains(w,h,x,y),contains(w,h,w-1-x,y));
                if contains(w,h,x,y) {assert!(pixel(w,h,x,y,true,false).3>0);}
            }}
        }
    }
    // ------------------------=
    // FUNC: exterior_glow_is_bounded_and_stateful
    // DESC: Verifies glow outside the silhouette without leaking into content or illuminating inactive tabs.
    // ------------------=
    #[test]
    fn exterior_glow_is_bounded_and_stateful() {
        for scale in 1..=4 {
            let (w,h)=(224*scale,36*scale);
            assert!(halo(w,h,(w/2) as i32,-(scale as i32),true,false).3>0);
            assert_eq!(halo(w,h,(w/2) as i32,-(7*scale as i32),true,false).3,0);
            assert_eq!(halo(w,h,(w/2) as i32,h as i32,true,false).3,0);
            assert_eq!(halo(w,h,(w/2) as i32,-1,false,false).3,0);
        }
    }
}
