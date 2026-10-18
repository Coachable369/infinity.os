//! Soft-shoulder tabs with shared native geometry and OS-derived dark colors.

// ------------------------=
// FUNC: shoulder
// DESC: Interpolates a rounded crown, sloped shoulder and flared foot in quarter pixels.
// ------------------=
fn shoulder(w:u32,h:u32,y:u32)->u32 {
    const PROFILE:[u32;37]=[100,80,73,69,67,65,63,61,59,57,55,53,51,49,47,45,43,41,39,37,35,33,31,29,27,25,23,21,19,17,14,11,8,5,3,1,0];
    let t=y*36*256/(h*4);let i=(t/256).min(35) as usize;
    let value=(PROFILE[i]*(256-t%256)+PROFILE[i+1]*(t%256))/256;
    (value*h/36).min(w)
}

// ------------------------=
// FUNC: coverage
// DESC: Supersamples soft sloping shoulders and their flat toolbar attachment.
// ------------------=
fn coverage(w:u32,h:u32,x:u32,y:u32)->u8 {
    if w==0 || h==0 || x>=w || y>=h {return 0;}
    let side=x.min(w-1-x)*4;
    if side>=shoulder(w,h,y*4) {return 255;}
    if side+3<shoulder(w,h,y*4+3) {return 0;}
    let mut count=0;
    for dy in 0..4 {for dx in 0..4 {
        let px=x*4+dx;let inset=shoulder(w,h,y*4+dy);
        if px>=inset && w*4-1-px>=inset {count+=1;}
    }}
    (count*255/16) as u8
}
// ------------------------=
// FUNC: contains
// DESC: Uses the visible soft-shoulder silhouette for tab pointer ownership.
// ------------------=
pub fn contains(w:u32,h:u32,x:u32,y:u32)->bool {coverage(w,h,x,y)>=128}
// ------------------------=
// FUNC: pixel
// DESC: Supplies the default OS palette for standalone component previews.
// ------------------=
pub fn pixel(w:u32,h:u32,x:u32,y:u32,active:bool,hover:bool)->(u8,u8,u8,u8) {
    themed_pixel(w,h,x,y,active,hover,0x082238,0x78b7df)
}
// ------------------------=
// FUNC: toolbar_color
// DESC: Derives a dark neutral surface tinted by the user's OS Primary color.
// ------------------=
pub fn toolbar_color(primary:u32)->[u8;3] {
    [34+((primary>>16)&255) as u8/4,34+((primary>>8)&255) as u8/4,34+(primary&255) as u8/4]
}
// ------------------------=
// FUNC: themed_pixel
// DESC: Renders quiet OS-themed raised tabs with subtle edges and no luminous rail.
// ------------------=
pub fn themed_pixel(w:u32,h:u32,x:u32,y:u32,active:bool,hover:bool,primary:u32,accent:u32)->(u8,u8,u8,u8) {
    let alpha=coverage(w,h,x,y);if alpha==0 {return (0,0,0,0);}
    let scale=(h/36).max(1);
    let solid_interior=y>=scale && x.min(w-1-x)*4>=shoulder(w,h,y*4)+4*scale;
    let mut interior=if solid_interior {16}else{0};
    if !solid_interior {for dy in 0..4 {for dx in 0..4 {
        let px=x*4+dx;let py=y*4+dy;
        let cut=shoulder(w,h,py)+4*scale;
        if py>=4*scale && px>=cut && w*4-1-px>=cut {interior+=1;}
    }}}
    let edge=if y<h-2*scale {255-(interior*255/16).min(alpha as u32)*255/alpha as u32}else{0};
    let base=toolbar_color(primary);let mut rgb=[0;3];
    for c in 0..3 {
        let color=base[c] as u32;
        let tint=(accent>>(16-8*c))&255;
        let border=((color*3+tint)/4+if active {26}else if hover {20}else{4}).min(255);
        let fill=if active {color+8*(h-1-y)/h}else{color*2/3}+if hover {6}else{0};
        rgb[c]=((border*edge+fill*(255-edge))/255) as u8;
    }
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
    // FUNC: shoulders_and_theme_colors_scale_together
    // DESC: Checks curved cutouts, close target coverage and actual Primary/Accent palette changes.
    // ------------------=
    #[test] fn shoulders_and_theme_colors_scale_together() {
        for scale in 1..=4 {for width in [65,112,224] {
            let (w,h)=(width*scale,36*scale);
            assert!(!contains(w,h,scale,scale));
            assert!(contains(w,h,18*scale,6*scale));
            let rail=pixel(w,h,w/2,scale,true,false);
            let idle=pixel(w,h,w/2,scale,false,false);
            assert!(rail.1>idle.1 && rail.2>idle.2);
            assert!(pixel(w,h,w/2,4*scale,true,false).2>pixel(w,h,w/2,h-2,true,false).2);
            for y in 6*scale..30*scale {for x in w-40*scale..w-16*scale {
                assert!(contains(w,h,x,y));
            }}
            for (x,y) in [(w,0),(0,h),(w,h)] {assert_eq!(pixel(w,h,x,y,true,true).3,0);}
            assert_ne!(themed_pixel(w,h,w/2,h/2,true,false,0x203040,0x80b0df),themed_pixel(w,h,w/2,h/2,true,false,0x502030,0x80b0df));
            assert_ne!(themed_pixel(w,h,w/2,0,true,false,0x203040,0x80b0df),themed_pixel(w,h,w/2,0,true,false,0x203040,0xd080b0));
            assert_eq!(themed_pixel(w,h,w/2,h-1,true,false,0x203040,0x80b0df),(42,46,50,255));
        }}
    }
    // ------------------------=
    // FUNC: shoulders_have_partial_coverage_and_antialiased_borders
    // DESC: Verifies fractional silhouette coverage and smooth border transitions at all display scales.
    // ------------------=
    #[test] fn shoulders_have_partial_coverage_and_antialiased_borders() {
        for scale in 1..=4 {
            let (w,h)=(224*scale,36*scale);let mut fractional=0;let mut blended=0;
            for y in 0..h {for x in 0..30*scale {
                let p=pixel(w,h,x,y,true,false);
                if p.3>0 && p.3<255 {fractional+=1;}
                let fill=42+8*(h-1-y)/h;
                let border=(42*3+183)/4+26;
                if p.3==255 && p.1 as u32>fill && (p.1 as u32)<border {blended+=1;}
            }}
            assert!(fractional>=h/2);
            assert!(blended>0);
        }
    }
}
