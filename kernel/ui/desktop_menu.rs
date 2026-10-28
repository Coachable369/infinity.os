//! Validated, capability-neutral desktop navigation records authored by InfinityStudio.
use core::sync::atomic::{AtomicU32,Ordering};
pub const LIMIT:usize=128;
pub const PAGE:usize=10;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Item<'a>{pub id:[u8;16],pub label:&'a[u8],pub parent:Option<usize>,pub action:u8,pub target:u8,pub enabled:bool,pub checked:bool}
const EMPTY:Item<'static>=Item{id:[0;16],label:b"",parent:None,action:6,target:0,enabled:false,checked:false};
#[derive(Clone,Copy)]
pub struct Menu<'a>{pub items:[Item<'a>;LIMIT],pub count:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Destination{System(u8),Settings(u8),Application(u8),Screen(u8),Window(u8),Submenu(usize)}
impl<'a> Menu<'a>{
    // ------------------------=
    // FUNC: parse
    // DESC: Rejects truncated, duplicated, excessive or unresolvable menu records before any action can execute.
    // ------------------=
    pub fn parse(bytes:&'a[u8],screens:u16)->Option<Self>{
        if bytes.len()<8||bytes[..5]!=[73,78,65,86,1]{return None;}
        let expected=u16::from_le_bytes([bytes[5],bytes[6]])as usize;
        if expected>LIMIT||bytes[7]as usize>expected{return None;}
        let mut menu=Self{items:[EMPTY;LIMIT],count:0};let mut cursor=8;
        for _ in 0..bytes[7]{menu.read_node(bytes,&mut cursor,None,0,screens)?;}
        (menu.count==expected&&cursor==bytes.len()).then_some(menu)
    }
    // ------------------------=
    // FUNC: read_node
    // DESC: Parses one bounded subtree into stable flat records with parent links.
    // ------------------=
    fn read_node(&mut self,bytes:&'a[u8],cursor:&mut usize,parent:Option<usize>,depth:usize,screens:u16)->Option<()> {
        if depth>=4||self.count==LIMIT{return None;}
        let h=bytes.get(*cursor..cursor.checked_add(21)?)?;*cursor+=21;
        let mut id=[0;16];id.copy_from_slice(&h[..16]);
        let (action,target,flags,children,length)=(h[16],h[17],h[18],h[19],h[20]as usize);
        if length>63||flags&!3!=0||action>6||(action!=5&&children!=0)||self.items[..self.count].iter().any(|i|i.id==id){return None;}
        let label=bytes.get(*cursor..cursor.checked_add(length)?)?;*cursor+=length;
        let text=core::str::from_utf8(label).ok()?;
        if text.chars().any(|c|c.is_control())||(action!=6&&text.trim().is_empty()){return None;}
        let valid=match action {0=>target<10,1=>(1..=11).contains(&target),2=>target<5,3|4=>target>=4&&target as u16<=screens,5|6=>target==0,_=>false};
        if !valid{return None;}
        let index=self.count;self.count+=1;
        self.items[index]=Item{id,label,parent,action,target,enabled:flags&1!=0,checked:flags&2!=0};
        for _ in 0..children{self.read_node(bytes,cursor,Some(index),depth+1,screens)?;}
        Some(())
    }
    // ------------------------=
    // FUNC: children
    // DESC: Iterates sibling item indices in authored order.
    // ------------------=
    pub fn children(&self,parent:Option<usize>)->impl Iterator<Item=usize>+'_ {(0..self.count).filter(move|i|self.items[*i].parent==parent)}
    // ------------------------=
    // FUNC: destination
    // DESC: Resolves only enabled leaf/submenu actions, including enabled ancestry.
    // ------------------=
    pub fn destination(&self,index:usize)->Option<Destination>{
        let item=*self.items.get(index).filter(|_|index<self.count)?;
        let mut ancestor=Some(index);
        while let Some(i)=ancestor {if !self.items[i].enabled{return None;}ancestor=self.items[i].parent;}
        Some(match item.action {0=>Destination::System(item.target),1=>Destination::Settings(item.target),2=>Destination::Application(item.target),3=>Destination::Screen(item.target),4=>Destination::Window(item.target),5=>Destination::Submenu(index),_=>return None})
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct View{pub parent:Option<usize>,pub selected:usize,pub offset:usize}
impl View{
    // ------------------------=
    // FUNC: new
    // DESC: Starts at the root menu without restoring another session's navigation.
    // ------------------=
    pub const fn new()->Self {Self{parent:None,selected:0,offset:0}}
    // ------------------------=
    // FUNC: count
    // DESC: Includes a real back row in a submenu, keeping even empty menus navigable.
    // ------------------=
    pub fn count(self,menu:&Menu)->usize {menu.children(self.parent).count()+usize::from(self.parent.is_some())}
    // ------------------------=
    // FUNC: item
    // DESC: Resolves a logical row excluding the optional submenu back entry.
    // ------------------=
    pub fn item(self,menu:&Menu,row:usize)->Option<usize>{menu.children(self.parent).nth(row.checked_sub(usize::from(self.parent.is_some()))?)}
    // ------------------------=
    // FUNC: selectable
    // DESC: Treats separators and disabled entries as non-actions while keeping the back entry usable.
    // ------------------=
    pub fn selectable(self,menu:&Menu,row:usize)->bool {if self.parent.is_some()&&row==0{true}else{self.item(menu,row).is_some_and(|i|menu.destination(i).is_some())}}
    // ------------------------=
    // FUNC: select
    // DESC: Selects an actual actionable row and scrolls it into the bounded visible page.
    // ------------------=
    pub fn select(&mut self,menu:&Menu,row:usize){if self.selectable(menu,row){self.selected=row;if row<self.offset{self.offset=row;}if row>=self.offset+PAGE{self.offset=row+1-PAGE;}}}
    // ------------------------=
    // FUNC: step
    // DESC: Cycles keyboard focus across actionable siblings without modulo-by-zero on empty menus.
    // ------------------=
    pub fn step(&mut self,menu:&Menu,forward:bool){let n=self.count(menu);if n==0{return;}for step in 1..=n{let row=(self.selected+if forward{step}else{n-step})%n;if self.selectable(menu,row){self.select(menu,row);break;}}}
    // ------------------------=
    // FUNC: enter
    // DESC: Opens a real submenu and initializes its first available action.
    // ------------------=
    pub fn enter(&mut self,menu:&Menu,index:usize){if menu.destination(index)==Some(Destination::Submenu(index)){self.parent=Some(index);self.selected=0;self.offset=0;}}
    // ------------------------=
    // FUNC: back
    // DESC: Returns to the parent menu with the exited submenu selected.
    // ------------------=
    pub fn back(&mut self,menu:&Menu)->bool {let Some(parent)=self.parent else{return false;};self.parent=menu.items[parent].parent;self.offset=0;let row=menu.children(self.parent).position(|i|i==parent).unwrap_or(0)+usize::from(self.parent.is_some());self.select(menu,row);true}
}
static VIEW:AtomicU32=AtomicU32::new(0);
static REVISION:AtomicU32=AtomicU32::new(0);
static SCENE:AtomicU32=AtomicU32::new(0);
// ------------------------=
// FUNC: view
// DESC: Reads the menu's small serialized presentation state.
// ------------------=
pub fn view()->View {let n=VIEW.load(Ordering::Relaxed);View{parent:((n&255)!=0).then_some((n&255).saturating_sub(1)as usize),selected:((n>>8)&255)as usize,offset:((n>>16)&255)as usize}}
// ------------------------=
// FUNC: publish
// DESC: Publishes navigation and invalidates pixels even when row focus stays unchanged across submenu transitions.
// ------------------=
pub fn publish(v:View){let n=v.parent.map_or(0,|p|p as u32+1)|((v.selected as u32)<<8)|((v.offset as u32)<<16);if VIEW.swap(n,Ordering::Relaxed)!=n{REVISION.fetch_add(1,Ordering::Relaxed);}}
// ------------------------=
// FUNC: revision
// DESC: Supplies the retained compositor's menu content invalidation token.
// ------------------=
pub fn revision()->u32 {REVISION.load(Ordering::Relaxed)}
// ------------------------=
// FUNC: scene
// DESC: Reads the authored screen currently displayed by the native window host.
// ------------------=
pub fn scene()->u8 {SCENE.load(Ordering::Relaxed)as u8}
// ------------------------=
// FUNC: set_scene
// DESC: Changes the native authored-screen host without granting application or session authority.
// ------------------=
pub fn set_scene(id:u8){if SCENE.swap(id as u32,Ordering::Relaxed)!=id as u32{REVISION.fetch_add(1,Ordering::Relaxed);}}
