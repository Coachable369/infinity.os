//! Desktop AI controls reuse the shipped native app typography and glass-menu recipe.
use super::*;
use crate::ui::{ai_widget, geometry::Rect, system_layout::AiChatGeometry};

impl DisplayDevice {
    // ------------------------=
    // FUNC: ai_widget_notice_wrap
    // DESC: Measures word boundaries with the same native 14px caption face used to paint admission notices.
    // ------------------=
    pub(crate) fn ai_widget_notice_wrap(&self,text:&[u8],width:usize,scale:usize)->([usize;16],usize) {
        let mut ends=[0;16];let mut count=0;let mut start=0;
        while start<text.len() && count<ends.len() {
            while text.get(start)==Some(&b' ') {start+=1;}
            if start==text.len() {break;}
            let mut end=start;let mut space=None;
            while end<text.len() {
                if text[end]==b' ' {space=Some(end);}
                if self.app_menu_text_width(&text[start..=end],scale)>width {break;}
                end+=1;
            }
            if end<text.len() {end=space.filter(|offset|*offset>start).unwrap_or(end.max(start+1));}
            ends[count]=end;count+=1;start=end;
        }
        (ends,count)
    }

    // ------------------------=
    // FUNC: ai_widget_advice_notice
    // DESC: Shows a typed admission failure in the existing transcript card style, clipped above the unchanged composer.
    // ------------------=
    pub(crate) fn ai_widget_advice_notice(&mut self,bounds:Rect,text:&[u8],scale:usize)->Rect {
        let inset=10*scale;
        let text_width=(bounds.width as usize).saturating_sub(inset*2);
        let line_height=20*scale;
        let (ends,lines)=self.ai_widget_notice_wrap(text,text_width,scale);
        let height=(lines*line_height+44*scale).min(bounds.height as usize);
        let card=Rect {y:bounds.bottom()-height as i32,height:height as u32,..bounds};
        let saved=self.render_clip;
        self.intersect_render_clip(card.x.max(0)as usize,card.y.max(0)as usize,card.width as usize,card.height as usize);
        self.app_card_alpha(card,(16,27,58),(134,199,255),255,scale);
        self.app_label(Rect {x:card.x+inset as i32,y:card.y+(8*scale)as i32,
            width:text_width as u32,height:(24*scale)as u32},b"Request not sent",(255,205,210),true,scale);
        let mut start=0;
        for (row,end) in ends[..lines].iter().copied().enumerate() {
            while text.get(start)==Some(&b' ') {start+=1;}
            self.app_menu_label(Rect {x:card.x+inset as i32,y:card.y+(34*scale+row*line_height)as i32,
                width:text_width as u32,height:line_height as u32},&text[start..end],(224,235,243),scale);
            start=end;
        }
        self.render_clip=saved;
        card
    }

    // ------------------------=
    // FUNC: ai_widget_model
    // DESC: Shares the live model card with Studio while clipping content to the authored control.
    // ------------------=
    pub(crate) fn ai_widget_model(&mut self, bounds:Rect, name:&[u8], scale:usize) {
        if !self.desktop_bound_surface(66,bounds){return;}
        let saved=self.render_clip;
        self.intersect_render_clip(bounds.x.max(0)as usize,bounds.y.max(0)as usize,bounds.width as usize,bounds.height as usize);
        self.app_label(Rect{x:bounds.x+(12*scale)as i32,y:bounds.y+(4*scale)as i32,
            width:bounds.width.saturating_sub((24*scale)as u32),height:(20*scale)as u32},b"MODEL",(123,151,170),false,scale);
        self.app_label(Rect{x:bounds.x+(12*scale)as i32,y:bounds.y+(24*scale)as i32,
            width:bounds.width.saturating_sub((40*scale)as u32),height:(24*scale)as u32},name,(224,235,243),true,scale);
        self.app_disclosure(Rect{x:bounds.right()-(24*scale)as i32,y:bounds.y,width:(20*scale)as u32,height:bounds.height},false,(134,199,255),scale);
        self.render_clip=saved;
    }
    // ------------------------=
    // FUNC: ai_widget_empty_timeline
    // DESC: Uses the same bounded welcome content in the live conversation and authoring preview.
    // ------------------=
    pub(crate) fn ai_widget_empty_timeline(&mut self,bounds:Rect,scale:usize) {
        if bounds.width==0||bounds.height==0{return;}
        let saved=self.render_clip;
        self.intersect_render_clip(bounds.x.max(0)as usize,bounds.y.max(0)as usize,bounds.width as usize,bounds.height as usize);
        for (offset,text,strong) in [(18,b"How can I help with InfinityOS?".as_slice(),true),
            (44,b"Ask about this system, storage,".as_slice(),false),(62,b"devices, memory, or networking.".as_slice(),false)] {
            self.app_label(Rect{x:bounds.x+(14*scale)as i32,y:bounds.y+(offset*scale)as i32,
                width:bounds.width.saturating_sub((28*scale)as u32),height:(24*scale)as u32},text,
                if strong{(218,232,241)}else{(137,160,178)},strong,scale);
        }
        self.render_clip=saved;
    }
    // ------------------------=
    // FUNC: ai_widget_send
    // DESC: Draws the authored send surface with the shared antialiased action glyph.
    // ------------------=
    pub(crate) fn ai_widget_send(&mut self,bounds:Rect,enabled:bool,scale:usize) {
        if !self.desktop_bound_surface(69,bounds){return;}
        self.app_disclosure(bounds,false,if enabled{(250,253,255)}else{(120,145,160)},scale);
    }
    // ------------------------=
    // FUNC: ai_widget_header
    // DESC: Paints live microphone amplitude beside status, with a separate stop-audio control.
    // ------------------=
    pub(super) fn ai_widget_header(&mut self, geometry: AiChatGeometry, status: &[u8],
        minimized: bool, busy: bool, scale: usize, accent: (u8,u8,u8)) {
        let g = geometry;
        self.desktop_bound_surface(65,g.status);
        if g.status.width>0 && g.status.height>0 {
        let saved=self.render_clip;
        self.intersect_render_clip(g.status.x.max(0)as usize,g.status.y.max(0)as usize,g.status.width as usize,g.status.height as usize);
        let dot_y = g.status.y + g.status.height as i32 / 2;
        for index in 0..17usize {
            let envelope = 9 - index.abs_diff(8);
            let height = scale * (2 + ai_widget::meter() * envelope * 2 / 32768);
            self.fill_rounded_rect_alpha((g.status.x + ((8 + index * 3) * scale) as i32).max(0) as usize,
                (dot_y - height as i32 / 2).max(0) as usize, scale.max(1), height, scale,
                accent.0, accent.1, accent.2, 220);
        }
        self.app_menu_label(Rect { x: g.status.x + (66 * scale) as i32,
            width: g.status.width.saturating_sub((100 * scale) as u32), ..g.status },
            status, (194,220,235), scale);
        self.paint_bitmap_alpha_fit_rect_opacity(include_bytes!("../../../assets/ui-design-kit/default/stop-audio-v1.bmp"),
            (g.status.right() - (30 * scale) as i32).max(0) as usize,
            g.status.y.max(0) as usize, 30 * scale, 30 * scale, if busy {255} else {120});
        self.render_clip=saved;
        }
        for (rect, symbol) in [(g.minimize, if minimized {b'+'} else {b'-'}), (g.close,b'x')] {
            if rect.width>0&&rect.height>0 {
                self.desktop_bound_surface(if symbol==b'x' {71}else{70},rect);
                self.app_symbol(rect, symbol, (190,214,228), scale);
            }
        }
    }

