//! Shared coverage filtering for authored-size native font atlases.

// ------------------------=
// FUNC: coverage
// DESC: Area-filters reduced glyphs and bilinearly samples enlarged glyphs without sampling neighboring characters.
// ------------------=
pub fn coverage(atlas: &[u8], stride: usize, offset: usize, source_width: usize,
    source_height: usize, width: usize, height: usize, x: usize, y: usize) -> u8 {
    if width == 0 || height == 0 || x>=width || y>=height || source_width == 0 || source_height == 0 { return 0; }
    if width == source_width && height == source_height {
        return atlas.get(y * stride + offset + x).copied().unwrap_or(0);
    }
    if width < source_width || height < source_height {
        let x0=x*source_width*256/width; let x1=(x+1)*source_width*256/width;
        let y0=y*source_height*256/height; let y1=(y+1)*source_height*256/height;
        let mut sum=0u64; let mut area=0u64;
        for sy in y0/256..y1.div_ceil(256).min(source_height) {
            let wy=y1.min((sy+1)*256).saturating_sub(y0.max(sy*256));
            for sx in x0/256..x1.div_ceil(256).min(source_width) {
                let wx=x1.min((sx+1)*256).saturating_sub(x0.max(sx*256));
                let weight=(wx*wy) as u64;
                sum+=u64::from(atlas.get(sy*stride+offset+sx).copied().unwrap_or(0))*weight;
                area+=weight;
            }
        }
        return (sum/area.max(1)) as u8;
    }
    let fx=(((2*x+1)*source_width*128/width) as isize-128).max(0) as usize;
    let fy=(((2*y+1)*source_height*128/height) as isize-128).max(0) as usize;
    let (sx,sy)=(fx/256,fy/256); let (wx,wy)=(fx%256,fy%256);
    let mut sum=0usize;
    for (dy,weight_y) in [(0,256-wy),(1,wy)] {
        for (dx,weight_x) in [(0,256-wx),(1,wx)] {
            let at=(sy+dy).min(source_height-1)*stride+offset+(sx+dx).min(source_width-1);
            sum+=usize::from(atlas.get(at).copied().unwrap_or(0))*weight_x*weight_y;
        }
    }
    ((sum+32768)/65536) as u8
}
