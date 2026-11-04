//! Terminal editor uses the existing editor typography and compositor-owned Command Window.
use super::DisplayDevice;
use crate::ui::{terminal_edit as edit,text_editor,geometry::Rect,editor_tools::preferences};
impl DisplayDevice {
    // ------------------------=
    // FUNC: terminal_edit_view
    // DESC: Paints a responsive wrapped text grid, selection, caret, file status and keyboard controls in the existing native terminal panel.
    // ------------------=
    pub(super) fn terminal_edit_view(&mut self,area:Rect,scale:usize)->bool {
        if !edit::inspect(|s|s.active){return false;}
        let p=preferences::current();let cell=p.cell()*scale;let line=p.line()*scale;
        let columns=(area.width as usize).saturating_sub(36*scale)/cell.max(1);
        let rows=(area.height as usize).saturating_sub(4*line+24*scale)/line.max(1);
        let columns=columns.max(1);let rows=rows.max(1);
        edit::mutate(|s|{let cursor=text_editor::visual_cursor_row(s.document.bytes(),columns,s.document.cursor());if cursor<s.top{s.top=cursor;}else if cursor>=s.top+rows{s.top=cursor+1-rows;}});
        edit::inspect(|s|{
            let left=area.x.max(0)as usize;let top=area.y.max(0)as usize;
            let bg=p.color(0);self.fill_rect(left,top,area.width as usize,area.height as usize,bg.0,bg.1,bg.2);
            let mut header=[0u8;128];let prefix=if s.document.is_saved(){b"edit  ".as_slice()}else{b"edit * ".as_slice()};
            header[..prefix.len()].copy_from_slice(prefix);header[prefix.len()..prefix.len()+s.path_len].copy_from_slice(&s.path[..s.path_len]);
            self.app_label(Rect{x:area.x+18*scale as i32,y:area.y,width:area.width.saturating_sub(36*scale as u32),height:(line+12*scale)as u32},&header[..prefix.len()+s.path_len],p.color(6),false,scale);
            let y0=top+line+12*scale;let bytes=s.document.bytes();let cursor=s.document.cursor();let selection=s.document.selection();
            let first=text_editor::visual_line_start(bytes,columns,s.top);let mut row=0;let mut col=0;
            for at in first..=bytes.len(){
                if row>=rows{break;}
                let x=left+18*scale+col*cell;let y=y0+row*line;
                if selection.is_some_and(|(a,b)|at>=a&&at<b){let c=p.color(3);self.fill_rect(x,y,cell,line,c.0,c.1,c.2);}
                if at==cursor&&s.prompt==0{let c=p.color(5);self.fill_rect(x,y,2*scale,line,c.0,c.1,c.2);}
                let Some(&b)=bytes.get(at)else{break;};
                if b==b'\n'{row+=1;col=0;continue;}
                self.editor_glyph(x,y,b,scale,p.color(4));col+=1;if col==columns{row+=1;col=0;}
            }
            let foot=Rect{x:area.x+18*scale as i32,y:area.bottom()-(2*line+12*scale)as i32,width:area.width.saturating_sub(36*scale as u32),height:line as u32};
            if s.prompt==2||s.prompt==3 {
                let mut field=[0;128];let label=if s.prompt==2{b"Find: ".as_slice()}else{b"Go to line: ".as_slice()};field[..label.len()].copy_from_slice(label);field[label.len()..label.len()+s.query_len].copy_from_slice(&s.query[..s.query_len]);
                self.app_label(foot,&field[..label.len()+s.query_len],p.color(6),false,scale);
            }else{self.app_label(foot,s.notice,p.color(6),false,scale);}
            self.app_label(Rect{y:foot.y+line as i32,..foot},b"^S Save  ^X Exit  ^W Find  ^G Line  ^K Cut line  ^U Paste  ^Z Undo",p.color(8),false,scale);
        });true
    }
}
