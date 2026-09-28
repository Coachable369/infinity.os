#[path = "../sdk/infinity-browser-core/tab_style.rs"]
mod tab;

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
    use std::io::Write;
    let mut output=std::fs::File::create("build/behavior-tests/browser-tab-artwork-proof.ppm").unwrap();
    write!(output,"P6\n{width} {height}\n255\n").unwrap();
    output.write_all(&image).unwrap();
}
