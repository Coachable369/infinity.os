//! Session-local native text context menu and clipboard viewer geometry.
use crate::ui::geometry::{Point,Rect};
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Target {Editor,Browser,Address,Navigator}
pub const LABELS:[&[u8];7]=[b"Copy",b"Cut",b"Paste",b"Select all",b"See Clipboard",b"Search with Google",b"Remember selection"];
#[derive(Clone,Copy)]
pub struct Menu {
    pub open:bool,pub viewer:bool,pub target:Target,pub rect:Rect,pub scale:usize,
    pub row:usize,pub page:usize,pub bytes:[u8;16384],pub length:usize,
    pub query:[u8;64],pub query_length:usize,
    pub entries:[crate::ui::clipboard::HistoryPreview;crate::ui::clipboard::HISTORY_CAPACITY],
    pub selected_id:u64,
    pub can_paste:bool,
}
impl Menu {
    // ------------------------=
    // FUNC: new
    // DESC: Starts without retained clipboard data or an open overlay.
    // ------------------=
    pub const fn new()->Self {Self{open:false,viewer:false,target:Target::Editor,
        rect:Rect{x:0,y:0,width:0,height:0},scale:1,row:0,page:0,bytes:[0;16384],length:0,
        query:[0;64],query_length:0,entries:[crate::ui::clipboard::HistoryPreview::EMPTY;crate::ui::clipboard::HISTORY_CAPACITY],selected_id:0,can_paste:true}}
    // ------------------------=
    // FUNC: place
    // DESC: Keeps every row and the clipboard sheet inside the display at supported scales.
    // ------------------=
    pub fn place(&mut self,x:i32,y:i32,width:usize,height:usize,scale:usize) {
        let (base_width,base_height)=if self.viewer {(520,350)}else{(248,222)};
        self.scale=scale.max(1).min((width/base_width).max(1)).min((height/base_height).max(1));
        let w=base_width*self.scale;
        let h=base_height*self.scale;
        self.rect=Rect{x:x.clamp(0,width.saturating_sub(w) as i32),y:y.clamp(0,height.saturating_sub(h) as i32),
            width:w.min(width) as u32,height:h.min(height) as u32};
    }
    // ------------------------=
    // FUNC: row_at
    // DESC: Uses the painter's exact padded row bounds for context actions.
    // ------------------=
    pub fn row_at(&self,p:Point)->Option<usize> {
        if !self.open || self.viewer || !self.rect.contains(p) {return None;}
        let offset=p.y-self.rect.y-6*self.scale as i32;
        if offset<0 {return None;}
        let row=offset as usize/(30*self.scale);(row<LABELS.len()).then_some(row)
    }
    // ------------------------=
    // FUNC: viewer_action
    // DESC: Shares exact logical row and footer hit regions with the clipboard painter.
    // ------------------=
    pub fn viewer_action(&self,p:Point)->Option<usize> {
        if !self.open || !self.viewer || !self.rect.contains(p) {return None;}
        let x=(p.x-self.rect.x) as usize/self.scale;
        let y=(p.y-self.rect.y) as usize/self.scale;
        if x>=458 && y<42 {return Some(0);}
        if (16..200).contains(&x) && (100..292).contains(&y) && (y-100)%64<56 {
            return Some(10+self.row/3*3+(y-100)/64);
        }
        if (308..340).contains(&y) && (16..496).contains(&x) && (x-16)%80<74 {return Some(1+(x-16)/80);}
        None
    }
}
static mut MENU:Menu=Menu::new();
static mut DAMAGE:Option<Rect>=None;
// ------------------------=
// FUNC: current
// DESC: Copies the UI-thread-owned overlay without borrowing across input dispatch.
// ------------------=
pub fn current()->Menu {unsafe{MENU}}
// ------------------------=
// FUNC: publish
// DESC: Retains the old and new overlay footprint for restoration on dismissal.
// ------------------=
pub fn publish(menu:Menu) {unsafe{
    for r in [MENU.open.then_some(MENU.rect),menu.open.then_some(menu.rect)].into_iter().flatten() {
        DAMAGE=Some(DAMAGE.map_or(r,|d|d.union(r)));
    }
    MENU=menu;
}}
// ------------------------=
// FUNC: close
// DESC: Erases the viewer's copied data as well as dismissing the menu.
// ------------------=
pub fn close() {publish(Menu::new());}
// ------------------------=
// FUNC: take_damage
// DESC: Consumes only the overlay union that must be repainted underneath the next frame.
// ------------------=
pub fn take_damage()->Option<Rect> {unsafe{let value=DAMAGE;DAMAGE=None;value}}
