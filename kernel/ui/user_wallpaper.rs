//! UI-thread-owned wallpaper cache. Decode once; damage paints use the same bitmap.
use super::bitmap::RuntimeBitmap;
pub const LIMIT: usize = 8 * 1024 * 1024;
static mut PIXELS: [u8; LIMIT] = [0; LIMIT];
static mut LENGTH: usize = 0;
static mut DIRTY: bool = false;
static mut OWNER: [u8;16] = [0;16];
// ------------------------=
// FUNC: image_path
// DESC: Offers image actions for known image extensions; decoding remains authoritative.
// ------------------=
pub fn image_path(path:&[u8])->bool {let ext=path.rsplit(|b|*b==b'.').next().unwrap_or(b"");[b"png".as_slice(),b"bmp",b"jpg",b"jpeg",b"webp",b"gif"].iter().any(|v|ext.eq_ignore_ascii_case(v))}
// ------------------------=
// FUNC: encode
// DESC: Converts bounded decoded pixels into a validated top-down opaque BMP cache.
// ------------------=
pub fn encode(out:&mut[u8],w:usize,h:usize,mut pixel:impl FnMut(usize,usize)->Option<[u8;4]>)->Option<usize>{
    if w==0||h==0||w>8192||h>8192{return None;}
    let stride=(w*3+3)&!3;let n=54usize.checked_add(stride.checked_mul(h)?)?;if n>out.len(){return None;}
    out[..54].fill(0);out[..2].copy_from_slice(b"BM");out[2..6].copy_from_slice(&(n as u32).to_le_bytes());out[10..14].copy_from_slice(&54u32.to_le_bytes());out[14..18].copy_from_slice(&40u32.to_le_bytes());
    out[18..22].copy_from_slice(&(w as i32).to_le_bytes());out[22..26].copy_from_slice(&(-(h as i32)).to_le_bytes());out[26..28].copy_from_slice(&1u16.to_le_bytes());out[28..30].copy_from_slice(&24u16.to_le_bytes());out[54..n].fill(0);
    for y in 0..h{for x in 0..w{let [r,g,b,a]=pixel(x,y)?;let i=54+y*stride+x*3;out[i..i+3].copy_from_slice(&[((b as u16*a as u16)/255)as u8,((g as u16*a as u16)/255)as u8,((r as u16*a as u16)/255)as u8]);}}
    RuntimeBitmap::parse(&out[..n])?;Some(n)
}
// ------------------------=
// FUNC: install
// DESC: Publishes already validated decoded pixels for the current authenticated owner.
// ------------------=
pub fn install(owner:[u8;16],w:usize,h:usize,pixel:impl FnMut(usize,usize)->Option<[u8;4]>)->bool{unsafe{LENGTH=0;let Some(n)=encode(&mut *(&raw mut PIXELS),w,h,pixel)else{DIRTY=true;return false;};LENGTH=n;OWNER=owner;DIRTY=true;true}}
// ------------------------=
// FUNC: bitmap
// DESC: Exposes the retained bitmap without file reads, decoding or allocations during painting.
// ------------------=
pub fn bitmap<R>(f:impl FnOnce(Option<&[u8]>)->R)->R{unsafe{f((LENGTH!=0).then(||&PIXELS[..LENGTH]))}}
// ------------------------=
// FUNC: session
// DESC: Removes private wallpaper pixels when authorization or owner changes.
// ------------------=
pub fn session(owner:[u8;16],active:bool){unsafe{if LENGTH!=0&&(!active||OWNER!=owner){PIXELS[..LENGTH].fill(0);LENGTH=0;OWNER=[0;16];DIRTY=true;}}}
// ------------------------=
// FUNC: take_damage
// DESC: Invalidates retained desktop and glass caches exactly once after a wallpaper change.
// ------------------=
pub fn take_damage(width:usize,height:usize)->Option<super::geometry::Rect>{unsafe{if !DIRTY{return None;}DIRTY=false;Some(super::geometry::Rect{x:0,y:0,width:width as u32,height:height as u32})}}
