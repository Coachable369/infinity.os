//! Session-private standalone terminal state; all mutation belongs to the UI thread.
use super::geometry::{Point, Rect};
use super::editor_tools::preferences::Preferences;
pub const ROWS: usize = 256;
pub const COLS: usize = 1024;
pub const INPUT: usize = 512;
pub const BACKGROUNDS: &[(&[u8], (u8,u8,u8))] = &[
    (b"Scheme default", (0,0,0)), (b"Black", (0,0,0)),
    (b"Midnight", (9,15,38)), (b"Graphite", (32,33,36)),
    (b"Deep teal", (8,33,36)), (b"Plum", (34,20,42)),
];
pub const PREFERENCE_ROWS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position { pub row: u64, pub column: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu { None, Appearance, Schemes, Fonts, Sizes, Clipboard }
pub struct State {
    pub row_override:usize,pub column_override:usize,pub opacity:u8,pub background:u8,
    pub help:bool,pub help_detail:bool,pub help_selected:usize,
    pub remote:bool,pub setting_value:usize,pub setting_edit:bool,
    lines: [[u8; COLS]; ROWS], lengths: [usize; ROWS],
    pub first: u64, pub count: usize, stream_open: bool, stream_cursor: usize, escape: u8,
    pub owner: [u8;16], pub authorized: bool, pub capture: bool,
    pub appearance: Preferences, pub revision: u32,
    pub input: [u8; INPUT], pub input_length: usize, pub cursor: usize,
    pub prompt: [u8;256], pub prompt_length: usize,
    pub scroll: usize, pub columns: usize, pub rows: usize,
    pub selection: Option<(Position,Position)>, pub dragging: bool, pub scroll_drag: bool, pub grab: i32,
    pub search: bool, pub query: [u8;64], pub query_length: usize, pub found: Option<Position>,
    pub menu: Menu, pub menu_row: usize, pub notice: &'static [u8],
    pub last_click: u64, pub click_position: Option<Position>, pub clicks: u8,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes bounded storage without heap allocation or host services.
    // ------------------=
    pub const fn new() -> Self {
        Self { row_override:0,column_override:0,opacity:255,background:0,help:false,help_detail:false,help_selected:0,remote:false,setting_value:0,setting_edit:false,lines:[[0;COLS];ROWS],lengths:[0;ROWS],first:0,count:0,stream_open:false,stream_cursor:0,escape:0,
            owner:[0;16],authorized:false,capture:false,appearance:Preferences{scheme:1,font:2,size:3},revision:0,
            input:[0;INPUT],input_length:0,cursor:0,prompt:[0;256],prompt_length:0,
            scroll:0,columns:80,rows:24,selection:None,dragging:false,scroll_drag:false,grab:0,
            search:false,query:[0;64],query_length:0,found:None,menu:Menu::None,menu_row:0,notice:b"",
            last_click:0,click_position:None,clicks:0 }
    }
    // ------------------------=
    // FUNC: changed
    // DESC: Invalidates retained pixels after an observable terminal state change.
    // ------------------=
    pub fn changed(&mut self) { self.revision=self.revision.wrapping_add(1); }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases history and all transient selection/search coordinates in place.
    // ------------------=
    pub fn clear(&mut self) {
        for row in &mut self.lines { row.fill(0); }
        self.lengths.fill(0);self.first=0;self.count=0;self.stream_open=false;self.stream_cursor=0;self.escape=0;
        self.scroll=0;self.selection=None;self.found=None;self.dragging=false;self.scroll_drag=false;self.changed();
    }
    // ------------------------=
    // FUNC: session
    // DESC: Revokes transcript and draft on lock or identity changes; returns whether preferences need loading.
    // ------------------=
    pub fn session(&mut self,owner:[u8;16],active:bool)->bool {
        if self.owner==owner && self.authorized==active {return false;}
        self.clear();self.owner=owner;self.authorized=active;self.capture=false;
        self.input.fill(0);self.input_length=0;self.cursor=0;self.prompt.fill(0);self.prompt_length=0;
        self.help=false;self.remote=false;self.setting_edit=false;self.row_override=0;self.column_override=0;self.opacity=255;self.background=0;
        self.search=false;self.query.fill(0);self.query_length=0;self.menu=Menu::None;self.notice=b"";
        self.appearance=Preferences{scheme:1,font:2,size:3};self.click_position=None;self.clicks=0;
        active
    }
    // ------------------------=
    // FUNC: end
    // DESC: Identifies the current draft row independently of ring storage slots.
    // ------------------=
    pub fn end(&self)->u64 {self.first+self.count as u64}
    // ------------------------=
    // FUNC: length
    // DESC: Returns output or live prompt length without copying the transcript.
    // ------------------=
    pub fn length(&self,row:u64)->usize {
        if row==self.end(){self.prompt_length+self.input_length}
        else if row>=self.first&&row<self.end(){self.lengths[row as usize%ROWS]}else{0}
    }
    // ------------------------=
    // FUNC: byte
    // DESC: Reads one logical cell from retained output or the integrated live prompt.
    // ------------------=
    pub fn byte(&self,p:Position)->u8 {
        if p.row==self.end(){if p.column<self.prompt_length {self.prompt[p.column]}else{self.input.get(p.column-self.prompt_length).copied().unwrap_or(b' ')}}
        else if p.row>=self.first&&p.row<self.end()&&p.column<self.length(p.row){self.lines[p.row as usize%ROWS][p.column]}else{b' '}
    }
    // ------------------------=
    // FUNC: new_line
    // DESC: Advances the fixed ring and invalidates only selection anchors that were evicted.
    // ------------------=
    fn new_line(&mut self) {
        if self.count==ROWS {self.first+=1;}else{self.count+=1;}
        let slot=(self.end()-1)as usize%ROWS;self.lines[slot].fill(0);self.lengths[slot]=0;
        self.stream_open=true;self.stream_cursor=0;
        if self.selection.is_some_and(|(a,b)|a.row<self.first||b.row<self.first){self.selection=None;}
        if self.found.is_some_and(|p|p.row<self.first){self.found=None;}
    }
    // ------------------------=
    // FUNC: append
    // DESC: Streams bounded terminal bytes, honoring CR, backspace, tabs and safely consuming escape sequences.
    // ------------------=
    pub fn append(&mut self,bytes:&[u8]) {
        let before=self.visual_count();
        for &b in bytes {
            if self.escape!=0 {
                self.escape=match (self.escape,b) {(1,b'[')=>2,(1,b']')=>3,(2,0x40..=0x7e)=>0,(2,_)=>2,(3,7)=>0,(3,27)=>4,(3,_)=>3,(4,b'\\')=>0,(4,_)=>3,_=>0};continue;
            }
            if b==27 {self.escape=1;continue;}
            if b==b'\r' {self.stream_cursor=0;continue;}
            if b==8 {self.stream_cursor=self.stream_cursor.saturating_sub(1);continue;}
            if b!=b'\n'&&b!=b'\t'&&!(32..=126).contains(&b){continue;}
            if !self.stream_open {self.new_line();}
            if b==b'\n' {self.stream_open=false;continue;}
            let copies=if b==b'\t'{4-self.stream_cursor%4}else{1};
            for _ in 0..copies {
                if self.stream_cursor==COLS {self.new_line();}
                let slot=(self.end()-1)as usize%ROWS;
                self.lines[slot][self.stream_cursor]=if b==b'\t'{b' '}else{b};self.stream_cursor+=1;
                self.lengths[slot]=self.lengths[slot].max(self.stream_cursor);
            }
        }
        if self.scroll!=0 {self.scroll=self.scroll.saturating_add(self.visual_count().saturating_sub(before));}
        self.scroll=self.scroll.min(self.max_scroll());self.changed();
    }
    // ------------------------=
    // FUNC: line
    // DESC: Appends complete command output without merging it into an unfinished process line.
    // ------------------=
    pub fn line(&mut self,bytes:&[u8]) {
        if self.stream_open {self.append(b"\n");}self.append(bytes);self.append(b"\n");
    }
    // ------------------------=
    // FUNC: publish
    // DESC: Synchronizes the authoritative command draft and namespace without repainting unchanged frames.
    // ------------------=
    pub fn publish(&mut self,path:&[u8],input:&[u8],cursor:usize) {
        self.publish_draft(path,input,cursor,true);
    }
    // ------------------------=
    // FUNC: publish_draft
    // DESC: Publishes the local prefix only while the local shell owns the terminal.
    // ------------------=
    pub fn publish_draft(&mut self,path:&[u8],input:&[u8],cursor:usize,local:bool) {
        let mut prompt=[0;256];let lead=b"infinity ";prompt[..lead.len()].copy_from_slice(lead);
        let n=path.len().min(256-lead.len()-3);prompt[lead.len()..lead.len()+n].copy_from_slice(&path[..n]);
        let length=lead.len()+n+3;prompt[length-3..length].copy_from_slice(b" % ");
        let length=if local{length}else{0};
        let input=&input[..input.len().min(INPUT)];let cursor=cursor.min(input.len());
        if self.prompt[..self.prompt_length]!=prompt[..length]||self.input[..self.input_length]!=*input||self.cursor!=cursor {
            self.prompt=prompt;self.prompt_length=length;self.input.fill(0);self.input[..input.len()].copy_from_slice(input);
            self.input_length=input.len();self.cursor=cursor;self.changed();
        }
    }
    // ------------------------=
    // FUNC: visual_rows
    // DESC: Reserves a trailing caret cell only for the live draft at exact wrap boundaries.
    // ------------------=
    fn visual_rows(&self,row:u64)->usize {(self.length(row)+usize::from(row==self.end())).max(1).div_ceil(self.columns.max(1))}
    // ------------------------=
    // FUNC: visual_count
    // DESC: Counts wrapped output and input rows using the same grid as paint and pointer hit testing.
    // ------------------=
    pub fn visual_count(&self)->usize {(self.first..=self.end()).map(|r|self.visual_rows(r)).sum()}
    // ------------------------=
    // FUNC: max_scroll
    // DESC: Returns bounded scroll distance from the live tail.
    // ------------------=
    pub fn max_scroll(&self)->usize {self.visual_count().saturating_sub(self.rows)}
    // ------------------------=
    // FUNC: top
    // DESC: Resolves the first visible visual row.
    // ------------------=
    pub fn top(&self)->usize {self.max_scroll().saturating_sub(self.scroll)}
    // ------------------------=
    // FUNC: visual
    // DESC: Resolves a visual row to a stable logical line and byte offset.
    // ------------------=
    pub fn visual(&self,mut index:usize)->Option<Position> {
        for row in self.first..=self.end(){let n=self.visual_rows(row);if index<n{return Some(Position{row,column:index*self.columns});}index-=n;}None
    }
    // ------------------------=
    // FUNC: index
    // DESC: Resolves a logical position back to a wrapped row.
    // ------------------=
    pub fn index(&self,p:Position)->usize {(self.first..p.row.min(self.end())).map(|r|self.visual_rows(r)).sum::<usize>()+p.column/self.columns.max(1)}
    // ------------------------=
    // FUNC: reflow
    // DESC: Preserves the top logical anchor across resize and font changes; tail-follow remains at the tail.
    // ------------------=
    pub fn reflow(&mut self,columns:usize,rows:usize) {
        let anchor=if self.scroll!=0 {self.visual(self.top())}else{None};
        self.columns=columns.max(1);self.rows=rows.max(1);
        self.scroll=anchor.map_or(0,|p|self.max_scroll().saturating_sub(self.index(p))); 
    }
    // ------------------------=
    // FUNC: scroll_by
    // DESC: Scrolls signed row deltas without losing the viewport bounds.
    // ------------------=
    pub fn scroll_by(&mut self,delta:isize) {self.scroll=(self.scroll as isize+delta).clamp(0,self.max_scroll() as isize)as usize;self.changed();}
    // ------------------------=
    // FUNC: selected
    // DESC: Applies half-open ordered selection independent of drag direction.
    // ------------------=
    pub fn selected(&self,p:Position)->bool {self.selection.is_some_and(|(a,b)|p>=a.min(b)&&p<a.max(b))}
    // ------------------------=
    // FUNC: copy
    // DESC: Copies logical text with real line breaks; overflow fails atomically rather than silently truncating.
    // ------------------=
    pub fn copy(&self,out:&mut [u8])->Option<usize> {
        let (a,b)=self.selection?;let (a,b)=(a.min(b),a.max(b));let mut n=0;
        for row in a.row.max(self.first)..=b.row.min(self.end()) {
            let start=if row==a.row{a.column}else{0};let end=if row==b.row{b.column.min(self.length(row))}else{self.length(row)};
            for column in start..end {if n==out.len(){return None;}out[n]=self.byte(Position{row,column});n+=1;}
            if row<b.row {if n==out.len(){return None;}out[n]=b'\n';n+=1;}
        }Some(n)
    }
    // ------------------------=
    // FUNC: input_range
    // DESC: Allows editing only the selected part of the live command, never immutable output or prompt text.
    // ------------------=
    pub fn input_range(&self)->Option<(usize,usize)> {
        let (a,b)=self.selection?;let (a,b)=(a.min(b),a.max(b));
        (a.row==self.end()&&b.row==self.end()&&a.column>=self.prompt_length&&a!=b)
            .then_some((a.column.saturating_sub(self.prompt_length).min(self.input_length),b.column.saturating_sub(self.prompt_length).min(self.input_length)))
    }
    // ------------------------=
    // FUNC: select_unit
    // DESC: Selects a complete word or logical line for repeated pointer clicks.
    // ------------------=
    pub fn select_unit(&mut self,p:Position,line:bool) {
        let len=self.length(p.row);let mut a=p.column.min(len);let mut b=a;
        if line{a=0;b=len;}else{
            let word=|v:u8|v.is_ascii_alphanumeric()||b"_-/.:~".contains(&v);
            let kind=word(self.byte(Position{row:p.row,column:a}));
            while a>0&&word(self.byte(Position{row:p.row,column:a-1}))==kind{a-=1;}
            while b<len&&word(self.byte(Position{row:p.row,column:b}))==kind{b+=1;}
        }self.selection=Some((Position{row:p.row,column:a},Position{row:p.row,column:b}));self.changed();
    }
    // ------------------------=
    // FUNC: matches
    // DESC: Tests a case-insensitive bounded query against actual transcript cells.
    // ------------------=
    fn matches(&self,p:Position)->bool {
        self.query_length!=0&&p.column+self.query_length<=self.length(p.row)&&(0..self.query_length).all(|i|
            self.byte(Position{column:p.column+i,..p}).eq_ignore_ascii_case(&self.query[i]))
    }
    // ------------------------=
    // FUNC: find
    // DESC: Searches in either direction with wraparound and reveals the matched row.
    // ------------------=
    pub fn find(&mut self,backward:bool) {
        let mut first=None;let mut last=None;let mut before=None;let mut after=None;
        for row in self.first..=self.end(){for column in 0..self.length(row){let p=Position{row,column};
            if self.matches(p){first=first.or(Some(p));last=Some(p);
                if self.found.is_some_and(|old|p<old){before=Some(p);}
                if self.found.is_none_or(|old|p>old)&&after.is_none(){after=Some(p);}
            }
        }}self.found=if backward{before.or(last)}else{after.or(first)};
        if let Some(p)=self.found {self.scroll=self.max_scroll().saturating_sub(self.index(p).saturating_sub(self.rows/2));}
        self.changed();
    }
    // ------------------------=
    // FUNC: background_color
    // DESC: Resolves the terminal surface independently of its text and ANSI colors.
    // ------------------=
    pub fn background_color(&self)->(u8,u8,u8) {
        if self.background == 0 {self.appearance.color(0)} else {BACKGROUNDS[self.background as usize].1}
    }
    // ------------------------=
    // FUNC: menu_count
    // DESC: Exposes only real executable menu entries.
    // ------------------=
    pub fn menu_count(&self)->usize {match self.menu {Menu::None=>0,Menu::Appearance=>PREFERENCE_ROWS,Menu::Schemes=>10,Menu::Fonts=>4,Menu::Sizes=>9,Menu::Clipboard=>5}}
}

#[derive(Clone, Copy)]
pub struct Layout {pub body:Rect,pub text:Rect,pub status:Rect,pub toolbar:Rect,pub appearance:Rect,pub find:Rect,pub clipboard:Rect,pub field:Rect,pub previous:Rect,pub next:Rect,pub close:Rect,pub track:Rect,pub cell:usize,pub line:usize,pub scale:usize}
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Defines the shared compact/HiDPI geometry for terminal painting and all input targets.
    // ------------------=
    pub fn new(body:Rect,toolbar:Rect,scale:usize,p:Preferences,search:bool)->Self {
        let s=scale.max(1);let pad=(18*s).min(body.width as usize/8);let footer=(28*s).min(body.height as usize/4);
        let top=if search{38*s}else{0};let cell=p.cell()*s;let line=p.line()*s;
        let text=Rect{x:body.x+pad as i32,y:body.y+(12*s+top)as i32,width:body.width.saturating_sub((pad*2+10*s)as u32),height:body.height.saturating_sub((24*s+top+footer)as u32)};
        let status=Rect{x:body.x,y:body.bottom()-footer as i32,width:body.width,height:footer as u32};
        let appearance=Rect{x:toolbar.x+8*s as i32,y:toolbar.y+4*s as i32,width:(120*s).min(toolbar.width as usize/3)as u32,height:toolbar.height.saturating_sub(8*s as u32)};
        let find=Rect{x:appearance.right()+6*s as i32,width:36*s as u32,..appearance};
        let clipboard=Rect{x:find.right()+6*s as i32,width:(92*s).min(toolbar.width as usize/4)as u32,..appearance};
        let close=Rect{x:body.right()-40*s as i32,y:body.y+4*s as i32,width:32*s as u32,height:30*s as u32};
        let next=Rect{x:close.x-36*s as i32,..close};let previous=Rect{x:next.x-36*s as i32,..close};
        let field=Rect{x:body.x+pad as i32,y:close.y,width:(previous.x-body.x-pad as i32-8*s as i32).max(0)as u32,height:close.height};
        let track=Rect{x:body.right()-10*s as i32,y:text.y,width:6*s as u32,height:text.height};
        Self{body,text,status,toolbar,appearance,find,clipboard,field,previous,next,close,track,cell,line,scale:s}
    }
    // ------------------------=
    // FUNC: header_menus
    // DESC: Shares File, Preferences and Performance label bounds between painting and pointer routing.
    // ------------------=
    pub fn header_menus(toolbar:Rect, scale:usize)->[Rect;3] {
        let s=scale.max(1) as u32;
        let end=toolbar.width.saturating_sub(112*s);
        let start=end.saturating_sub(244*s).clamp(54*s,168*s).min(end);
        let available=end-start;
        let mut left=toolbar.x+start as i32;
        [48u32,98,98].map(|nominal| {
            let width=nominal*(available.min(244*s))/244;
            let rect=Rect{x:left,y:toolbar.y-(48*s) as i32,width,height:48*s};
            left+=width as i32;
            rect
        })
    }
    // ------------------------=
    // FUNC: performance_panel_left
    // DESC: Keeps the Performance menu within the window when its label is near the controls.
    // ------------------=
    pub fn performance_panel_left(toolbar:Rect,scale:usize)->i32 {
        Self::header_menus(toolbar,scale)[2].x.min(toolbar.right()-(232*scale) as i32).max(toolbar.x)
    }
    // ------------------------=
    // FUNC: columns
    // DESC: Returns a nonzero exact glyph-cell count.
    // ------------------=
    pub fn columns(self)->usize {(self.text.width as usize/self.cell.max(1)).max(1)}
    // ------------------------=
    // FUNC: rows
    // DESC: Returns complete visible line cells only.
    // ------------------=
    pub fn rows(self)->usize {(self.text.height as usize/self.line.max(1)).max(1)}
    // ------------------------=
    // FUNC: at
    // DESC: Maps pointer position to the closest bounded logical transcript location.
    // ------------------=
    pub fn at(self,p:Point,state:&State)->Position {
        let r=((p.y-self.text.y).max(0)as usize/self.line.max(1)).min(self.rows()-1);
        let mut pos=state.visual(state.top()+r).unwrap_or(Position{row:state.end(),column:state.length(state.end())});
        pos.column=(pos.column+((p.x-self.text.x).max(0)as usize+self.cell/2)/self.cell.max(1)).min(state.length(pos.row));pos
    }
    // ------------------------=
    // FUNC: track_pointer
    // DESC: Captures the thumb's grab point and follows the pointer to either endpoint even outside the rail.
    // ------------------=
    pub fn track_pointer(self,state:&mut State,point:Point,pressed:bool,held:bool)->bool {
        if !state.scroll_drag {
            let hit=Rect{x:self.track.x-3*self.scale as i32,width:self.track.width+6*self.scale as u32,..self.track};
            if !pressed||!hit.contains(point)||state.remote||state.help||state.max_scroll()==0{return false;}
            let thumb=self.thumb(state);
            state.grab=if point.y>=thumb.y&&point.y<thumb.bottom(){point.y-thumb.y}else{thumb.height as i32/2};
        }
        let max=state.max_scroll();let next=max-super::scroll_capture::offset(point.y,self.track.y,self.track.height,self.thumb(state).height,state.grab,max);
        if state.scroll_drag!=held || next!=state.scroll{state.scroll_drag=held;state.scroll=next;state.changed();}
        true
    }
    // ------------------------=
    // FUNC: thumb
    // DESC: Computes a bounded scrollbar thumb including short transcripts.
    // ------------------=
    pub fn thumb(self,state:&State)->Rect {
        let h=self.track.height as usize;let thumb=(h*self.rows()/state.visual_count().max(self.rows())).max(20*self.scale).min(h);
        let y=(h-thumb)*state.top()/state.max_scroll().max(1);
        Rect{y:self.track.y+y as i32,height:thumb as u32,..self.track}
    }
    // ------------------------=
    // FUNC: menu_rect
    // DESC: Bounds menus to the window and caps row height on short viewports.
    // ------------------=
    pub fn menu_rect(self,state:&State)->(Rect,usize) {
        let top=self.toolbar.y;
        let available=(self.body.bottom()-top).max(1) as usize;
        let row=(30*self.scale).min(available/state.menu_count().max(1)).max(1);
        let width=(236*self.scale).min(self.body.width as usize);
        let x=if state.menu==Menu::Clipboard{self.clipboard.x}else{Self::header_menus(self.toolbar,self.scale)[1].x};
        (Rect{x:x.min(self.body.right()-width as i32).max(self.body.x),y:top,width:width as u32,height:(row*state.menu_count())as u32},row)
    }
}
static mut STATE: State=State::new();
// ------------------------=
// FUNC: inspect
// DESC: Borrows serialized UI state without copying its scrollback onto the kernel stack.
// ------------------=
pub fn inspect<R>(f:impl FnOnce(&State)->R)->R {unsafe{f(&*(&raw const STATE))}}
// ------------------------=
// FUNC: mutate
// DESC: Mutates state only from the console/UI event thread.
// ------------------=
pub fn mutate<R>(f:impl FnOnce(&mut State)->R)->R {unsafe{f(&mut *(&raw mut STATE))}}
pub struct Capture(bool);
impl Capture {
    // ------------------------=
    // FUNC: new
    // DESC: Temporarily routes authorized command output to this terminal, restoring prior routing on all returns.
    // ------------------=
    pub fn new(enabled:bool)->Self {Self(mutate(|s|core::mem::replace(&mut s.capture,enabled&&s.authorized)))}
}
impl Drop for Capture {
    // ------------------------=
    // FUNC: drop
    // DESC: Restores outer output routing after nested command dispatch.
    // ------------------=
    fn drop(&mut self){mutate(|s|s.capture=self.0&&s.authorized);}
}
// ------------------------=
// FUNC: output
// DESC: Captures complete command-engine lines only inside the authorized routing scope.
// ------------------=
pub fn output(bytes:&[u8]) {mutate(|s|{if s.capture&&s.authorized{s.line(bytes);}});}
// ------------------------=
// FUNC: stream
// DESC: Captures already-authorized asynchronous native process output without replaying it.
// ------------------=
pub fn stream(bytes:&[u8]) {mutate(|s|{if s.capture&&s.authorized{s.append(bytes);}});}

