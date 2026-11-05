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
