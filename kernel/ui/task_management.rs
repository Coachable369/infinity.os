//! Shared Task Manager automation presentation, editing and hit geometry.
use super::geometry::{Point, Rect};
use super::text_input::{self, TextEditKey};

pub const CAPACITY: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab { Processes, Services, Scheduled }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Browse, Create, Edit, Delete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target { None, Tab(Tab), Row(usize), New, Edit, Delete, Start, Stop, Restart, Kill, Enabled, Trigger, RestartPolicy, TriggerOption(u8), RestartOption(u8), Field(usize), Save, Cancel, Confirm, Previous, Next }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback { None, Saved, Failed, Invalid, SaveOrCancel, Deleted }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text { pub bytes: [u8; 96], pub len: usize, pub caret: usize }
impl Text {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded field without allocating.
    // ------------------=
    pub const fn new() -> Self { Self { bytes: [0;96], len:0, caret:0 } }
    // ------------------------=
    // FUNC: from_bytes
    // DESC: Copies one label or script path into an editable presentation value.
    // ------------------=
    pub fn from_bytes(value: &[u8]) -> Self { let mut s=Self::new();s.len=value.len().min(96);s.bytes[..s.len].copy_from_slice(&value[..s.len]);s.caret=s.len;s }
    // ------------------------=
    // FUNC: bytes
    // DESC: Returns the populated portion of the field.
    // ------------------=
    pub fn bytes(&self) -> &[u8] { &self.bytes[..self.len] }
    // ------------------------=
    // FUNC: edit
    // DESC: Applies the shared bounded text editor to a field and its caret.
    // ------------------=
    pub fn edit(&mut self,key:TextEditKey)->bool { match key {
        TextEditKey::Character(c)=>text_input::insert_ascii(&mut self.bytes,&mut self.len,&mut self.caret,c),
        TextEditKey::Backspace=>text_input::backspace(&mut self.bytes,&mut self.len,&mut self.caret),
        TextEditKey::Delete=>text_input::delete(&mut self.bytes,&mut self.len,&mut self.caret),
        TextEditKey::Left=>text_input::move_caret(&mut self.caret,self.len,-1),
        TextEditKey::Right=>text_input::move_caret(&mut self.caret,self.len,1),
        TextEditKey::Home=>text_input::move_caret(&mut self.caret,self.len,-2),
        TextEditKey::End=>text_input::move_caret(&mut self.caret,self.len,2),
    } }
}
#[derive(Clone, Copy)]
pub struct Entry { pub id:u32, pub name:Text, pub path:Text, pub enabled:bool, pub trigger:u8, pub timing:u64, pub restart:u8, pub state:u8, pub next:u64, pub timing_text:Text, pub next_text:Text, pub exit:Option<i64>, pub runs:u64 }
impl Entry {
    // ------------------------=
    // FUNC: empty
    // DESC: Defines an unused catalog presentation slot.
    // ------------------=
    pub const fn empty()->Self { Self{id:0,name:Text::new(),path:Text::new(),enabled:false,trigger:0,timing:0,restart:0,state:0,next:0,timing_text:Text::new(),next_text:Text::new(),exit:None,runs:0} }
}
#[derive(Clone,Copy)]
pub struct State { pub tab:Tab, pub mode:Mode, pub selected:usize, pub first:usize, pub focus:Target, pub fields:[Text;3], pub enabled:bool, pub trigger:u8, pub restart:u8, pub choice:u8, pub feedback:Feedback }
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Opens on the existing Processes dashboard with no unsaved draft.
    // ------------------=
    pub const fn new()->Self { Self{tab:Tab::Processes,mode:Mode::Browse,selected:0,first:0,focus:Target::None,fields:[Text::new();3],enabled:true,trigger:0,restart:0,choice:0,feedback:Feedback::None} }
    // ------------------------=
    // FUNC: switch
    // DESC: Changes tabs only when a draft has been saved or explicitly cancelled.
    // ------------------=
    pub fn switch(&mut self,tab:Tab)->bool { if self.mode!=Mode::Browse {self.feedback=Feedback::SaveOrCancel;return false;} self.tab=tab;self.selected=0;self.first=0;self.focus=Target::Tab(tab);self.feedback=Feedback::None;true }
    // ------------------------=
    // FUNC: cancel
    // DESC: Discards a draft or deletion prompt and returns to its current record.
    // ------------------=
    pub fn cancel(&mut self) {self.mode=Mode::Browse;self.choice=0;self.focus=Target::New;self.feedback=Feedback::None;}
    // ------------------------=
    // FUNC: begin
    // DESC: Creates a new draft or copies a selected service record without mutating the catalog.
    // ------------------=
    pub fn begin(&mut self,mode:Mode,entry:Option<Entry>) {
        self.mode=mode;self.choice=0;self.feedback=Feedback::None;self.focus=if mode==Mode::Delete{Target::Cancel}else{Target::Field(0)};
        self.fields=[Text::new();3];self.enabled=true;self.trigger=if self.tab==Tab::Scheduled{2}else{0};self.restart=0;
        if mode!=Mode::Create {if let Some(e)=entry {self.fields[0]=e.name;self.fields[1]=e.path;self.fields[2]=if e.trigger==3{e.timing_text}else{number(e.timing)};self.enabled=e.enabled;self.trigger=e.trigger;self.restart=e.restart;}}
        else {self.fields[2]=Text::from_bytes(if self.tab==Tab::Scheduled{b"60"}else{b"0"});}
    }
    // ------------------------=
    // FUNC: select
    // DESC: Selects a bounded catalog entry while keeping its row visible.
    // ------------------=
    pub fn select(&mut self,index:usize,count:usize,rows:usize) {if self.mode!=Mode::Browse {self.feedback=Feedback::SaveOrCancel;return;}self.selected=index.min(count.saturating_sub(1));if self.selected<self.first{self.first=self.selected;}if self.selected>=self.first+rows.max(1){self.first=self.selected+1-rows.max(1);}self.focus=Target::Row(self.selected);self.feedback=Feedback::None;}
}
#[derive(Clone,Copy)]
pub struct View {pub owner:[u8;16],pub session:[u8;16],pub state:State,pub entries:[Entry;CAPACITY],pub count:usize,pub available:bool}
impl View {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an unavailable catalog view until the native service publishes it.
    // ------------------=
    pub const fn new()->Self {Self{owner:[0;16],session:[0;16],state:State::new(),entries:[Entry::empty();CAPACITY],count:0,available:false}}
    // ------------------------=
    // FUNC: selected
    // DESC: Retrieves the selected actual entry without fabricating an empty task.
    // ------------------=
    pub fn selected(&self)->Option<Entry> {(self.state.selected<self.count).then(||self.entries[self.state.selected])}
}
static mut VIEW: View=View::new();
#[no_mangle]
pub static mut INFINITY_TASK_MANAGEMENT_SNAPSHOT:[u64;8]=[0x495441534b554931,1,0,0,0,0,0,0];
// ------------------------=
// FUNC: current
// DESC: Reads the presentation snapshot on the serialized desktop event loop.
// ------------------=
pub fn current()->View {unsafe{VIEW}}
// ------------------------=
// FUNC: publish
// DESC: Publishes a completed UI state from the serialized desktop event loop.
// ------------------=
pub fn publish(view:View) {unsafe{
    VIEW=view;
    let sequence=INFINITY_TASK_MANAGEMENT_SNAPSHOT[2].wrapping_add(2);
    INFINITY_TASK_MANAGEMENT_SNAPSHOT[2]=sequence|1;
    INFINITY_TASK_MANAGEMENT_SNAPSHOT[3]=match view.state.tab{Tab::Processes=>0,Tab::Services=>1,Tab::Scheduled=>2};
    INFINITY_TASK_MANAGEMENT_SNAPSHOT[4]=match view.state.mode{Mode::Browse=>0,Mode::Create=>1,Mode::Edit=>2,Mode::Delete=>3};
    INFINITY_TASK_MANAGEMENT_SNAPSHOT[5]=view.state.selected as u64;INFINITY_TASK_MANAGEMENT_SNAPSHOT[6]=view.count as u64;
    core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
    INFINITY_TASK_MANAGEMENT_SNAPSHOT[7]=sequence;INFINITY_TASK_MANAGEMENT_SNAPSHOT[2]=sequence;
}}
// ------------------------=
// FUNC: number
// DESC: Formats an unsigned catalog timing or counter as a bounded editable value.
// ------------------=
pub fn number(mut value:u64)->Text {let mut scratch=[0;20];let mut n=20;loop{n-=1;scratch[n]=b'0'+(value%10)as u8;value/=10;if value==0{break;}}Text::from_bytes(&scratch[n..])}
// ------------------------=
// FUNC: decimal
// DESC: Parses timing only when every digit fits an unsigned value.
// ------------------=
pub fn decimal(text:&Text)->Option<u64> {if text.len==0{return None;}text.bytes().iter().try_fold(0u64,|n,c|if c.is_ascii_digit(){n.checked_mul(10)?.checked_add(u64::from(c-b'0'))}else{None})}

