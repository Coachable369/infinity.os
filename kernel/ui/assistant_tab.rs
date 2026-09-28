//! Default Infinity UI Design Kit image-backed attached assistant tab.
const WIDTH: usize = 112;
const HEIGHT: usize = 416;
const IMAGE: &[u8; WIDTH * HEIGHT * 4] = include_bytes!("../../assets/ui-design-kit/default/ai-window-tab-v1.rgba");

// ------------------------=
// FUNC: pixel
// DESC: Area-samples the transparent kit sprite, mirrored for left attachment, without runtime allocation.
// ------------------=
pub fn pixel(width:u32,height:u32,x:i32,y:i32,left:bool,emphasis:u8)->(u8,u8,u8,u8) {
    if width==0 || height==0 || x<0 || y<0 || x>=width as i32 || y>=height as i32 {return (0,0,0,0);}
    let x=if left {width as usize-1-x as usize}else{x as usize};
    let y=y as usize;
    let x0=(x*WIDTH/width as usize).min(WIDTH-1);
    let y0=(y*HEIGHT/height as usize).min(HEIGHT-1);
    let x1=((x+1)*WIDTH/width as usize).max(x0+1).min(WIDTH);
    let y1=((y+1)*HEIGHT/height as usize).max(y0+1).min(HEIGHT);
    let mut channels=[0u32;3];
    let mut alpha=0u32;
    for sy in y0..y1 {for sx in x0..x1 {
        let at=(sy*WIDTH+sx)*4;
        let a=IMAGE[at+3] as u32;
        alpha+=a;
        for c in 0..3 {channels[c]+=IMAGE[at+c] as u32*a;}
    }}
    if alpha==0 {return (0,0,0,0);}
    let gain=256+emphasis.saturating_sub(48) as u32/5;
    let color=|c:usize| ((channels[c]/alpha)*gain/256).min(255) as u8;
    (color(0),color(1),color(2),(alpha/((x1-x0)*(y1-y0)) as u32) as u8)
}
