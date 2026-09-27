//! Horizontal counterpart of the window-attached AI adornment. No allocation or blur.

// ------------------------=
// FUNC: inset
// DESC: Returns the sloped shoulder in quarter-pixels, tapering into the attachment rail.
// ------------------=
fn inset(height: u32, y: u32) -> u32 {
    let shoulder = height * 4 / 3;
    shoulder.saturating_mul(height * 4 - y) / (height * 4)
}

// ------------------------=
// FUNC: contains
// DESC: Shares the visible bevel silhouette with native tab hit testing.
// ------------------=
pub fn contains(width: u32, height: u32, x: u32, y: u32) -> bool {
    if width == 0 || height == 0 || x >= width || y >= height { return false; }
    let cut = inset(height, y * 4 + 2);
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
        let cut = inset(height, y * 4 + dy);
        for dx in [1,3] {
            if x * 4 + dx >= cut && width * 4 - (x * 4 + dx) >= cut { covered += 1; }
        }
    }
    if covered == 0 { return (0,0,0,0); }
    let cut = inset(height,y * 4 + 2);
    let side = (x * 4 + 2).min((width-x)*4-2).saturating_sub(cut);
    let edge = side.min(y * 4 + 2);
    let unit = (height / 36).max(1);
    let strength = if edge < 5*unit {if active || hover {230}else{96}}
        else if edge < 12*unit {if active {82}else if hover {54}else{12}}
        else if edge < 24*unit && active {24}else{0};
    let shine = (height-y) * 12 / height;
    let base = [7+shine,14+shine,26+shine*2];
    let rim = [72+112*x/width,188-70*x/width,255];
    let channel = |i:usize| ((base[i]*(255-strength)+rim[i]*strength)/255) as u8;
    (channel(0),channel(1),channel(2),(covered*255/4) as u8)
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
}
