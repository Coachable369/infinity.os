//! ANSI usage wheel drawn on the shared production monospace grid.
use core::fmt::{self, Write};
use super::{Canvas, Preferences};
use crate::ui::command_terminal::{State, Layout};
const COLORS: [(u8,u8,u8);8] = [(41,184,219),(35,209,139),(214,112,214),(245,205,67),
    (59,142,234),(241,108,76),(143,203,98),(166,148,239)];
const MUTED:(u8,u8,u8)=(137,148,164);
struct Text { data:[u8;192], len:usize }
impl Text {
    // ------------------------=
    // FUNC: new
    // DESC: Creates bounded stack text for one terminal label.
    // ------------------=
    fn new()->Self {Self{data:[0;192],len:0}}
    // ------------------------=
    // FUNC: push
    // DESC: Appends bounded path bytes without allocation.
    // ------------------=
    fn push(&mut self,b:&[u8]) {let n=b.len().min(self.data.len()-self.len);self.data[self.len..self.len+n].copy_from_slice(&b[..n]);self.len+=n;}
    // ------------------------=
    // FUNC: bytes
    // DESC: Returns initialized label bytes.
    // ------------------=
    fn bytes(&self)->&[u8] {&self.data[..self.len]}
}
impl fmt::Write for Text {
    // ------------------------=
    // FUNC: write_str
    // DESC: Formats numeric metadata into bounded terminal text.
    // ------------------=
    fn write_str(&mut self,s:&str)->fmt::Result {self.push(s.as_bytes());Ok(())}
}
// ------------------------=
// FUNC: size
// DESC: Formats binary logical sizes without rounding small files down to zero.
// ------------------=
fn size(bytes:u64)->Text {
    let mut t=Text::new();let units=["B","KiB","MiB","GiB","TiB","PiB","EiB"];
    let mut divisor=1u64;let mut unit=0;
    while unit<6 && bytes/divisor>=1024 {divisor*=1024;unit+=1;}
    if unit==0 {let _=write!(t,"{} B",bytes);}else{let _=write!(t,"{}.{} {}",bytes/divisor,u128::from(bytes%divisor)*10/u128::from(divisor),units[unit]);}
    t
}
// ------------------------=
// FUNC: label
// DESC: Draws one clipped row segment with the user's actual terminal font metrics.
// ------------------=
fn label(c:&mut impl Canvas,g:Layout,p:Preferences,x:usize,y:usize,width:usize,b:&[u8],color:(u8,u8,u8)) {
    if y>=g.rows() || x>=g.columns(){return;}
    for (i,&ch) in b.iter().take(width.min(g.columns()-x)).enumerate(){
        c.glyph(g.text.x+((x+i)*g.cell)as i32,g.text.y+(y*g.line+2*g.scale)as i32,
            if ch.is_ascii_graphic()||ch==b' ' {ch}else{b'?'},g.scale,p,color);
    }
}
// ------------------------=
// FUNC: angle
// DESC: Maps a vector clockwise from twelve o'clock into bounded octant-interpolated turns.
// ------------------=
pub fn angle(x:i64,y:i64)->u32 {
    let ax=x.unsigned_abs();let ay=y.unsigned_abs();
    if ax==0 && ay==0{return 0;}
    let oct=if ay>=ax {ax*8192/ay.max(1)}else{16384-ay*8192/ax.max(1)} as u32;
    match (x>=0,y<0){(true,true)=>oct,(true,false)=>32768-oct,(false,false)=>32768+oct,(false,true)=>(65536-oct)%65536}
}
// ------------------------=
// FUNC: paint
// DESC: Paints physical partition bars and color-linked logical-size wheel without storage IO.
// ------------------=
pub fn paint(c:&mut impl Canvas,g:Layout,s:&State) {
    let v=&s.disk;let p=s.appearance;let cols=g.columns();let rows=g.rows();
    c.fill(g.body,s.background_color());
    let mut title=Text::new();title.push(b"DISK  /  ");
    if v.depth==0 {title.push(b"System disk");}else{title.push(v.partition.label());title.push(b"  ");title.push(&v.path[..v.path_len]);}
    label(c,g,p,0,0,cols,title.bytes(),COLORS[0]);
    if rows<8 || cols<24 {
        let visible=rows.saturating_sub(2);let base=v.selected/visible.max(1)*visible.max(1);
        for i in base..v.count.min(base+visible){label(c,g,p,0,i-base+1,cols,v.rows[i].label(),COLORS[i%8]);}
        if rows>1{label(c,g,p,0,rows-1,cols,b"Arrows / Enter / Left / Q",MUTED);}return;
    }
    let parts=if v.depth==0 {&v.rows[..v.count]}else{&v.partitions[..v.partition_count]};
    label(c,g,p,0,1,1,b"[",MUTED);
    for x in 1..cols.saturating_sub(1){
        let byte=u128::from(v.capacity)*(x-1)as u128/cols.saturating_sub(2).max(1)as u128;
        let index=v.partition_at(byte);
        let selected=index.is_some_and(|i|if v.depth==0{i==v.selected}else{parts[i].first==v.partition.first});
        label(c,g,p,x,1,1,if selected{b"#"}else if index.is_some(){b"="}else{b"."},index.map_or(MUTED,|i|COLORS[i%8]));
    }
    if cols>1{label(c,g,p,cols-1,1,1,b"]",MUTED);}
    let mut usage=Text::new();usage.push(b"Pool allocation: ");
    if v.pool_capacity>0 {usage.push(size(v.used).bytes());usage.push(b" / ");usage.push(size(v.pool_capacity).bytes());}
    else {usage.push(b"unavailable");}
    label(c,g,p,0,2,cols,usage.bytes(),MUTED);
    let start=4;let height=rows-6;
    let chart=cols>=64 && height>=7 && !v.inspect;
    let left=if chart {cols*3/5}else{cols};
    let path_width=left.saturating_sub(18).max(1);
    label(c,g,p,0,3,path_width,b"PATH",MUTED);
    label(c,g,p,path_width,3,left.saturating_sub(path_width),b"      SIZE   SHARE",MUTED);
    if v.inspect && v.selected<v.count {
        let r=&v.rows[v.selected];let lines=r.label().len().div_ceil(cols.max(1));
        for (n,line) in r.label().chunks(cols.max(1)).take(height).enumerate(){label(c,g,p,0,start+n,cols,line,COLORS[0]);}
        if lines<height {let mut info=Text::new();info.push(b"Logical size: ");info.push(size(r.bytes).bytes());
            label(c,g,p,0,start+lines,cols,info.bytes(),p.color(4));}
    }else{
        let base=v.selected/height*height;
        let total=if v.depth==0{v.capacity}else{v.total};
        for i in base..v.count.min(base+height){
            let r=&v.rows[i];let y=start+i-base;let color=COLORS[i%8];
            label(c,g,p,0,y,2,if i==v.selected{b"> "}else{b"  "},color);
            let width=path_width.saturating_sub(3);let name=r.label();
            if name.len()>width && width>3 {
                label(c,g,p,2,y,3,b"...",color);
                label(c,g,p,5,y,width-3,&name[name.len()-(width-3)..],color);
            }else{label(c,g,p,2,y,width,name,color);}
            let mut info=Text::new();let sz=size(r.bytes);
            let _=write!(info,"{:>9} {:>4}%",core::str::from_utf8(sz.bytes()).unwrap_or("?"),u128::from(r.bytes)*100/u128::from(total.max(1)));
            label(c,g,p,path_width,y,left.saturating_sub(path_width),info.bytes(),color);
        }
        if v.count==0 {label(c,g,p,0,start,left,b"No accessible entries in this directory.",MUTED);}
    }
    if chart {
        let x0=left+2;let width=cols-x0;let radius=(width*g.cell).min(height*g.line)as i64/2;
        let cx=(width*g.cell)as i64/2;let cy=(height*g.line)as i64/2;let r2=radius*radius;
        for y in 0..height {for x in 0..width {
            let dx=(x*g.cell+g.cell/2)as i64-cx;let dy=(y*g.line+g.line/2)as i64-cy;let d=dx*dx+dy*dy;
            if d>r2 || d*100<r2*16 {continue;}
            let item=v.sector(angle(dx,dy));
            let mark=if item==Some(v.selected){b"#".as_slice()}else if item.is_some(){b":".as_slice()}else{b".".as_slice()};
            label(c,g,p,x0+x,start+y,1,mark,item.map_or(MUTED,|i|COLORS[i%8]));
        }}
        let total=size(if v.depth==0{v.capacity}else{v.total});
        let hole=(radius*7/10/g.cell as i64).max(0)as usize;
        if total.len<=hole {label(c,g,p,x0+(width-total.len)/2,start+height/2,total.len,total.bytes(),p.color(4));}
        label(c,g,p,x0,3,width,if v.depth==0{b"PARTITION CAPACITY"}else{b"LOGICAL BYTES"},MUTED);
    }
    let mut status=Text::new();
    if v.confirm {status.push(if v.rows.get(v.selected).is_some_and(|r|r.label().starts_with(b"/trash/")){b"Permanently delete? Y confirms; any other key cancels."}else{b"Move to Trash? Y confirms; any other key cancels."});}
    else if !v.message.is_empty(){status.push(v.message);}
    else if v.incomplete {status.push(b"Partial listing: some entries could not be included.");}
    else if v.selected<v.count {status.push(v.rows[v.selected].label());}
    label(c,g,p,0,rows-2,cols,status.bytes(),if v.confirm||v.incomplete{COLORS[3]}else{MUTED});
    label(c,g,p,0,rows-1,cols,b"Up/Down select  Enter open  Left back  D clean  U restore  R refresh  Q quit",MUTED);
}
