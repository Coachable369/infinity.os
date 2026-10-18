//! Tron Lightrail tabs: bounded native pixels, no decode, allocation or idle work.

// ------------------------=
// FUNC: coverage
// DESC: Supersamples clipped upper corners while keeping a flat toolbar attachment.
// ------------------=
fn coverage(w:u32,h:u32,x:u32,y:u32)->u8 {
    if w==0 || h==0 || x>=w || y>=h {return 0;}
    let radius=(h*8/36).max(1).min(w/2);
    let mut count=0;
    for dy in 0..4 {for dx in 0..4 {
        let px=x*4+dx;let py=y*4+dy;let r=radius*4;
        if px+py>=r && (w*4-1-px)+py>=r {count+=1;}
    }}
    (count*255/16) as u8
}
// ------------------------=
// FUNC: contains
// DESC: Uses the visible clipped silhouette for tab pointer ownership.
// ------------------=
pub fn contains(w:u32,h:u32,x:u32,y:u32)->bool {coverage(w,h,x,y)>=128}
// ------------------------=
// FUNC: pixel
// DESC: Paints smoked gradients and an inset cyan lightrail without expanding damage bounds.
// ------------------=
pub fn pixel(w:u32,h:u32,x:u32,y:u32,active:bool,hover:bool)->(u8,u8,u8,u8) {
    let alpha=coverage(w,h,x,y);if alpha==0 {return (0,0,0,0);}
    let scale=(h/36).max(1);
    let bevel=(h*8/36).max(1).min(w/2);
    let side=x.min(w-1-x);
    let edge=y<scale || side<scale || side+y<bevel+scale || alpha<255;
    let rail=active && y<2*scale && side>=bevel;
    let rgb=if rail {
        if hover {[155,249,255]}else{[69,226,250]}
    }else if edge {
        if active {[35,160,194]}else if hover {[47,133,162]}else{[32,65,86]}
    }else {
        let top=if active {if hover {[22u32,52,72]}else{[16,42,61]}}
            else if hover {[15,35,51]}else{[10,24,38]};
        let lift=if active && y<8*scale {(8*scale-y)*3/scale}else{0};
        let t=y*255/h.max(1);
        [((top[0]*(255-t)+8*t)/255) as u8,
         (((top[1]*(255-t)+20*t)/255)+lift) as u8,
         (((top[2]*(255-t)+32*t)/255)+lift) as u8]
    };
    (rgb[0],rgb[1],rgb[2],alpha)
}
// ------------------------=
// FUNC: halo
// DESC: Retains the caller contract with no exterior halo or paint outside tab bounds.
// ------------------=
pub fn halo(_:u32,_:u32,_:i32,_:i32,_:bool,_:bool)->(u8,u8,u8,u8) {(0,0,0,0)}

#[cfg(test)] mod tests {
    use super::*;
    // ------------------------=
    // FUNC: attached_tabs_are_symmetric_bounded_and_stateful
    // DESC: Verifies actual coverage, state colors and flat lower attachment at supported scales.
    // ------------------=
    #[test] fn attached_tabs_are_symmetric_bounded_and_stateful() {
        for scale in 1..=4 {let(w,h)=(224*scale,36*scale);
            assert_eq!(pixel(w,h,0,0,true,false).3,0);
            assert_eq!(pixel(w,h,w/2,h/2,true,false).3,255);
            assert!(contains(w,h,0,h-1));
            assert_ne!(pixel(w,h,w/2,h/2,true,false),pixel(w,h,w/2,h/2,false,false));
            assert_ne!(pixel(w,h,w/2,h/2,false,true),pixel(w,h,w/2,h/2,false,false));
            for y in 0..h {for x in 0..w {assert_eq!(contains(w,h,x,y),contains(w,h,w-1-x,y));}}
            assert_eq!(halo(w,h,-1,0,true,true).3,0);
        }
    }
    // ------------------------=
    // FUNC: lightrail_and_clipped_targets_scale_together
    // DESC: Checks rail luminance, gradient, diagonal cutouts and close-target coverage at crowded widths.
    // ------------------=
    #[test] fn lightrail_and_clipped_targets_scale_together() {
        for scale in 1..=4 {for width in [65,112,224] {
            let (w,h)=(width*scale,36*scale);
            assert!(!contains(w,h,scale,scale));
            assert!(contains(w,h,5*scale,5*scale));
            let rail=pixel(w,h,w/2,scale,true,false);
            let idle=pixel(w,h,w/2,scale,false,false);
            assert!(rail.1>idle.1 && rail.2>idle.2);
            assert!(pixel(w,h,w/2,4*scale,true,false).2>pixel(w,h,w/2,h-2,true,false).2);
            for y in 6*scale..30*scale {for x in w-32*scale..w-8*scale {
                assert!(contains(w,h,x,y));
            }}
            for (x,y) in [(w,0),(0,h),(w,h)] {assert_eq!(pixel(w,h,x,y,true,true).3,0);}
        }}
    }
}
