#[path = "../kernel/ui/assistant_tab.rs"]
mod tab;

// ------------------------=
// FUNC: main
// DESC: Verifies mirrored production pixels and renders the shared fin recipe for visual review.
// ------------------=
fn main() {
    for s in 1..=3 {
        let (w,h)=(48*s,104*s);
        assert!(tab::pixel(w,h,0,(h/2) as i32,false,48).3>230);
        assert_eq!(tab::pixel(w,h,(w-1) as i32,0,false,48).3,0);
        assert!(tab::pixel(w,h,(w-1) as i32,(h/2) as i32,false,48).3>0);
        for y in 0..h as i32 {for x in 0..w as i32 {
            assert_eq!(tab::pixel(w,h,x,y,false,48),tab::pixel(w,h,w as i32-x-1,y,true,48));
        }}
    }
    let (width,height)=(780usize,360usize);
    let mut image=vec![0u8;width*height*3];
    for y in 0..height {for x in 0..width {
        let at=(y*width+x)*3;
        image[at..at+3].copy_from_slice(&[9,15,29]);
        let local=x%260;
        if (25..180).contains(&local) && (30..330).contains(&y) {
            image[at..at+3].copy_from_slice(&[18,23,33]);
        }
        let (r,g,b,a)=tab::pixel(72,156,local as i32-179,y as i32-95,false,if x<260 {48}else{180});
        for (c,value) in [r,g,b].into_iter().enumerate() {
            image[at+c]=((image[at+c] as u32*(255-a as u32)+value as u32*a as u32)/255) as u8;
        }
    }}
    use std::io::Write;
    let mut output=std::fs::File::create("build/behavior-tests/assistant-tab-proof.ppm").unwrap();
    write!(output,"P6\n{width} {height}\n255\n").unwrap();
    output.write_all(&image).unwrap();
}