    // ------------------------=
    // FUNC: ai_widget_composer
    // DESC: Paints the real proportional prompt viewport and caret inside a fixed clipped native input field.
    // ------------------=
    pub(super) fn ai_widget_composer(&mut self, field: Rect, text: &[u8], cursor: usize,
        caret: Option<(bool,usize)>, scale: usize, accent: (u8,u8,u8)) {
        if field.width==0||field.height==0{return;}
        self.app_input_surface(field,caret.is_some(),scale);
        let inner = Rect { x: field.x + (12 * scale) as i32, y: field.y + (5 * scale) as i32,
            width: field.width.saturating_sub((24 * scale) as u32),
            height: field.height.saturating_sub((10 * scale) as u32) };
        let view = ai_widget::prompt_viewport(text, cursor, inner.width as usize, scale);
        let old = self.render_clip;
        self.intersect_render_clip(inner.x.max(0) as usize, inner.y.max(0) as usize,
            inner.width as usize, inner.height as usize);
        let y = field.y.max(0) as usize + (field.height as usize).saturating_sub(24 * scale) / 2;
        self.app_text(inner.x.max(0) as usize, y,
            if text.is_empty() { b"Ask InfinityOS..." } else { &text[view.start..view.end] },
            if text.is_empty() { (124,151,171) } else { (224,237,246) }, false, scale);
        if caret.is_some_and(|(visible,_)| visible) {
            self.fill_rect(inner.x.max(0) as usize + view.caret_x,
                field.y.max(0) as usize + (field.height as usize).saturating_sub(20 * scale) / 2,
                scale, 20 * scale, accent.0, accent.1, accent.2);
        }
        self.render_clip = old;
    }

    // ------------------------=
    // FUNC: ai_widget_status_menu
    // DESC: Paints typed status actions above the timeline using the existing compact native glass dropdown.
    // ------------------=
    pub(super) fn ai_widget_status_menu(&mut self, anchor: Rect, menu: ai_widget::MenuState,
        voice_enabled: bool, can_stop: bool, scale: usize) {
        if !menu.open { return; }
        let layout = ai_widget::MenuGeometry::new(anchor,self.width,self.height,scale);
        self.app_dropdown_panel(layout.panel,scale);
        let labels: [&[u8];3] = [if voice_enabled {b"Turn voice off"} else {b"Turn voice on"},
            b"Stop response", b"AI & Voice settings"];
        for (index,row) in layout.rows.into_iter().enumerate() {
            let enabled = index != 1 || can_stop;
            self.app_dropdown_row(row,menu.selected==index,scale);
            self.app_menu_label(Rect {x:row.x+(10*scale)as i32,width:row.width.saturating_sub((20*scale)as u32),..row},
                labels[index],if enabled {(223,237,246)}else{(97,124,145)},scale);
        }
    }
}
