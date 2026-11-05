//! Bounded VT terminal used by authenticated remote shells; escape sequences never execute OS actions.
pub const WIDTH:usize=320;
pub const HEIGHT:usize=96;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Cell {pub byte:u8,pub fg:u8,pub bg:u8,pub bold:bool,pub inverse:bool}
const EMPTY:Cell=Cell{byte:b' ',fg:7,bg:0,bold:false,inverse:false};
pub struct Terminal {
    pub cells:[[Cell;WIDTH];HEIGHT],saved:[[Cell;WIDTH];HEIGHT],pub columns:usize,pub rows:usize,
    pub x:usize,pub y:usize,saved_cursor:(usize,usize),main_cursor:(usize,usize),pub cursor_visible:bool,pub application_cursor:bool,
    style:Cell,state:u8,params:[usize;16],param:usize,private:bool,top:usize,bottom:usize,wrap:bool,pending_wrap:bool,alternate:bool,graphics:bool,
    reply:[u8;64],reply_len:usize,
}
impl Terminal {
    // ------------------------=
    // FUNC: new
    // DESC: Constructs a fixed-memory VT grid without heap allocation.
    // ------------------=
    pub const fn new()->Self {Self{cells:[[EMPTY;WIDTH];HEIGHT],saved:[[EMPTY;WIDTH];HEIGHT],columns:80,rows:24,x:0,y:0,saved_cursor:(0,0),main_cursor:(0,0),cursor_visible:true,application_cursor:false,style:EMPTY,state:0,params:[0;16],param:0,private:false,top:0,bottom:23,wrap:true,pending_wrap:false,alternate:false,graphics:false,reply:[0;64],reply_len:0}}
    // ------------------------=
    // FUNC: reset
    // DESC: Erases visible and alternate buffers on connection/session boundaries without a large stack copy.
    // ------------------=
    pub fn reset(&mut self){for row in &mut self.cells{row.fill(EMPTY);}for row in &mut self.saved{row.fill(EMPTY);}self.x=0;self.y=0;self.state=0;self.top=0;self.bottom=self.rows-1;self.style=EMPTY;self.alternate=false;self.wrap=true;self.pending_wrap=false;self.cursor_visible=true;self.application_cursor=false;self.graphics=false;self.reply.fill(0);self.reply_len=0;}
    // ------------------------=
    // FUNC: resize
    // DESC: Preserves the visible grid while bounding the cursor and scroll region to measured cell dimensions.
    // ------------------=
    pub fn resize(&mut self,cols:usize,rows:usize){let cols=cols.clamp(1,WIDTH);let rows=rows.clamp(1,HEIGHT);if self.columns==cols&&self.rows==rows{return;}self.columns=cols;self.rows=rows;self.x=self.x.min(cols-1);self.y=self.y.min(rows-1);self.top=0;self.bottom=rows-1;self.pending_wrap=false;}
    // ------------------------=
    // FUNC: scroll
    // DESC: Scrolls only the active DEC region, preserving headers and alternate-screen state.
    // ------------------=
    fn scroll(&mut self,up:bool,n:usize){let blank=self.blank();let n=n.min(self.bottom-self.top+1);for _ in 0..n{if up {for y in self.top..self.bottom{self.cells[y]=self.cells[y+1];}self.cells[self.bottom].fill(blank);}else{for y in (self.top+1..=self.bottom).rev(){self.cells[y]=self.cells[y-1];}self.cells[self.top].fill(blank);}}}
    // ------------------------=
    // FUNC: blank
    // DESC: Uses the current rendition when erasing cells.
    // ------------------=
    fn blank(&self)->Cell{Cell{byte:b' ',..self.style}}
    // ------------------------=
    // FUNC: newline
    // DESC: Applies line feed independently of carriage return, as required by remote PTYs.
    // ------------------=
    fn newline(&mut self){self.pending_wrap=false;if self.y==self.bottom{self.scroll(true,1);}else{self.y=(self.y+1).min(self.rows-1);}}
    // ------------------------=
    // FUNC: feed
    // DESC: Incrementally parses fragmented control sequences, colors, cursor movement, scrolling and printable ASCII.
    // ------------------=
    pub fn feed(&mut self,bytes:&[u8]){for &b in bytes{
        if self.state==3{if b==7{self.state=0;}else if b==27{self.state=4;}continue;}
        if self.state==4{self.state=if b==b'\\'{0}else{3};continue;}
        if self.state==5{self.graphics=b==b'0';self.state=0;continue;}
        if self.state==1{self.state=0;match b{b'['=>{self.state=2;self.params.fill(0);self.param=0;self.private=false;},b']'|b'P'|b'^'|b'_'=>self.state=3,b'('|b')'=>self.state=5,b'7'=>self.saved_cursor=(self.x,self.y),b'8'=>{(self.x,self.y)=self.saved_cursor;self.x=self.x.min(self.columns-1);self.y=self.y.min(self.rows-1);},b'D'=>self.newline(),b'E'=>{self.x=0;self.newline();},b'M'=>{if self.y==self.top{self.scroll(false,1);}else{self.y=self.y.saturating_sub(1);}},b'c'=>self.reset(),_=>()}continue;}
        if self.state==2{match b{b'0'..=b'9'=>self.params[self.param]=(self.params[self.param]*10+(b-b'0')as usize).min(9999),b';'=>self.param=(self.param+1).min(15),b'?'=>self.private=true,0x40..=0x7e=>{self.state=0;self.csi(b);},27=>self.state=1,_=>()}continue;}
        match b {27=>self.state=1,b'\r'=>{self.x=0;self.pending_wrap=false;},b'\n'|11|12=>self.newline(),8=>{self.x=self.x.saturating_sub(1);self.pending_wrap=false;},9=>{self.x=((self.x/8+1)*8).min(self.columns-1);self.pending_wrap=false;},32..=126=>{
            if self.pending_wrap&&self.wrap{self.x=0;self.newline();}self.pending_wrap=false;
            let byte=if self.graphics{match b{b'q'=>b'-',b'x'=>b'|',b'j'|b'k'|b'l'|b'm'|b'n'|b't'|b'u'|b'v'|b'w'=>b'+',_=>b}}else{b};
            self.cells[self.y][self.x]=Cell{byte,..self.style};if self.x+1<self.columns{self.x+=1;}else{self.pending_wrap=true;}
        },_=>()}
    }}
    // ------------------------=
    // FUNC: csi
    // DESC: Executes bounded VT control operations and ignores unsupported commands without leaking escape text.
    // ------------------=
    fn csi(&mut self,b:u8){let n=self.params[0].max(1);let blank=self.blank();self.pending_wrap=false;match b{
        b'A'=>self.y=self.y.saturating_sub(n),b'B'|b'e'=>self.y=(self.y+n).min(self.rows-1),b'C'|b'a'=>self.x=(self.x+n).min(self.columns-1),b'D'=>self.x=self.x.saturating_sub(n),
        b'E'=>{self.x=0;self.y=(self.y+n).min(self.rows-1);},b'F'=>{self.x=0;self.y=self.y.saturating_sub(n);},b'G'|b'`'=>self.x=(n-1).min(self.columns-1),b'd'=>self.y=(n-1).min(self.rows-1),
        b'H'|b'f'=>{self.y=(n-1).min(self.rows-1);self.x=self.params[1].max(1).saturating_sub(1).min(self.columns-1);},
        b'J'=>match self.params[0]{0=>{self.cells[self.y][self.x..self.columns].fill(blank);for y in self.y+1..self.rows{self.cells[y].fill(blank);}},1=>{for y in 0..self.y{self.cells[y].fill(blank);}self.cells[self.y][..=self.x].fill(blank);},2|3=>{for row in &mut self.cells{row.fill(blank);}},_=>()},
        b'K'=>match self.params[0]{0=>self.cells[self.y][self.x..self.columns].fill(blank),1=>self.cells[self.y][..=self.x].fill(blank),2=>self.cells[self.y].fill(blank),_=>()},
        b'X'=>{let end=(self.x+n).min(self.columns);self.cells[self.y][self.x..end].fill(blank);},
        b'P'=>{let n=n.min(self.columns-self.x);self.cells[self.y].copy_within(self.x+n..self.columns,self.x);self.cells[self.y][self.columns-n..self.columns].fill(blank);},
        b'@'=>{let n=n.min(self.columns-self.x);self.cells[self.y].copy_within(self.x..self.columns-n,self.x+n);self.cells[self.y][self.x..self.x+n].fill(blank);},
        b'L'|b'M'=>{if self.y>=self.top&&self.y<=self.bottom{let top=self.top;self.top=self.y;self.scroll(b==b'M',n);self.top=top;}},
        b'S'=>self.scroll(true,n),b'T'=>self.scroll(false,n),
        b'r'=>{let bottom=self.params[1].checked_sub(1).unwrap_or(self.rows-1).min(self.rows-1);let top=(n-1).min(self.rows-1);if top<bottom{self.top=top;self.bottom=bottom;self.x=0;self.y=0;}},
        b's'=>self.saved_cursor=(self.x,self.y),b'u'=>{self.x=self.saved_cursor.0.min(self.columns-1);self.y=self.saved_cursor.1.min(self.rows-1);},
        b'h'|b'l' if self.private=>{for i in 0..=self.param{match self.params[i]{1=>self.application_cursor=b==b'h',7=>self.wrap=b==b'h',25=>self.cursor_visible=b==b'h',47|1047|1049=>self.alternate(b==b'h'),_=>()}}},
        b'm'=>{for i in 0..=self.param{match self.params[i]{0=>self.style=EMPTY,1=>self.style.bold=true,22=>self.style.bold=false,7=>self.style.inverse=true,27=>self.style.inverse=false,30..=37=>self.style.fg=(self.params[i]-30)as u8,40..=47=>self.style.bg=(self.params[i]-40)as u8,90..=97=>self.style.fg=(self.params[i]-90+8)as u8,100..=107=>self.style.bg=(self.params[i]-100+8)as u8,39=>self.style.fg=7,49=>self.style.bg=0,_=>()}}},
        b'n'=>{if self.params[0]==5{self.respond(b"\x1b[0n");}else if self.params[0]==6{let mut out=[0;32];out[..2].copy_from_slice(b"\x1b[");let mut at=2;at+=decimal(&mut out[at..],self.y+1);out[at]=b';';at+=1;at+=decimal(&mut out[at..],self.x+1);out[at]=b'R';self.respond(&out[..at+1]);}},
        b'c'=>self.respond(b"\x1b[?1;0c"),_=>()
    }}
    // ------------------------=
    // FUNC: alternate
    // DESC: Saves and restores the primary screen around interactive full-screen remote applications.
    // ------------------=
    fn alternate(&mut self,on:bool){if on==self.alternate{return;}for y in 0..HEIGHT{core::mem::swap(&mut self.cells[y],&mut self.saved[y]);}if on{self.main_cursor=(self.x,self.y);for row in &mut self.cells{row.fill(EMPTY);}self.x=0;self.y=0;}else{self.x=self.main_cursor.0.min(self.columns-1);self.y=self.main_cursor.1.min(self.rows-1);}self.alternate=on;}
    // ------------------------=
    // FUNC: respond
    // DESC: Queues bounded protocol replies separately from user keystrokes.
    // ------------------=
    fn respond(&mut self,b:&[u8]){if self.reply_len+b.len()<=self.reply.len(){self.reply[self.reply_len..self.reply_len+b.len()].copy_from_slice(b);self.reply_len+=b.len();}}
    // ------------------------=
    // FUNC: reply
    // DESC: Exposes terminal response bytes until encrypted transport acknowledges consumption.
    // ------------------=
    pub fn reply(&self)->&[u8]{&self.reply[..self.reply_len]}
    // ------------------------=
    // FUNC: consume_reply
    // DESC: Retains unsent response bytes under transport backpressure.
    // ------------------=
    pub fn consume_reply(&mut self,n:usize){let n=n.min(self.reply_len);self.reply.copy_within(n..self.reply_len,0);self.reply_len-=n;self.reply[self.reply_len..].fill(0);}
}
// ------------------------=
// FUNC: decimal
// DESC: Formats a bounded terminal coordinate without allocation.
// ------------------=
fn decimal(out:&mut[u8],mut n:usize)->usize{let mut digits=[0;20];let mut at=20;loop{at-=1;digits[at]=b'0'+(n%10)as u8;n/=10;if n==0{break;}}let len=20-at;out[..len].copy_from_slice(&digits[at..]);len}
static mut VT:Terminal=Terminal::new();
// ------------------------=
// FUNC: mutate
// DESC: Serializes remote screen updates on the UI thread.
// ------------------=
pub fn mutate<R>(f:impl FnOnce(&mut Terminal)->R)->R{unsafe{f(&mut *core::ptr::addr_of_mut!(VT))}}
// ------------------------=
// FUNC: inspect
// DESC: Borrows the remote screen without a large stack copy.
// ------------------=
pub fn inspect<R>(f:impl FnOnce(&Terminal)->R)->R{unsafe{f(&*core::ptr::addr_of!(VT))}}
