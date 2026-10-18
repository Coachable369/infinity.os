//! Shared, read-only quick-look state. The UI thread owns publication and content.
use crate::ui::geometry::{Point, Rect};
pub const CAPACITY: usize = 8 * 1024 * 1024;
// ------------------------=
// FUNC: can_read
// DESC: Restricts preview to an authenticated Personal Space reader and the file owner or shared legacy objects.
// ------------------=
pub fn can_read(authenticated:bool,personal:bool,owner:[u8;16],user:[u8;16])->bool {authenticated&&personal&&(owner==[0;16]||owner==user)}
#[derive(Clone,Copy,PartialEq,Eq)]
pub struct DesktopFile {pub path:[u8;96],pub length:usize}
pub const EMPTY_FILE:DesktopFile=DesktopFile{path:[0;96],length:0};
static mut DESKTOP:[DesktopFile;16]=[EMPTY_FILE;16];
static mut DESKTOP_DIRTY:bool=false;
// ------------------------=
// FUNC: desktop_files
// DESC: Copies the bounded desktop namespace projection without reading storage in paint.
// ------------------=
pub fn desktop_files()->[DesktopFile;16]{unsafe{DESKTOP}}
// ------------------------=
// FUNC: set_desktop_files
// DESC: Publishes only already-authorized regular desktop files.
// ------------------=
pub fn set_desktop_files(files:[DesktopFile;16]){unsafe{if DESKTOP!=files{DESKTOP=files;DESKTOP_DIRTY=true;}}}
// ------------------------=
// FUNC: desktop_damage
// DESC: Restores the desktop projection after an actual namespace/session change, not each frame.
// ------------------=
pub fn desktop_damage(width:usize,height:usize)->Option<Rect>{unsafe{if !DESKTOP_DIRTY{return None;}DESKTOP_DIRTY=false;Some(Rect{x:0,y:0,width:width as u32,height:height as u32})}}
// ------------------------=
// FUNC: desktop_rect
// DESC: Places desktop document tiles below the existing collection-reference row.
// ------------------=
pub fn desktop_rect(index:usize,width:usize,height:usize)->Rect {let columns=(width/110).clamp(1,8);let w=(width/columns).min(110);
    Rect{x:((index%columns)*w+16) as i32,y:(height*28/100+index/columns*100) as i32,width:w.saturating_sub(8) as u32,height:90}}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Text, Bitmap, Png, Unsupported, TooLarge, Unavailable, Directory }
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Preview {
    pub open: bool, pub menu: bool, pub rect: Rect, pub scale: usize,
    pub path: [u8; 96], pub path_len: usize, pub length: usize,
    pub kind: Kind, pub scroll: usize, pub lines: usize,
    pub owner: [u8;16], pub drag: Option<(Point, Rect, u8)>,
}
impl Preview {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty quick-look session without access to any file.
    // ------------------=
    pub const fn new() -> Self { Self {open:false,menu:false,rect:Rect{x:0,y:0,width:0,height:0},scale:1,
        path:[0;96],path_len:0,length:0,kind:Kind::Unavailable,scroll:0,lines:0,owner:[0;16],drag:None} }
    // ------------------------=
    // FUNC: place
    // DESC: Centers the sheet within the display using the reusable eight-pixel layout rhythm.
    // ------------------=
    pub fn place(&mut self,width:usize,height:usize,scale:usize) {
        self.scale=scale.max(1).min((width/640).max(1)).min((height/440).max(1));
        let w=(800*self.scale).min(width.saturating_sub(32));let h=(560*self.scale).min(height.saturating_sub(48));
        self.rect=Rect{x:((width-w)/2) as i32,y:((height-h)/2) as i32,width:w as u32,height:h as u32};
    }
    // ------------------------=
    // FUNC: body
    // DESC: Returns the single clipped content viewport used by painting and input.
    // ------------------=
    pub fn body(&self)->Rect {let s=self.scale as u32;Rect{x:self.rect.x+(20*s) as i32,y:self.rect.y+(64*s) as i32,
        width:self.rect.width.saturating_sub(56*s),height:self.rect.height.saturating_sub(104*s)}}
    // ------------------------=
    // FUNC: columns
    // DESC: Reserves enough width for bounded native text rows at the current scale.
    // ------------------=
    pub fn columns(&self)->usize {(self.body().width as usize/(12*self.scale)).max(1)}
    // ------------------------=
    // FUNC: rows
    // DESC: Counts fully visible rows without drawing through the footer.
    // ------------------=
    pub fn rows(&self)->usize {(self.body().height as usize/(24*self.scale)).max(1)}
    // ------------------------=
    // FUNC: scroll_by
    // DESC: Clamps navigation to actual wrapped content rather than an arbitrary page limit.
    // ------------------=
    pub fn scroll_by(&mut self,delta:isize) {self.scroll=self.scroll.saturating_add_signed(delta).min(self.lines.saturating_sub(self.rows()));}
    // ------------------------=
    // FUNC: close_rect
    // DESC: Shares the native close target between mouse input and rendering.
    // ------------------=
    pub fn close_rect(&self)->Rect {let s=self.scale as u32;Rect{x:self.rect.right()-(42*s) as i32,y:self.rect.y+(12*s) as i32,width:28*s,height:28*s}}
    // ------------------------=
    // FUNC: track
    // DESC: Places the scroll rail beside, never over, the preview contents.
    // ------------------=
    pub fn track(&self)->Rect {let b=self.body();Rect{x:b.right()+(12*self.scale) as i32,y:b.y,width:(8*self.scale) as u32,height:b.height}}
    // ------------------------=
    // FUNC: scroll_to_pointer
    // DESC: Converts a rail drag to a bounded scroll position.
    // ------------------=
    pub fn scroll_to_pointer(&mut self,p:Point) {let t=self.track();self.scroll=(p.y-t.y).max(0) as usize*self.lines.saturating_sub(self.rows())/(t.height as usize).max(1);self.scroll_by(0);}
}
// ------------------------=
// FUNC: classify
// DESC: Uses validated image headers or UTF-8 content; binary documents never become misleading text previews.
// ------------------=
pub fn classify(bytes:&[u8])->Kind {
    if crate::ui::bitmap::RuntimeBitmap::parse(bytes).is_some() {return Kind::Bitmap;}
    if bytes.starts_with(b"%PDF-") || bytes.starts_with(b"{\\rtf") || bytes.starts_with(b"PK\x03\x04") {return Kind::Unsupported;}
    if core::str::from_utf8(bytes).is_ok() && bytes.iter().all(|b|!b.is_ascii_control() || matches!(*b,b'\n'|b'\r'|b'\t')) {Kind::Text}else{Kind::Unsupported}
}
// ------------------------=
// FUNC: text_rows
// DESC: Enumerates wrapped UTF-8 rows without cutting a code point or adding artificial blank lines.
// ------------------=
pub fn text_rows(bytes:&[u8],columns:usize,mut row:impl FnMut(usize,&[u8]))->usize {
    let Ok(text)=core::str::from_utf8(bytes) else {return 0;};let mut index=0;
    for line in text.split('\n') {let line=line.strip_suffix('\r').unwrap_or(line);let mut start=0;let mut n=0;
        for (offset,_) in line.char_indices() {if n==columns.max(1) {row(index,&line.as_bytes()[start..offset]);index+=1;start=offset;n=0;}n+=1;}
        row(index,&line.as_bytes()[start..]);index+=1;
    }index
}
static mut STATE:Preview=Preview::new();
static mut CONTENT:[u8;CAPACITY]=[0;CAPACITY];
static mut PNG_COMPRESSED:[u8;CAPACITY]=[0;CAPACITY];
static mut PNG_RAW:[u8;CAPACITY]=[0;CAPACITY];
static mut PNG:Option<crate::ui::preview_png::Png>=None;
static mut DAMAGE:Option<Rect>=None;
// ------------------------=
// FUNC: decode_png
// DESC: Decodes once on open into bounded UI-thread scratch, never in the paint loop.
// ------------------=
pub fn decode_png(inflate:impl FnOnce(&[u8],&mut [u8])->bool)->bool {unsafe{PNG=crate::ui::preview_png::decode(&(&*(&raw const CONTENT))[..STATE.length],&mut *(&raw mut PNG_COMPRESSED),&mut *(&raw mut PNG_RAW),inflate);PNG.is_some()}}
// ------------------------=
// FUNC: png
// DESC: Borrows validated decoded pixels for synchronous preview composition.
// ------------------=
pub fn png(read:impl FnOnce(crate::ui::preview_png::Png,&[u8])){unsafe{if let Some(p)=PNG{read(p,&*(&raw const PNG_RAW));}}}
// ------------------------=
// FUNC: current
// DESC: Copies only small presentation state, never the file buffer.
// ------------------=
pub fn current()->Preview {unsafe{STATE}}
// ------------------------=
// FUNC: publish
// DESC: Retains old and new bounds so moving or closing restores all covered pixels.
// ------------------=
pub fn publish(state:Preview) {unsafe{for r in [STATE.open.then_some(STATE.rect),state.open.then_some(state.rect)].into_iter().flatten(){DAMAGE=Some(DAMAGE.map_or(r,|d|d.union(r)));}STATE=state;}}
// ------------------------=
// FUNC: content
// DESC: Borrows immutable file bytes only during synchronous UI-thread painting.
// ------------------=
pub fn content<R>(read:impl FnOnce(&[u8])->R)->R {unsafe{read(&(&*(&raw const CONTENT))[..STATE.length])}}
// ------------------------=
// FUNC: load
// DESC: Loads into bounded scratch storage and publishes only the successful byte count.
// ------------------=
pub fn load(read:impl FnOnce(&mut [u8])->Option<usize>)->Option<usize> {unsafe{let buffer=&mut *(&raw mut CONTENT);buffer.fill(0);read(buffer).filter(|n|*n<=CAPACITY)}}
// ------------------------=
// FUNC: close
// DESC: Erases file bytes and ownership when the preview closes or the session is revoked.
// ------------------=
pub fn close() {unsafe{(&mut *(&raw mut CONTENT)).fill(0);(&mut *(&raw mut PNG_RAW)).fill(0);(&mut *(&raw mut PNG_COMPRESSED)).fill(0);PNG=None;}publish(Preview::new());}
// ------------------------=
// FUNC: take_damage
// DESC: Consumes the exact previous/current overlay union for desktop restoration.
// ------------------=
pub fn take_damage()->Option<Rect> {unsafe{(&mut *(&raw mut DAMAGE)).take()}}
