//! Native standalone terminal presentation inside compositor-owned Infinity chrome.
use super::DisplayDevice;
use crate::ui::{command_terminal::{self as terminal,Layout,Menu},command_terminal_paint::{self,Canvas},geometry::Rect,editor_tools::preferences::{self,Preferences}};
impl Canvas for DisplayDevice {
    // ------------------------=
    // FUNC: fill
    // DESC: Adapts terminal bands to the existing damage-clipped framebuffer fill.
    // ------------------=
    fn fill(&mut self,r:Rect,c:(u8,u8,u8)) {self.fill_rect(r.x.max(0)as usize,r.y.max(0)as usize,r.width as usize,r.height as usize,c.0,c.1,c.2);}
    // ------------------------=
    // FUNC: glyph
    // DESC: Blends packaged monospace glyphs into the current clipped production surface.
    // ------------------=
    fn glyph(&mut self,x:i32,y:i32,b:u8,scale:usize,p:Preferences,c:(u8,u8,u8)) {
        let Some(region)=self.clipped_render_region(x.max(0)as usize,y.max(0)as usize,p.cell()*scale,p.record()[1]as usize*scale)else{return;};
        command_terminal_paint::raster_glyph(p,b,scale,|dx,dy,a|{
            let px=x+dx as i32;let py=y+dy as i32;
            if px>=region.left as i32&&px<region.right as i32&&py>=region.top as i32&&py<region.bottom as i32 {
                self.blend_color_unchecked(px as usize,py as usize,c.0,c.1,c.2,a);
            }
        });self.mark_dirty_rect(region.left,region.top,region.right-region.left,region.bottom-region.top);
    }
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: command_terminal_view
    // DESC: Paints the integrated terminal, actionable toolbar, inline search and shared Precision menus.
    // ------------------=
    pub(super) fn command_terminal_view(&mut self,body:Rect,toolbar:Rect,scale:usize) {
        terminal::mutate(|s|{let g=Layout::new(body,toolbar,scale,s.appearance,s.search);s.reflow(g.columns(),g.rows());});
        terminal::inspect(|s|{
            let p=s.appearance;let g=Layout::new(body,toolbar,scale,p,s.search);
            let caret=crate::ui::text_input::caret(0).is_some_and(|(visible,_)|visible);
            command_terminal_paint::content(self,s,g,caret);
            self.fill(g.toolbar,p.color(1));
            self.app_button_surface(g.appearance,s.menu!=Menu::None&&s.menu!=Menu::Clipboard,0,false,scale);
            self.app_label(g.appearance,b"Appearance",p.color(4),false,scale);
            self.polished_toolbar_button(g.find.x.max(0)as usize,g.find.y.max(0)as usize,g.find.width as usize,g.find.height as usize,b"",27);
            self.app_button_surface(g.clipboard,s.menu==Menu::Clipboard,0,false,scale);
            self.app_label(g.clipboard,b"Edit",p.color(4),false,scale);
            self.fill(g.status,p.color(1));
            let status=if !s.notice.is_empty(){s.notice}else if s.scroll>0 {b"Scrollback"}else{&s.prompt[9.min(s.prompt_length)..s.prompt_length.saturating_sub(3).max(9.min(s.prompt_length))]};
            self.app_label(Rect{x:g.status.x+18*scale as i32,width:g.status.width.saturating_sub(170*scale as u32),..g.status},status,p.color(8),false,scale);
            self.app_label(Rect{x:g.status.right()-148*scale as i32,width:136*scale as u32,..g.status},preferences::FONTS[p.font as usize],p.color(8),false,scale);
            if s.search {
                self.app_field_surface(g.field,1,false,scale);
                let label=if s.query_length==0{b"Find in scrollback".as_slice()}else{&s.query[..s.query_length]};
                let color=if s.query_length!=0&&s.found.is_none(){(240,128,128)}else{p.color(4)};
                self.app_label(Rect{x:g.field.x+8*scale as i32,width:g.field.width.saturating_sub(16*scale as u32),..g.field},label,color,false,scale);
                for (r,label) in [(g.previous,b"<".as_slice()),(g.next,b">"),(g.close,b"x")] {
                    self.app_button_surface(r,false,0,false,scale);self.app_label(r,label,p.color(4),false,scale);
                }
            }
            if s.menu!=Menu::None {
                let (menu,row)=g.menu_rect(s);
                self.app_card(menu,p.color(1),p.color(8),scale);
                for i in 0..s.menu_count(){
                    let r=Rect{x:menu.x+scale as i32,y:menu.y+(i*row)as i32,width:menu.width.saturating_sub(2*scale as u32),height:row as u32};
                    if i==s.menu_row{self.fill(r,p.color(3));}
                    let label=match s.menu {
                        Menu::Appearance=>[b"Color scheme".as_slice(),b"Monospace font",b"Text size"][i],
                        Menu::Schemes=>preferences::NAMES[i],Menu::Fonts=>preferences::FONTS[i],Menu::Sizes=>preferences::SIZE_NAMES[i],
                        Menu::Clipboard=>[b"Copy".as_slice(),b"Paste",b"Select all",b"Clear scrollback",b"Jump to prompt"][i],Menu::None=>b"",
                    };
                    let selected=match s.menu {Menu::Schemes=>p.scheme as usize==i,Menu::Fonts=>p.font as usize==i,Menu::Sizes=>p.size as usize==i,_=>false};
                    self.app_label(Rect{x:r.x+14*scale as i32,width:r.width.saturating_sub(40*scale as u32),..r},label,p.color(4),selected,scale);
                    if selected {self.app_label(Rect{x:r.right()-26*scale as i32,width:20*scale as u32,..r},b"*",p.color(5),true,scale);}
                }
            }
        });
    }
}
