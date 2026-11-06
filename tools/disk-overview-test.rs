#![allow(dead_code)]
#[path="../kernel/ui/scroll_capture.rs"]pub mod scroll_capture;
#[path="../kernel/ui/geometry.rs"]pub mod geometry;
#[path="../kernel/ui/editor_preferences.rs"]pub mod preferences;
#[path="../kernel/ui/command_terminal.rs"]pub mod command_terminal;
#[path="../kernel/ui/command_terminal_paint.rs"]pub mod command_terminal_paint;
#[path="../kernel/ui/terminal_vt.rs"]pub mod terminal_vt;
pub mod editor_tools {pub use crate::preferences;}
mod ui {pub use crate::command_terminal;}
use command_terminal::disk_view::{DiskView,LIMIT};
use command_terminal_paint::{Canvas,disk};
use geometry::Rect;
use std::io::Write;
struct Image {w:usize,h:usize,pixels:Vec<u8>,cells:std::collections::HashSet<(i32,i32)>}
impl Canvas for Image {
    // ------------------------=
    // FUNC: fill
    // DESC: Clears the production terminal canvas rectangle.
    // ------------------=
    fn fill(&mut self,r:Rect,c:(u8,u8,u8)) {
        assert!(r.x>=0&&r.y>=0&&r.right()<=self.w as i32&&r.bottom()<=self.h as i32);
        for y in r.y..r.bottom(){for x in r.x..r.right(){let at=(y as usize*self.w+x as usize)*3;self.pixels[at..at+3].copy_from_slice(&[c.0,c.1,c.2]);}}
    }
    // ------------------------=
    // FUNC: glyph
    // DESC: Asserts cell exclusivity and raster bounds while using packaged production fonts.
    // ------------------=
    fn glyph(&mut self,x:i32,y:i32,b:u8,scale:usize,p:preferences::Preferences,c:(u8,u8,u8)) {
        assert!(self.cells.insert((x,y)),"Overlapping cell at {x},{y}");
        command_terminal_paint::raster_glyph(p,b,scale,|dx,dy,a|{
            let px=x+dx as i32;let py=y+dy as i32;
            assert!(px>=0&&py>=0&&px<self.w as i32&&py<self.h as i32);
            let at=(py as usize*self.w+px as usize)*3;
            for (k,v) in [c.0,c.1,c.2].iter().enumerate(){self.pixels[at+k]=((self.pixels[at+k]as u32*(255-a as u32)+*v as u32*a as u32)/255)as u8;}
        });
    }
}
// ------------------------=
// FUNC: render
// DESC: Captures the actual disk painter at user-sized grids with overlap and framebuffer checks.
// ------------------=
fn render(s:&mut command_terminal::State,columns:usize,rows:usize,name:&str) {
    s.column_override=columns;s.row_override=rows;
    let metrics=s.appearance.record();let cell=metrics[3]as usize;let line=metrics[1]as usize+4;
    let w=columns*cell+48;let h=rows*line+80;
    let body=Rect{x:0,y:0,width:w as u32,height:h as u32};
    let g=s.layout(body,Rect{x:0,y:0,width:w as u32,height:0},1);
    let mut image=Image{w,h,pixels:vec![0;w*h*3],cells:Default::default()};
    disk::paint(&mut image,g,s);
    assert!(!image.cells.is_empty());
    if columns>=64&&rows>=13 {
        let chart_x=g.text.x+((g.columns()*3/5+2)*g.cell)as i32;
        assert!(image.cells.iter().any(|&(x,y)|x>chart_x&&y>g.text.y+(4*g.line)as i32));
    }
    let path=format!("build/disk-{name}-{columns}x{rows}.ppm");
    let mut file=std::fs::File::create(path).unwrap();write!(file,"P6\n{w} {h}\n255\n").unwrap();file.write_all(&image.pixels).unwrap();
}
// ------------------------=
// FUNC: main
// DESC: Exercises aggregation, parent transitions, bounded scans, sector boundaries and rendering.
// ------------------=
fn main(){
    let mut v=DiskView::new();v.capacity=8192;
    v.push(b"second",1024,8,[0;16],true);v.push(b"first",512,2,[0;16],false);
    for (at,want) in [(1023,None),(1024,Some(1)),(1535,Some(1)),(1536,None),(4096,Some(0)),(5120,None)]{assert_eq!(v.partition_at(at),want);}
    v.selected=0;assert!(v.enter());assert_eq!(v.depth,1);assert_eq!(v.partition_at(4096),Some(0));
    v.begin_listing();
    v.add_path(b"/home/Alice/code/main.c",300,[1;16],false);
    v.add_path(b"/home/Alice/music/theme.wav",700,[2;16],false);
    v.add_path(b"/System",0,[0;16],true);v.add_path(b"/System/grep.py",100,[3;16],false);
    assert_eq!((v.count,v.total),(2,1100));assert_eq!(v.rows[0].bytes,1000);assert_eq!(v.rows[1].bytes,100);
    assert!(v.rows[0].directory&&v.rows[1].directory);
    assert_eq!(v.sector(0),Some(0));assert_eq!(v.sector(65535),Some(1));
    assert!(v.enter());assert_eq!(&v.path[..v.path_len],b"/home/");
    v.begin_listing();v.add_path(b"/home/Alice/code/main.c",300,[1;16],false);
    v.add_path(b"/homepage/outside",999,[9;16],false);assert_eq!(v.total,300);
    assert!(v.enter());assert_eq!(&v.path[..v.path_len],b"/home/Alice/");
    assert!(v.back());assert_eq!(&v.path[..v.path_len],b"/home/");assert!(v.back());assert_eq!(v.path_len,1);
    v.begin_listing();v.add_path(b"/empty",0,[0;16],true);assert_eq!(v.total,0);assert_eq!(v.sector(0),None);
    assert!(v.enter());v.begin_listing();assert_eq!(v.count,0);assert!(v.back());
    v.begin_listing();v.add_path(b"/hello.c",42,[8;16],false);assert!(!v.enter());assert!(v.inspect);
    assert!(!v.back());assert!(!v.inspect);assert!(v.back());assert_eq!(v.depth,0);
    v.depth=1;v.begin_listing();
    for i in 0..LIMIT+5 {v.add_path(format!("/file-{i}").as_bytes(),10,[0;16],false);}
    assert_eq!((v.count,v.total,v.omitted),(LIMIT,(LIMIT+5)as u64*10,50));assert!(v.incomplete);
    assert_eq!(v.sector(65535),None);
    for (x,y,want) in [(0,-1,0),(1,0,16384),(0,1,32768),(-1,0,49152)]{assert_eq!(disk::angle(x,y),want);}
    let mut s=Box::new(command_terminal::State::new());s.disk.active=true;s.disk.capacity=16*1024*1024*1024;
    s.disk.used=420*1024*1024;s.disk.pool_capacity=2*1024*1024*1024;
    s.disk.push(b"Infinity EFI",5*1024*1024*1024,2048,[0;16],false);
    s.disk.push(b"Infinity Container",10*1024*1024*1024,10_500_000,[0;16],true);
    for (c,r) in [(86,15),(120,30),(48,12),(24,6),(10,8)]{render(&mut s,c,r,"overview");}
    s.disk.selected=1;s.disk.enter();s.disk.begin_listing();
    for (path,bytes) in [("/System/runtime/python",850_000_000),("/System/fonts/inter.ttf",35_000_000),
        ("/home/Jonathan/Documents/notes.txt",2_000_000),("/home/Jonathan/Media/demo.mp4",150_000_000),
        ("/Applications/browser",400_000_000),("/Recovery/checkpoint",90_000_000),("/Projects/calculator/main.c",12_000_000)]{
        s.disk.add_path(path.as_bytes(),bytes,[0;16],false);
    }
    s.disk.rows[..s.disk.count].sort_unstable_by(|a,b|b.bytes.cmp(&a.bytes));
    for (c,r) in [(86,15),(120,30),(48,12)]{render(&mut s,c,r,"directories");}
}
