//! Horizontal counterpart of the window-attached AI adornment. No allocation or blur.

// ------------------------=
// FUNC: inset
// DESC: Returns the sloped shoulder in quarter-pixels, tapering into the attachment rail.
// ------------------=
fn inset(width: u32, height: u32, y: u32) -> u32 {
    let shoulder = (height * 8 / 3).min(width * 4 / 5);
    shoulder.saturating_mul(height * 4 - y) / (height * 4)
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
// DESC: Samples antialiased glass, a bounded luminous bevel and specular rim without image resampling.
// ------------------=
pub fn pixel(width: u32, height: u32, x: u32, y: u32, active: bool, hover: bool) -> (u8,u8,u8,u8) {
    if width == 0 || height == 0 || x >= width || y >= height { return (0,0,0,0); }
    let mut covered = 0;
    for dy in [1,3] {
        let cut = inset(width, height, y * 4 + dy);
        for dx in [1,3] {
            if x * 4 + dx >= cut && width * 4 - (x * 4 + dx) >= cut { covered += 1; }
        }
    }
    if covered == 0 { return (0,0,0,0); }
    let cut = inset(width,height,y * 4 + 2);
    let side = (x * 4 + 2).min((width-x)*4-2).saturating_sub(cut);
    let edge = side.min(y * 4 + 2);
    let unit = (height / 36).max(1);
    let strength = if edge < 5*unit {if active || hover {255}else{96}}
        else if edge < 12*unit {if active {82}else if hover {54}else{12}}
        else if edge < 24*unit && active {24}else{0};
    let shine = (height-y) * 8 / height;
    let base = [5+shine,10+shine,22+shine*2];
    let rim = if edge<3*unit && (active || hover) {[180,207,255]}
        else {[64+124*x/width,140-40*x/width,255]};
    let channel = |i:usize| ((base[i]*(255-strength)+rim[i]*strength)/255) as u8;
    (channel(0),channel(1),channel(2),(covered*255/4) as u8)
}

// ------------------------=
// FUNC: halo
// DESC: Samples a bounded exterior blue-violet glow around the sloped shoulders and top rim.
// ------------------=
pub fn halo(width:u32,height:u32,x:i32,y:i32,active:bool,hover:bool)->(u8,u8,u8,u8) {
    if width==0 || height==0 || (!active && !hover) {return (0,0,0,0);}
    let radius=(height/36).max(1) as i32*6;
    if y < -radius || y >= height as i32 {return (0,0,0,0);}
    let cut=inset(width,height,y.max(0) as u32*4) as i32/4;
    let edge=if y<0 {(-y).max((cut-x).max(x-(width as i32-cut)))}
        else {(cut-x).max(x-(width as i32-cut))};
    if edge<=0 || edge>radius {return (0,0,0,0);}
    let strength=if active {110}else{80};
    let a=strength*(radius-edge)*(radius-edge)/(radius*radius);
    (60+(x.clamp(0,width as i32) as u32*116/width) as u8,88,255,a as u8)
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
            assert_eq!(pixel(w,h,0,0,true,false).3,0);
            assert!(contains(w,h,w/2,h/2));
            assert_eq!(pixel(w,h,w/2,h/2,true,false).3,255);
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
