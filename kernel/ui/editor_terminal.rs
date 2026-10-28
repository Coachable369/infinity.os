//! Native editor dock: bounded scrollback, shared hit geometry and durable preferences.
use super::super::geometry::{Point, Rect};
pub const ROWS: usize = 192;
pub const COLS: usize = 192;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences { pub open: bool, pub height: u16 }
impl Preferences {
    // ------------------------=
    // FUNC: new
    // DESC: Keeps a discoverable collapsed terminal with a useful first expansion height.
    // ------------------=
    pub const fn new() -> Self { Self { open: false, height: 224 } }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes only layout choices, never shell input or user output.
    // ------------------=
    pub fn encode(self) -> [u8; 8] { let h=self.height.to_le_bytes(); [b'I',b'E',b'T',1,self.open as u8,h[0],h[1],0] }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt or unbounded preferences before application to a viewport.
    // ------------------=
    pub fn decode(b:&[u8])->Option<Self> {
        if b.len()!=8 || b[..4]!=[b'I',b'E',b'T',1] || b[4]>1 || b[7]!=0 {return None;}
        let height=u16::from_le_bytes([b[5],b[6]]);
        (112..=1024).contains(&height).then_some(Self{open:b[4]!=0,height})
    }
}
#[derive(Clone, Copy)]
pub struct State {
    pub preferences: Preferences, pub focused: bool, pub attached: bool,
    pub dragging: bool, pub scroll: usize, pub restart: bool,
    pub scroll_dragging: bool, pub scroll_drag_offset: i32,
    pub lines: [[u8;COLS];ROWS], pub lengths:[usize;ROWS], pub count:usize,
    pub input:[u8;512], pub input_length:usize, pub cursor:usize,
    pub running: bool, pub line_open: bool, pub streamed_bytes:u64,
    pub viewport_columns:usize, pub viewport_rows:usize,
}
impl State {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an isolated empty presentation on authentication without previous-user scrollback.
    // ------------------=
    pub const fn new()->Self {Self{preferences:Preferences::new(),focused:false,attached:false,dragging:false,scroll:0,restart:false,scroll_dragging:false,scroll_drag_offset:0,
        lines:[[0;COLS];ROWS],lengths:[0;ROWS],count:0,input:[0;512],input_length:0,cursor:0,running:false,line_open:false,streamed_bytes:0,viewport_columns:0,viewport_rows:0}}
    // ------------------------=
    // FUNC: visual_count
    // DESC: Counts soft-wrapped display rows without changing retained logical output or diagnostic identity.
    // ------------------=
    pub fn visual_count(&self,columns:usize)->usize {let width=columns.max(1);self.lengths[..self.count].iter().map(|n|(*n).max(1).div_ceil(width)).sum()}
    // ------------------------=
    // FUNC: visual_line
    // DESC: Maps one displayed row back to its exact logical output row and bounded byte segment.
    // ------------------=
    pub fn visual_line(&self,mut index:usize,columns:usize)->Option<(usize,usize,usize)> {
        let width=columns.max(1);
        for row in 0..self.count {let count=self.lengths[row].max(1).div_ceil(width);
            if index<count {let start=index*width;return Some((row,start,(start+width).min(self.lengths[row])));}index-=count;
        }None
    }
    // ------------------------=
    // FUNC: reflow
    // DESC: Preserves a scrolled reader's logical row and byte anchor when the dock or window changes size.
    // ------------------=
    pub fn reflow(&mut self,columns:usize,visible:usize){
        let columns=columns.max(1);let visible=visible.max(1);let maximum=self.visual_count(columns).saturating_sub(visible);
        if self.scroll>0&&self.viewport_columns>0&&(columns!=self.viewport_columns||visible!=self.viewport_rows){
            let first=self.visual_count(self.viewport_columns).saturating_sub(self.viewport_rows).saturating_sub(self.scroll);
            if let Some((row,offset,_))=self.visual_line(first,self.viewport_columns){
                let anchor:usize=self.lengths[..row].iter().map(|n|(*n).max(1).div_ceil(columns)).sum::<usize>()+offset/columns;
                self.scroll=maximum.saturating_sub(anchor);
            }
        }
        self.scroll=self.scroll.min(maximum);self.viewport_columns=columns;self.viewport_rows=visible;
    }
    // ------------------------=
    // FUNC: row
    // DESC: Advances bounded scrollback, preserving the most recent complete output.
    // ------------------=
    fn row(&mut self) {
        if self.scroll>0{self.scroll=self.scroll.saturating_add(1);}
        if self.count==ROWS {self.lines.copy_within(1..ROWS,0);self.lengths.copy_within(1..ROWS,0);self.count-=1;}
        self.lengths[self.count]=0;self.count+=1;self.line_open=true;
    }
    // ------------------------=
    // FUNC: append
    // DESC: Streams output without dropped chunks, with CR suppression and bounded line wrapping.
    // ------------------=
    pub fn append(&mut self,bytes:&[u8]) {
        self.streamed_bytes=self.streamed_bytes.saturating_add(bytes.len() as u64);
        for &b in bytes {
            if b==b'\r' {continue;}
            if !self.line_open {self.row();}
            if b==b'\n' {self.line_open=false;continue;}
            if self.lengths[self.count-1]==COLS {self.row();}
            let row=self.count-1;let n=self.lengths[row];
            if self.scroll>0&&self.viewport_columns>0&&n>0&&n%self.viewport_columns==0{self.scroll=self.scroll.saturating_add(1);}
            self.lines[row][n]=if b.is_ascii_graphic()||b==b' ' {b}else{b' '};self.lengths[row]+=1;
        }
    }
}
static mut STATE:State=State::new();
// ------------------------=
// FUNC: read
// DESC: Reads the UI-thread-owned terminal model for painting and hit dispatch.
// ------------------=
pub fn read()->State {unsafe{*(&raw const STATE)}}
// ------------------------=
// FUNC: inspect
// DESC: Reads compact status fields without copying the scrollback arena on every scheduler tick.
// ------------------=
pub fn inspect<T>(f:impl FnOnce(&State)->T)->T {unsafe{f(&*(&raw const STATE))}}
// ------------------------=
// FUNC: mutate
// DESC: Mutates the terminal on the same serialized desktop thread as native console output.
// ------------------=
pub fn mutate(f:impl FnOnce(&mut State)) {unsafe{f(&mut *(&raw mut STATE));}}
// ------------------------=
// FUNC: preferences
// DESC: Reads the compact layout without copying the scrollback buffer.
// ------------------=
pub fn preferences()->Preferences {unsafe{(*(&raw const STATE)).preferences}}
// ------------------------=
// FUNC: layout
// DESC: Reflows the same transcript used for paint and hit testing before resolving the exact scrollbar geometry.
// ------------------=
pub fn layout(area:Rect,scale:usize,cell:usize)->Layout {
    let provisional=Layout::new(area,scale,preferences(),0,0);
    if preferences().open{mutate(|state|state.reflow((provisional.output.width as usize/cell.max(1)).max(1),provisional.rows(scale)));}
    inspect(|state|Layout::new(area,scale,state.preferences,state.visual_count(state.viewport_columns),state.scroll))
}
#[derive(Clone,Copy,Debug)]
pub struct Layout {pub panel:Rect,pub splitter:Rect,pub header:Rect,pub output:Rect,pub input:Rect,pub toggle:Rect,pub run:Rect,pub stop:Rect,pub restart:Rect,pub track:Rect,pub thumb:Rect}
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Divides the document viewport while retaining code space and a bounded draggable dock.
    // ------------------=
    pub fn new(area:Rect,scale:usize,p:Preferences,rows:usize,scroll:usize)->Self {
        let s=scale.max(1) as u32; let header=32*s;
        let maximum=area.height.saturating_sub(72*s).max(header);
        let height=if p.open {(p.height as u32*s).max(112*s).min(maximum)}else{header.min(area.height)};
        let panel=Rect{x:area.x,y:area.bottom()-height as i32,width:area.width,height};
        let head=Rect{height:header.min(height),..panel};
        let input=Rect{x:panel.x+8*s as i32,y:panel.bottom()-(36*s) as i32,width:panel.width.saturating_sub(16*s),height:28*s};
        let output=Rect{x:panel.x+12*s as i32,y:head.bottom()+4*s as i32,width:panel.width.saturating_sub(34*s),height:height.saturating_sub(72*s)};
        let control=|index:u32|Rect{x:panel.right()-((3-index)*72*s+8*s) as i32,y:head.y+3*s as i32,width:68*s,height:26*s};
        let track=Rect{x:panel.right()-12*s as i32,y:output.y,width:5*s,height:output.height};
        let visible=(output.height/(18*s)).max(1) as usize;
        let thumb_height=if rows<=visible {track.height}else{(track.height as usize*visible/rows).max(16*s as usize).min(track.height as usize) as u32};
        let top=rows.saturating_sub(visible).saturating_sub(scroll.min(rows.saturating_sub(visible)));
        let thumb=Rect{y:track.y+(track.height.saturating_sub(thumb_height) as usize*top/rows.saturating_sub(visible).max(1)) as i32,height:thumb_height,..track};
        Self{panel,splitter:Rect{y:panel.y-3*s as i32,height:6*s,..panel},header:head,output,input,
            toggle:Rect{width:panel.width.saturating_sub(224*s),..head},run:control(0),stop:control(1),restart:control(2),track,thumb}
    }
    // ------------------------=
    // FUNC: rows
    // DESC: Returns complete readable scrollback rows with no drawing outside the dock.
    // ------------------=
    pub fn rows(self,scale:usize)->usize {(self.output.height as usize/(18*scale.max(1))).max(1)}
    // ------------------------=
    // FUNC: row_at
    // DESC: Resolves a clicked visible output row to its retained diagnostic line.
    // ------------------=
    pub fn row_at(self,p:Point,scale:usize,count:usize,scroll:usize)->Option<usize> {
        if !self.output.contains(p){return None;}
        let start=count.saturating_sub(self.rows(scale)).saturating_sub(scroll);
        let row=start+(p.y-self.output.y) as usize/(18*scale.max(1));(row<count).then_some(row)
    }
    // ------------------------=
    // FUNC: drag_scroll
    // DESC: Maps the grabbed scrollbar thumb to a bounded offset from the live output tail.
    // ------------------=
    pub fn drag_scroll(self,y:i32,grab:i32,scale:usize,count:usize)->usize {
        let maximum=count.saturating_sub(self.rows(scale));let range=self.track.height.saturating_sub(self.thumb.height).max(1);
        let offset=(y-self.track.y-grab).clamp(0,range as i32)as usize;
        maximum.saturating_sub(offset*maximum/range as usize)
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Location {pub line:usize,pub column:usize}
// ------------------------=
// FUNC: diagnostic
// DESC: Extracts compiler file:line:column coordinates only when they belong to the current source path.
// ------------------=
pub fn diagnostic(text:&[u8],path:&[u8])->Option<Location> {
    if path.is_empty() {return None;}
    let name=path.rsplit(|b|*b==b'/').next()?;
    for candidate in [path,name] {
        let Some(at)=text.windows(candidate.len()).position(|w|w==candidate) else{continue;};
        if at>0&&!matches!(text[at-1],b' '|b'\t'|b'"'|b'\''){continue;}
        let tail=&text[at+candidate.len()..];
        if tail.first()==Some(&b':') {
            let mut values=tail[1..].split(|b|*b==b':');
            let line=number(values.next()?)?;let column=values.next().and_then(number).unwrap_or(1);
            return (line>0).then_some(Location{line,column:column.max(1)});
        }
        if tail.starts_with(b"\", line ") {let line=number(tail[8..].split(|b|!b.is_ascii_digit()).next()?)?;return(line>0).then_some(Location{line,column:1});}
    }
    None
}
// ------------------------=
// FUNC: number
// DESC: Parses bounded positive diagnostic coordinates without allocations or overflow.
// ------------------=
fn number(bytes:&[u8])->Option<usize>{if bytes.is_empty(){return None;}bytes.iter().try_fold(0usize,|n,b|if b.is_ascii_digit(){n.checked_mul(10)?.checked_add((b-b'0')as usize)}else{None})}
