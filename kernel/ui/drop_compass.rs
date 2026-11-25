//! UI-thread-owned desktop hold gesture and shared radial geometry.
use super::geometry::{Point,Rect};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Compass { pub pending:Option<(Point,u64)>,pub center:Option<Point>,pub selected:Option<usize>,pub file:bool }
impl Compass {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no retained gesture or user data.
    // ------------------=
    pub const fn new()->Self {Self{pending:None,center:None,selected:None,file:false}}
    // ------------------------=
    // FUNC: arm
    // DESC: Begins a deliberate stationary hold without opening on an ordinary click.
    // ------------------=
    pub fn arm(&mut self,p:Point,now:u64,file:bool){*self=Self{pending:Some((p,now)),file,..Self::new()};}
    // ------------------------=
    // FUNC: advance
    // DESC: Opens after 550ms and cancels on release or movement beyond eight pixels.
    // ------------------=
    pub fn advance(&mut self,p:Point,held:bool,now:u64,w:usize,h:usize)->bool{
        let old=*self;
        if let Some((origin,start))=self.pending {
            let dx=i64::from(p.x-origin.x);let dy=i64::from(p.y-origin.y);
            if !held || dx*dx+dy*dy>64 {self.pending=None;}
            else if now.saturating_sub(start)>=550 {
                self.center=Some(Point{x:p.x.clamp(380.min(w as i32/2),(w as i32-380).max(w as i32/2)),y:p.y.clamp(270.min(h as i32/2),(h as i32-270).max(h as i32/2))});self.pending=None;
            }
        }
        if self.center.is_some(){self.selected=self.cards(w,h).iter().position(|r|r.contains(p));}
        old!=*self
    }
    // ------------------------=
    // FUNC: cards
    // DESC: Lays out four edge-safe cards around the shared circular glass surface.
    // ------------------=
    pub fn cards(self,w:usize,h:usize)->[Rect;4]{
        let c=self.center.unwrap_or(Point{x:0,y:0});let sx=(w as i32).min(760);let sy=(h as i32-80).max(240).min(520);
        let cw=(sx*34/100).min(244);let ch=76.min(sy/5);let rx=(sx-cw)/2;let ry=(sy-ch)/2;
        [(0,-ry),(rx,0),(0,ry),(-rx,0)].map(|(x,y)|Rect{x:c.x+x-cw/2,y:c.y+y-ch/2,width:cw as u32,height:ch as u32})
    }
    // ------------------------=
    // FUNC: activate
    // DESC: Captures destination and file mode before dismissal; releasing over no card keeps the hold menu usable.
    // ------------------=
    pub fn activate(&mut self,clicked:bool,released:bool,cancel:bool)->Option<(usize,bool)> {
        if cancel {self.dismiss();return None;}
        let action=if clicked||released {self.selected.map(|i|(i,self.file))}else{None};
        if clicked||action.is_some(){self.dismiss();}
        action
    }
    // ------------------------=
    // FUNC: dismiss
    // DESC: Clears presentation and any pending gesture without performing an action.
    // ------------------=
    pub fn dismiss(&mut self){*self=Self::new();}
}
static mut STATE:Compass=Compass::new();
static mut OLD:Option<Rect>=None;
// ------------------------=
// FUNC: bounds
// DESC: Covers all compass pixels including the four destination cards.
// ------------------=
fn bounds(state:Compass)->Option<Rect>{state.center.map(|c|Rect{x:c.x-390,y:c.y-275,width:780,height:550})}
// ------------------------=
// FUNC: take_damage
// DESC: Restores both dismissed and visible overlays before alpha compositing to prevent trails.
// ------------------=
pub fn take_damage()->Option<Rect>{unsafe{let old=OLD;OLD=None;match(old,bounds(STATE)){(Some(a),Some(b))=>Some(a.union(b)),(a,b)=>a.or(b)}}}
// ------------------------=
// FUNC: current
// DESC: Copies UI-thread state for paint without accessing storage or services.
// ------------------=
pub fn current()->Compass{unsafe{STATE}}
// ------------------------=
// FUNC: publish
// DESC: Publishes state on the owning UI thread.
// ------------------=
pub fn publish(state:Compass){unsafe{if STATE!=state{if let Some(r)=bounds(STATE){OLD=Some(OLD.map_or(r,|old|old.union(r)));}STATE=state;}}}
