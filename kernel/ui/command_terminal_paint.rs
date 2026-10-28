//! Shared production terminal cell painter, also exercised by host framebuffer tests.
use super::{command_terminal::{State,Layout,Position},geometry::Rect,editor_tools::preferences::Preferences};
pub trait Canvas {
    // ------------------------=
    // FUNC: fill
    // DESC: Fills a clipped terminal rectangle.
    // ------------------=
    fn fill(&mut self,r:Rect,color:(u8,u8,u8));
    // ------------------------=
    // FUNC: glyph
    // DESC: Paints one packaged antialiased monospace glyph.
    // ------------------=
    fn glyph(&mut self,x:i32,y:i32,b:u8,scale:usize,p:Preferences,color:(u8,u8,u8));
}
// ------------------------=
// FUNC: raster_glyph
// DESC: Samples exactly the selected packaged font face and size, without host font dependencies.
// ------------------=
pub fn raster_glyph(p:Preferences,b:u8,scale:usize,mut pixel:impl FnMut(usize,usize,u8)) {
    if !(32..=126).contains(&b){return;}
    let record=p.record();let start=(b as usize-32)*24;
    for y in 0..record[1]as usize*scale {for x in 0..record[3]as usize*scale {
        let a=record[4+(y/scale)*24*95+start+x/scale];if a!=0 {pixel(x,y,a);}
    }}
}
// ------------------------=
// FUNC: content
// DESC: Paints soft-wrapped output, selections, search hits and the live inline prompt on one continuous grid.
// ------------------=
pub fn content(canvas:&mut impl Canvas,s:&State,g:Layout,caret:bool) {
    let p=s.appearance;canvas.fill(g.body,p.color(0));
    for visual in 0..g.rows() {
        let Some(start)=s.visual(s.top()+visual)else{break;};
        let y=g.text.y+(visual*g.line)as i32;
        for cell in 0..g.columns() {
            let at=Position{row:start.row,column:start.column+cell};
            if at.column>s.length(at.row){break;}
            let x=g.text.x+(cell*g.cell)as i32;
            let rect=Rect{x,y,width:g.cell as u32,height:g.line as u32};
            let found=s.search&&s.found.is_some_and(|f|f.row==at.row&&at.column>=f.column&&at.column<f.column+s.query_length);
            if s.selected(at){canvas.fill(rect,p.color(3));}
            else if found{canvas.fill(rect,p.color(2));}
            let color=if found{p.color(7)}else if at.row==s.end()&&at.column<s.prompt_length {p.color(6)}else{p.color(4)};
            if at.column<s.length(at.row){canvas.glyph(x,y+2*g.scale as i32,s.byte(at),g.scale,p,color);}
            if caret&&!s.search&&at.row==s.end()&&at.column==s.prompt_length+s.cursor {
                canvas.fill(Rect{x,y:y+2*g.scale as i32,width:2*g.scale as u32,height:(g.line-4*g.scale)as u32},p.color(5));
            }
        }
    }
    if s.max_scroll()>0 {canvas.fill(g.track,p.color(1));canvas.fill(g.thumb(s),p.color(8));}
}
