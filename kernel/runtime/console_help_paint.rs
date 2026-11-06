//! Shared production Help painter, also exercised by the host pixel harness.
use crate::ui::{command_terminal::{Layout,State},command_terminal_paint::Canvas,editor_tools::preferences::Preferences};
// ------------------------=
// FUNC: paint
// DESC: Renders the searchable catalog or a scrollable manual on the terminal font grid.
// ------------------=
pub fn paint(canvas:&mut impl Canvas,g:Layout,s:&State) {
    use crate::runtime::console_help as help;
    use crate::runtime::console_execution::{Bytes, OUTPUT_BYTES};
    canvas.fill(g.body,s.background_color());
    let query = &s.help_query[..s.help_query_length];
    let selected = help::filtered_entry(query, s.help_selected);
    if s.help_detail {
        ascii(canvas,g,s.appearance,0,b"+-- COMMAND MANUAL ------------------------------------------+");
        let mut page = Bytes::<OUTPUT_BYTES>::new();
        if let Some(index) = selected { help::manual(index, &mut page); }
        let columns = g.columns().max(1);
        let lines = help::manual_lines(page.bytes(),columns);
        let visible = g.rows().saturating_sub(2);
        let offset = s.help_scroll.min(lines.clone().count().saturating_sub(visible));
        for (row, line) in lines.skip(offset).take(visible).enumerate() {
            ascii(canvas,g,s.appearance,row+1,line);
        }
        ascii(canvas,g,s.appearance,g.rows().saturating_sub(1),b"Up/Down: scroll  Enter/Tab: insert command  Esc: list");
    } else {
        let mut heading = Bytes::<128>::new();
        heading.write(b"Search: "); heading.write(query); heading.write(b"_");
        ascii(canvas,g,s.appearance,0,heading.bytes());
        ascii(canvas,g,s.appearance,1,b"+-- COMMAND -------------------------------------------------+");
        let visible = g.rows().saturating_sub(3).max(1);
        let start = s.help_selected / visible * visible;
        for (row,index) in (0..help::catalog_count()).filter(|index| help::catalog_matches(*index,query)).skip(start).take(visible).enumerate() {
            let mut name = Bytes::<128>::new(); help::catalog_name(index,&mut name);
            let mut line = Bytes::<160>::new();
            line.write(if Some(index)==selected { b"> " } else { b"  " }); line.write(name.bytes());
            colored(canvas,g,s.appearance,row+2,line.bytes(),if Some(index)==selected {(17,168,205)}else{s.appearance.color(4)});
        }
        if selected.is_none() { ascii(canvas,g,s.appearance,2,b"No matching commands. Backspace edits your search."); }
        ascii(canvas,g,s.appearance,g.rows().saturating_sub(1),b"Type: search  Arrows: select  Enter: manual  Tab: insert  Esc: exit");
    }
}
// ------------------------=
// FUNC: ascii
// DESC: Draws a bounded manual row with the user's terminal text color.
// ------------------=
fn ascii(canvas:&mut impl Canvas,g:Layout,p:Preferences,row:usize,bytes:&[u8]) { colored(canvas,g,p,row,bytes,p.color(4)); }
// ------------------------=
// FUNC: colored
// DESC: Clips catalog text to the terminal row and column bounds.
// ------------------=
fn colored(canvas:&mut impl Canvas,g:Layout,p:Preferences,row:usize,bytes:&[u8],color:(u8,u8,u8)) {
    if row>=g.rows(){return;}for(i,&b)in bytes.iter().take(g.columns()).enumerate(){canvas.glyph(g.text.x+(i*g.cell)as i32,g.text.y+(row*g.line)as i32,b,g.scale,p,color);}
}