impl State {
    // ------------------------=
    // FUNC: settings
    // DESC: Encodes only public per-user preferences, never the terminal transcript or credentials.
    // ------------------=
    pub fn settings(&self)->[u8;16] {
        let mut b=[0;16];b[..8].copy_from_slice(&self.appearance.encode());
        b[8..10].copy_from_slice(&(self.row_override as u16).to_le_bytes());
        b[10..12].copy_from_slice(&(self.column_override as u16).to_le_bytes());b[12]=self.opacity;b[13]=2;b[14]=self.background;b
    }
    // ------------------------=
    // FUNC: restore_settings
    // DESC: Validates bounded geometry and version before applying a stored preference record atomically.
    // ------------------=
    pub fn restore_settings(&mut self,b:&[u8])->bool {
        if b.len()!=16 || b[12]<64 || b[15]!=0 {return false;}
        let background=match (b[13],b[14]) {
            (1,0)=>0,
            (2,index) if (index as usize)<BACKGROUNDS.len()=>index,
            _=>return false,
        };
        let Some(p)=Preferences::decode(&b[..8])else{return false;};
        let rows=u16::from_le_bytes([b[8],b[9]])as usize;let cols=u16::from_le_bytes([b[10],b[11]])as usize;
        if rows>96||cols>320{return false;}self.appearance=p;self.row_override=rows;self.column_override=cols;self.opacity=b[12];self.background=background;self.changed();true
    }
    // ------------------------=
    // FUNC: layout
    // DESC: Applies explicit grid limits within the available viewport; zero means measured automatic sizing.
    // ------------------=
    pub fn layout(&self,body:Rect,toolbar:Rect,scale:usize)->Layout {
        let mut body=body;
        if toolbar.height>0 && body.y>=toolbar.bottom() {
            body.y-=toolbar.height as i32;
            body.height+=toolbar.height;
        }
        let mut g=Layout::new(body,toolbar,scale,self.appearance,self.search);
        if self.row_override!=0 {g.text.height=g.text.height.min((self.row_override*g.line)as u32);}
        if self.column_override!=0 {g.text.width=g.text.width.min((self.column_override*g.cell)as u32);}
        g.text.height=g.text.height.min((96*g.line)as u32);g.text.width=g.text.width.min((320*g.cell)as u32);g
    }
}

#[no_mangle]
pub static mut INFINITY_COMMAND_TERMINAL_SNAPSHOT:[u64;16]=[0;16];
// ------------------------=
// FUNC: publish_snapshot
// DESC: Exposes coherent non-content terminal geometry and UI state for installed behavioral verification.
// ------------------=
pub fn publish_snapshot(){
    let values=inspect(|s|[0x494e465454593031,1,0,s.authorized as u64,s.columns as u64,s.rows as u64,s.count as u64,s.remote as u64,s.menu as u64,s.help as u64,s.help_detail as u64,s.help_selected as u64,s.appearance.font as u64,s.appearance.size as u64,s.opacity as u64,0]);
    unsafe{let p=&mut *core::ptr::addr_of_mut!(INFINITY_COMMAND_TERMINAL_SNAPSHOT);let seq=(p[2].wrapping_add(2))&!1;p[2]=seq|1;p[15]=seq|1;core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);p[..2].copy_from_slice(&values[..2]);p[3..15].copy_from_slice(&values[3..15]);core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);p[15]=seq;p[2]=seq;}
}
