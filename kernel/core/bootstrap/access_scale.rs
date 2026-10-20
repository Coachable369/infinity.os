//! Bounded bilinear panel scaling shared by native targets and pixel tests.

// ------------------------=
// FUNC: sample
// DESC: Filters packed framebuffer channels independently without changing pixel format.
// ------------------=
pub fn sample(source: &[u32], width: usize, height: usize, x: usize, y: usize,
    output_width: usize, output_height: usize) -> u32 {
    if width==0 || height==0 || output_width==0 || output_height==0 || source.len()<width*height { return 0; }
    let fx=if output_width>1 { x.min(output_width-1)*(width-1)*256/(output_width-1) } else { (width-1)*128 };
    let fy=if output_height>1 { y.min(output_height-1)*(height-1)*256/(output_height-1) } else { (height-1)*128 };
    let (sx,sy)=(fx/256,fy/256);
    let (tx,ty)=((fx%256) as u32,(fy%256) as u32);
    let pixels=[source[sy*width+sx],source[sy*width+(sx+1).min(width-1)],
        source[(sy+1).min(height-1)*width+sx],source[(sy+1).min(height-1)*width+(sx+1).min(width-1)]];
    let mut result=0;
    for shift in [0,8,16,24] {
        let a=((pixels[0]>>shift)&255)*(256-tx)+((pixels[1]>>shift)&255)*tx;
        let b=((pixels[2]>>shift)&255)*(256-tx)+((pixels[3]>>shift)&255)*tx;
        result|=((a*(256-ty)+b*ty+32768)/65536)<<shift;
    }
    result
}
