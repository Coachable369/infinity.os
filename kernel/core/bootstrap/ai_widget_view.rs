//! Desktop AI controls reuse the shipped native app typography and glass-menu recipe.
use super::*;
use crate::ui::{ai_widget, geometry::Rect, system_layout::AiChatGeometry};

impl DisplayDevice {
    // ------------------------=
    // FUNC: ai_widget_header
    // DESC: Paints a bounded native status dropdown and distinct, generous window controls.
    // ------------------=
    pub(super) fn ai_widget_header(&mut self, geometry: AiChatGeometry, status: &[u8],
        minimized: bool, busy: bool, scale: usize, accent: (u8,u8,u8)) {
        let g = geometry;
        if !minimized {
            self.app_label(Rect { x: g.header.x + (16 * scale) as i32,
                y: g.header.y + (8 * scale) as i32, width: (82 * scale) as u32,
                height: (28 * scale) as u32 }, b"AI CHAT", (221,238,247), true, scale);
        }
        self.app_card(g.status, (9,30,47),
            if ai_widget::current_menu().open { accent } else { (44,87,114) }, scale);
        let dot_x = g.status.x + (10 * scale) as i32;
        let dot_y = g.status.y + g.status.height as i32 / 2;
        self.fill_rounded_rect_alpha(dot_x.max(0) as usize, (dot_y - 2 * scale as i32).max(0) as usize,
            4 * scale, 4 * scale, 2 * scale, accent.0, accent.1, accent.2, if busy {255} else {170});
        self.app_menu_label(Rect { x: g.status.x + (20 * scale) as i32,
            width: g.status.width.saturating_sub((40 * scale) as u32), ..g.status },
            status, (194,220,235), scale);
        let arrow_x = g.status.right() - (11 * scale) as i32;
        self.line(arrow_x - 3 * scale as i32, dot_y - scale as i32, arrow_x, dot_y + 2 * scale as i32, 148,190,215);
        self.line(arrow_x, dot_y + 2 * scale as i32, arrow_x + 3 * scale as i32, dot_y - scale as i32, 148,190,215);
        for (rect, symbol) in [(g.minimize, if minimized {b'+'} else {b'-'}), (g.close,b'x')] {
            self.app_card(rect, (10,28,43), (41,72,94), scale);
            self.app_symbol(rect, symbol, (190,214,228), scale);
        }
    }

    // ------------------------=
    // FUNC: ai_widget_composer
    // DESC: Paints the real proportional prompt viewport and caret inside a fixed clipped native input field.
    // ------------------=
    pub(super) fn ai_widget_composer(&mut self, field: Rect, text: &[u8], cursor: usize,
        caret: Option<(bool,usize)>, scale: usize, accent: (u8,u8,u8)) {
        self.app_card(field, (4,19,32), if caret.is_some() { accent } else { (43,80,104) }, scale);
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
