//! Generated browser glyph family, shared by live/installed ARM64 and x86_64.
const SIDE:usize=64;
const ATLAS:&[u8;16*SIDE*SIDE*4]=include_bytes!("../../assets/ui-design-kit/default/browser-glyphs-v1.rgba");

// ------------------------=
// FUNC: pixel
// DESC: Alpha-weighted area sampling preserves small glyph edges without decoding or allocating during paint.
// ------------------=
pub fn pixel(glyph:usize,size:usize,x:usize,y:usize,enabled:bool)->(u8,u8,u8,u8) {
    let glyph=if glyph==16 {12}else{glyph};
    if glyph>=16 || size==0 || x>=size || y>=size {return (0,0,0,0);}
    let x0=x*SIDE/size;let y0=y*SIDE/size;
    let x1=((x+1)*SIDE/size).max(x0+1).min(SIDE);
    let y1=((y+1)*SIDE/size).max(y0+1).min(SIDE);
    let mut a=0u32;let mut c=[0u32;3];
    for sy in y0..y1 {for sx in x0..x1 {
        let at=(glyph*SIDE*SIDE+sy*SIDE+sx)*4;
        let alpha=ATLAS[at+3] as u32;a+=alpha;
        for k in 0..3 {c[k]+=ATLAS[at+k] as u32*alpha;}
    }}
    if a==0 {return (0,0,0,0);}
    let alpha=a/((x1-x0)*(y1-y0)) as u32;
    ((c[0]/a) as u8,(c[1]/a) as u8,(c[2]/a) as u8,
        (if enabled {alpha}else{alpha*38/100}) as u8)
}
