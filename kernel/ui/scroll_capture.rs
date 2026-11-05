//! Shared thumb-drag math: retain the grab point and clamp at both rail endpoints.
// ------------------------=
// FUNC: offset
// DESC: Maps a captured pointer to content offset without accumulating rounding error or requiring the pointer to remain over the rail.
// ------------------=
pub fn offset(pointer:i32,track_top:i32,track_height:u32,thumb_height:u32,grab:i32,maximum:usize)->usize {
    let travel=track_height.saturating_sub(thumb_height)as i64;
    if travel==0||maximum==0{return 0;}
    let top=(pointer as i64-track_top as i64-grab as i64).clamp(0,travel)as u128;
    ((top*maximum as u128+travel as u128/2)/travel as u128).min(maximum as u128)as usize
}

#[derive(Clone,Copy)]
pub struct FileRail {pub x:i32,pub y:i32,pub width:u32,pub height:u32,pub thumb_y:i32,pub thumb_height:u32,pub maximum:usize}
// ------------------------=
// FUNC: file_rail
// DESC: Computes File Navigator's proportional rail from visible pixel rows, accounting for its four-column grid.
// ------------------=
pub fn file_rail(window:(usize,usize,usize,usize),scale:usize,rows:usize,extent:usize,scroll:usize)->Option<FileRail>{
    let (x,y,w,h)=window;let visible=h.saturating_sub(150*scale);
    let content=rows.saturating_mul(extent);let maximum=content.saturating_sub(visible);
    if maximum==0||visible==0{return None;}
    let size=(visible.saturating_mul(visible)/content).clamp((20*scale).min(visible),visible);
    let y=y+92*scale;
    Some(FileRail{x:(x+w.saturating_sub(18*scale))as i32,y:y as i32,width:(12*scale)as u32,height:visible as u32,
        thumb_y:(y+(visible-size)*scroll.min(maximum)/maximum)as i32,thumb_height:size as u32,maximum})
}
