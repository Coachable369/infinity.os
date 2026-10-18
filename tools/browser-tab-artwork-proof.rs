#[path = "../sdk/infinity-browser-core/tab_style.rs"]
mod tab;

// ------------------------=
// FUNC: label
// DESC: Overlays the actual native Inter atlas on the production-pixel review board.
// ------------------=
fn label(image:&mut [u8],width:usize,mut x:usize,y:usize,text:&[u8],scale:usize) {
    let size=14*scale;
    let atlas=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.atlas")).unwrap();
    let metrics=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.metrics")).unwrap();
    let kern=std::fs::read(format!("assets/fonts/InfinityBrowser-Regular-{size}.kern")).unwrap();
    let mut previous=None;
    for &byte in text {
        let index=(byte-32) as usize;
        if let Some(left)=previous {x=x.saturating_add_signed(kern[left*95+index] as isize-128);}
        for gy in 0..size+6 {for gx in 0..size {
            let a=atlas[gy*size*95+index*size+gx] as u32;
            let at=((y+gy)*width+x+gx)*3;
            for (c,value) in [217u32,227,245].into_iter().enumerate() {
                image[at+c]=((image[at+c] as u32*(255-a)+value*a)/255) as u8;
            }
        }}
        x+=metrics[index] as usize;previous=Some(index);
    }
}

// ------------------------=
// FUNC: main
// DESC: Renders actual production tab pixels across interaction states and scales for visual review.
// ------------------=
fn main() {
    let (width,height)=(1024usize,280usize);
    let mut image=vec![0u8;width*height*3];
    for pixel in image.chunks_exact_mut(3) {pixel.copy_from_slice(&[8,17,30]);}
    for (index,(active,hover)) in [(false,false),(false,true),(true,false),(true,true)].into_iter().enumerate() {
        let (w,h,x0,y0)=(224,36,24+index as i32*248,35);
        for y in -6..h as i32 {for x in -6..w as i32+6 {
            let (r,g,b,a)=if x>=0 && x<w as i32 && y>=0 {tab::pixel(w,h,x as u32,y as u32,active,hover)}
                else {tab::halo(w,h,x,y,active,hover)};
            let at=((y0+y) as usize*width+(x0+x) as usize)*3;
            for (c,value) in [r,g,b].into_iter().enumerate() {
                image[at+c]=((image[at+c] as u32*(255-a as u32)+value as u32*a as u32)/255) as u8;
            }
        }}
    }
    for (index,w) in [112,224].into_iter().enumerate() {
        let (w,h,x0,y0)=(w*2,72,32+index as i32*300,160);
        assert!(tab::contains(w,h,w/2,h/2));
        for y in -12..h as i32 {for x in -12..w as i32+12 {
            let (r,g,b,a)=if x>=0 && x<w as i32 && y>=0 {tab::pixel(w,h,x as u32,y as u32,true,false)}
                else {tab::halo(w,h,x,y,true,false)};
            let at=((y0+y) as usize*width+(x0+x) as usize)*3;
            for (c,value) in [r,g,b].into_iter().enumerate() {
                image[at+c]=((image[at+c] as u32*(255-a as u32)+value as u32*a as u32)/255) as u8;
            }
        }}
    }
    for (index,text) in [b"Inactive" as &[u8],b"Hover",b"Active",b"Active + hover"].into_iter().enumerate() {
        label(&mut image,width,60+index*248,46,text,1);
        let cx=24+index*248+204;let cy=53;
        for d in 0..9 {
            for x in [cx-4+d,cx+4-d] {
                let at=((cy-4+d)*width+x)*3;image[at..at+3].copy_from_slice(&[217,227,245]);
            }
        }
    }
    label(&mut image,width,56,182,b"Compact",2);
    label(&mut image,width,404,182,b"Infinity Browser",2);
    use std::io::Write;
    let mut output=std::fs::File::create("build/behavior-tests/browser-tab-artwork-proof.ppm").unwrap();
    write!(output,"P6\n{width} {height}\n255\n").unwrap();
    output.write_all(&image).unwrap();
}