#[derive(Clone,Copy)]
pub struct Control {pub target:Target,pub bounds:Rect,pub enabled:bool}
#[derive(Clone,Copy)]
pub struct Layout {pub controls:[Control;48],pub count:usize,pub list:Rect,pub detail:Rect,pub status:Rect,pub rows:usize}
impl Layout {
    // ------------------------=
    // FUNC: push
    // DESC: Adds one bounded semantic control to shared paint and input geometry.
    // ------------------=
    fn push(&mut self,target:Target,bounds:Rect,enabled:bool) {if self.count<self.controls.len(){self.controls[self.count]=Control{target,bounds,enabled};self.count+=1;}}
    // ------------------------=
    // FUNC: hit
    // DESC: Resolves only enabled controls, in reverse paint order.
    // ------------------=
    pub fn hit(&self,point:Point)->Option<Target> {self.controls[..self.count].iter().rev().find(|c|c.enabled&&c.bounds.contains(point)).map(|c|c.target)}
    // ------------------------=
    // FUNC: tab
    // DESC: Advances keyboard focus through reachable enabled controls.
    // ------------------=
    pub fn tab(&self,state:&mut State,reverse:bool) {let start=self.controls[..self.count].iter().position(|c|c.target==state.focus).unwrap_or(if reverse{0}else{self.count.saturating_sub(1)});for n in 1..=self.count {let i=if reverse{(start+self.count-n)%self.count}else{(start+n)%self.count};if self.controls[i].enabled&&(state.choice==0||matches!((state.choice,self.controls[i].target),(1,Target::TriggerOption(_))|(2,Target::RestartOption(_)))){state.focus=self.controls[i].target;break;}}}
}
// ------------------------=
// FUNC: rect
// DESC: Builds pixel geometry from a shared logical parent and scale.
// ------------------=
fn rect(base:Rect,x:usize,y:usize,w:usize,h:usize,s:usize)->Rect {Rect{x:base.x+(x*s)as i32,y:base.y+(y*s)as i32,width:(w*s)as u32,height:(h*s)as u32}}
// ------------------------=
// FUNC: layout
// DESC: Defines tabs, list, inline editor and action hit regions for both production paint and input.
// ------------------=
pub fn layout(content:Rect,toolbar:Rect,view:&View,scale:usize)->Layout {
    let s=scale.max(1);let w=content.width as usize/s;let h=content.height as usize/s;
    let empty=Control{target:Target::None,bounds:Rect{x:0,y:0,width:0,height:0},enabled:false};
    let list_width=188.min(w/3);let dx=list_width+32;let dw=w.saturating_sub(dx+16);
    let mut l=Layout{controls:[empty;48],count:0,list:rect(content,16,56,list_width,h.saturating_sub(112),s),detail:rect(content,dx,16,dw,h.saturating_sub(64),s),status:rect(content,16,h.saturating_sub(38),w.saturating_sub(32),32,s),rows:h.saturating_sub(152)/48};
    let tw=(toolbar.width as usize/s).saturating_sub(128)/3;
    for(i,tab)in[Tab::Processes,Tab::Services,Tab::Scheduled].into_iter().enumerate(){l.push(Target::Tab(tab),rect(toolbar,116+i*tw,6,tw.saturating_sub(6),32,s),true);}
    if view.state.tab==Tab::Processes{return l;}
    l.push(Target::New,rect(toolbar,10,6,94,32,s),view.available&&view.state.mode==Mode::Browse);
    for row in 0..l.rows {let i=view.state.first+row;if i>=view.count{break;}l.push(Target::Row(i),rect(l.list,0,row*48,list_width,44,s),view.state.mode==Mode::Browse);}
    let nav_y=h.saturating_sub(88);
    l.push(Target::Previous,rect(content,16,nav_y,88,28,s),view.state.mode==Mode::Browse&&view.state.first>0);
    l.push(Target::Next,rect(content,112,nav_y,list_width.saturating_sub(96),28,s),view.state.mode==Mode::Browse&&view.state.first+l.rows<view.count);
    let d=l.detail;
    if matches!(view.state.mode,Mode::Create|Mode::Edit) {
        for i in 0..3 {l.push(Target::Field(i),rect(d,12,60+i*66,dw.saturating_sub(24),32,s),!(i==0&&view.state.mode==Mode::Edit));}
        l.push(Target::Trigger,rect(d,12,238,dw.saturating_sub(24),32,s),true);
        let half=dw.saturating_sub(32)/2;
        l.push(Target::Enabled,rect(d,12,280,if view.state.tab==Tab::Services{half}else{dw.saturating_sub(24)},32,s),true);
        if view.state.tab==Tab::Services{l.push(Target::RestartPolicy,rect(d,20+half,280,half,32,s),true);}
        l.push(Target::Save,rect(d,12,326,half,32,s),view.available);
        l.push(Target::Cancel,rect(d,20+half,326,half,32,s),true);
    } else if view.state.mode==Mode::Delete {
        l.push(Target::Confirm,rect(d,12,128,(dw.saturating_sub(32))/2,32,s),view.available);
        l.push(Target::Cancel,rect(d,20+(dw.saturating_sub(32))/2,128,(dw.saturating_sub(32))/2,32,s),true);
    } else {
        let valid=view.selected().is_some()&&view.available;
        let half=dw.saturating_sub(32)/2;
        for(i,target)in[Target::Edit,Target::Enabled,Target::Start,Target::Stop,Target::Restart,Target::Kill,Target::Delete].into_iter().enumerate(){l.push(target,rect(d,12+(i%2)*(half+8),196+(i/2)*40,half,32,s),valid&&view.selected().is_some_and(|e|match target {
            Target::Edit|Target::Delete=>!matches!(e.state,1..=4),Target::Start=>!matches!(e.state,1..=4),
            Target::Stop=>matches!(e.state,1..=4),Target::Kill=>matches!(e.state,2..=4),
            Target::Restart=>view.state.tab==Tab::Services,_=>true,
        }));}
    }
    if view.state.choice!=0 && matches!(view.state.mode,Mode::Create|Mode::Edit) {
        let count=if view.state.choice==2{3}else if view.state.tab==Tab::Services{2}else{4};
        let y=if view.state.choice==1{238}else{280};
        for i in 0..count {l.push(if view.state.choice==1{Target::TriggerOption(i as u8)}else{Target::RestartOption(i as u8)},rect(d,12,y-count*28-8+i*28,dw.saturating_sub(24),28,s),true);}
    }
    l
}
