#![allow(dead_code)]
#[path="../kernel/ui/geometry.rs"]pub mod geometry;
#[path="../kernel/ui/editor_preferences.rs"]pub mod preferences;
#[path="../kernel/ui/command_terminal.rs"]pub mod command_terminal;
#[path="../kernel/ui/command_terminal_paint.rs"]pub mod command_terminal_paint;
#[path="../kernel/ui/terminal_vt.rs"]pub mod terminal_vt;
pub mod editor_tools {pub use crate::preferences;}
mod ui {pub use crate::{geometry,command_terminal,command_terminal_paint,terminal_vt};}

use ui::{command_terminal::{State,Position},geometry::Rect,terminal_vt::Terminal};
// ------------------------=
// FUNC: main
// DESC: Exercises terminal layout, persistence, scrollback, privacy, fragmented ANSI and full-screen screen restoration.
// ------------------=
fn main(){
    let mut s=Box::new(State::new());
    assert_eq!(s.appearance.font,2);
    let mut font_pixels=std::collections::HashSet::new();
    for font in 0..4 {
        let p=preferences::Preferences{font,..s.appearance};
        let mut pixels=Vec::new();
        for glyph in b"ASCII +-|[] ANSI 0123456789" {
            ui::command_terminal_paint::raster_glyph(p,*glyph,1,|x,y,a|pixels.push((x,y,a)));
        }
        assert!(!pixels.is_empty());font_pixels.insert(pixels);
    }
    assert_eq!(font_pixels.len(),4);
    s.session([1;16],true);s.publish(b"/home/alice",b"status",6);
    let body=Rect{x:0,y:40,width:1000,height:700};let toolbar=Rect{x:0,y:0,width:1000,height:40};
    let g=s.layout(body,toolbar,1);assert!(g.rows()>20);s.reflow(g.columns(),g.rows());
    for _ in 0..100{s.line(b"long retained output");}assert_eq!(s.count,100);assert!(s.max_scroll()>0);
    s.scroll_by(20);let anchor=s.visual(s.top()).unwrap();s.reflow(60,20);assert_eq!(s.visual(s.top()).unwrap().row,anchor.row);
    s.row_override=18;s.column_override=65;s.opacity=190;let b=s.settings();let mut other=Box::new(State::new());assert!(other.restore_settings(&b));assert_eq!(other.settings(),b);
    let g=s.layout(body,toolbar,1);assert_eq!((g.columns(),g.rows()),(65,18));let mut bad=b;bad[8]=255;assert!(!other.restore_settings(&bad));assert_eq!(other.settings(),b);
    s.session([2;16],true);assert_eq!(s.count,0);assert_eq!(s.input_length,0);assert_eq!(s.opacity,255);
    s.append(b"Hello\rJ\n");assert_eq!(s.byte(Position{row:s.first,column:0}),b'J');
    let mut t=Box::new(Terminal::new());t.resize(10,5);t.feed(b"hello\r\nworld");assert_eq!((t.x,t.y),(5,1));
    t.feed(b"\x1b[");t.feed(b"2;3H\x1b[31mX");assert_eq!(t.cells[1][2].byte,b'X');assert_eq!(t.cells[1][2].fg,1);
    t.feed(b"\x1b[?1049hTEMP\x1b[?1049l");assert_eq!(t.cells[1][2].byte,b'X');assert_eq!((t.x,t.y),(3,1));
    t.feed(b"\x1b[6n");assert_eq!(t.reply(),b"\x1b[2;4R");t.consume_reply(2);assert_eq!(t.reply(),b"2;4R");
    t.feed(b"\x1b[2J\x1b[H1234567890Z");assert_eq!(t.cells[1][0].byte,b'Z');
    t.feed(b"\x1b[?1h");assert!(t.application_cursor);t.reset();assert!(t.cells.iter().flatten().all(|c|c.byte==b' '));assert!(t.reply().is_empty());
    // Every possible fragmented byte must remain bounded, including malformed CSI parameters.
    for b in 0..=255{for _ in 0..32{t.feed(&[27,b'[',b]);}assert!(t.x<t.columns&&t.y<t.rows);}
    render();
    println!("terminal state, grid, settings, privacy, production pixels and VT behavior passed");
}

struct Image {pixels:Vec<u8>,width:usize,height:usize}
impl ui::command_terminal_paint::Canvas for Image {
    // ------------------------=
    // FUNC: fill
    // DESC: Records production terminal fills in a bounded test framebuffer.
    // ------------------=
    fn fill(&mut self,r:Rect,c:(u8,u8,u8)){for y in r.y.max(0)as usize..(r.bottom().max(0)as usize).min(self.height){for x in r.x.max(0)as usize..(r.right().max(0)as usize).min(self.width){let i=(y*self.width+x)*3;self.pixels[i..i+3].copy_from_slice(&[c.0,c.1,c.2]);}}}
    // ------------------------=
    // FUNC: glyph
    // DESC: Uses the production font rasterizer, not a substitute host font.
    // ------------------=
    fn glyph(&mut self,x:i32,y:i32,b:u8,scale:usize,p:preferences::Preferences,c:(u8,u8,u8)){
        ui::command_terminal_paint::raster_glyph(p,b,scale,|dx,dy,a|{let x=x+dx as i32;let y=y+dy as i32;if x<0||y<0||x as usize>=self.width||y as usize>=self.height{return;}let i=(y as usize*self.width+x as usize)*3;for(k,v)in [c.0,c.1,c.2].iter().enumerate(){self.pixels[i+k]=((self.pixels[i+k]as u32*(255-a as u32)+*v as u32*a as u32)/255)as u8;}});
    }
}
// ------------------------=
// FUNC: render
// DESC: Captures production terminal pixels at normal and HiDPI sizes and verifies actual text fills more than six rows.
// ------------------=
fn render(){
    use std::io::Write;
    for scale in [1,2]{let mut s=State::new();s.session([1;16],true);for n in 0..22{s.line(format!("{:02}  Native Command Window | ASCII: +-----+  [] {{}} /home/alice",n+1).as_bytes());}s.publish(b"/home/alice",b"ssh fixture@remote",18);
        let body=Rect{x:0,y:0,width:1000*scale,height:700*scale};let g=s.layout(body,Rect::default(),scale as usize);s.reflow(g.columns(),g.rows());
        let mut image=Image{pixels:vec![0;body.width as usize*body.height as usize*3],width:body.width as usize,height:body.height as usize};ui::command_terminal_paint::content(&mut image,&s,g,true);
        let bg=s.appearance.color(0);let mut ink_rows=0;
        for row in 0..g.rows(){let y=g.text.y as usize+row*g.line;let ink=(y..y+g.line).any(|y|(g.text.x as usize..g.text.right()as usize).any(|x|{let i=(y*image.width+x)*3;image.pixels[i..i+3]!=[bg.0,bg.1,bg.2]}));if ink{ink_rows+=1;}}
        assert!(ink_rows>6);let path=format!("build/command-terminal-{scale}x.ppm");let mut file=std::fs::File::create(path).unwrap();write!(file,"P6\n{} {}\n255\n",image.width,image.height).unwrap();file.write_all(&image.pixels).unwrap();
    }
}
