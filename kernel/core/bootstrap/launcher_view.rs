//! Shared native launcher painter used by the desktop and Studio preview.
use super::*;
impl DisplayDevice {
    // ------------------------=
    // FUNC: paint_app_launcher
    // DESC: Composes the complete translucent launcher from one stable desktop backdrop.
    // ------------------=
    pub(crate) fn paint_app_launcher(&mut self, scale: usize, query: &[u8], focus: usize) {
        let geometry = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_geometry();
        let presentation = crate::ui::app_launcher::launcher_presentation();
        let panel_left = geometry.panel.x.max(0) as usize;
        let panel_top = geometry.panel.y.max(0) as usize;
        let panel_width = geometry.panel.width as usize;
        let panel_height = geometry.panel.height as usize;
        let authored = crate::ui::desktop_template::bound_scene(81);
        let outer_clip = self.render_clip;
        if authored.is_some() {
            self.intersect_render_clip(panel_left,panel_top,panel_width,panel_height);
        }
        if let Some(scene) = authored {
            self.desktop_template_scene(crate::ui::desktop_template::BYTES,scene,&[],None,false,&[]);
        } else {
        self.fill_rounded_rect_alpha(
            panel_left.saturating_sub(8 * scale),
            panel_top + 10 * scale,
            panel_width.saturating_add(16 * scale),
            panel_height,
            26 * scale,
            0,
            2,
            10,
            132,
        );
        self.app_card_alpha(geometry.panel,(16,27,58),(134,199,255),230,scale);
        let close_left = geometry.close.x.max(0) as usize;
        let close_top = geometry.close.y.max(0) as usize;
        let close_size = geometry.close.width as usize;
        self.fill_rounded_rect_alpha(
            close_left,
            close_top,
            close_size,
            geometry.close.height as usize,
            7 * scale,
            12,
            30,
            47,
            238,
        );
        self.outline_rounded_rect(
            close_left,
            close_top,
            close_size,
            geometry.close.height as usize,
            7 * scale,
            75,
            111,
            137,
        );
        let center_x = close_left + close_size / 2;
        let center_y = close_top + geometry.close.height as usize / 2;
        self.icon_line(
            (center_x - 5 * scale) as i32,
            (center_y - 5 * scale) as i32,
            (center_x + 5 * scale) as i32,
            (center_y + 5 * scale) as i32,
            (201, 224, 239),
            close_size,
        );
        self.icon_line(
            (center_x + 5 * scale) as i32,
            (center_y - 5 * scale) as i32,
            (center_x - 5 * scale) as i32,
            (center_y + 5 * scale) as i32,
            (201, 224, 239),
            close_size,
        );

        }

        let search_left = geometry.search.x.max(0) as usize;
        let search_top = geometry.search.y.max(0) as usize;
        let search_width = geometry.search.width as usize;
        let search_height = geometry.search.height as usize;
        if authored.is_none() {
        self.fill_rounded_rect_alpha(
            search_left,
            search_top,
            search_width,
            search_height,
            search_height / 2,
            5,
            18,
            34,
            232,
        );
        self.outline_rounded_rect(
            search_left,
            search_top,
            search_width,
            search_height,
            search_height / 2,
            if focus == 0 { 101 } else { 50 },
            if focus == 0 { 205 } else { 121 },
            if focus == 0 { 255 } else { 168 },
        );
        }
        let search_clip = self.render_clip;
        self.intersect_render_clip(search_left,search_top,search_width,search_height);
        let search_icon_size = (24 * scale).min(search_height.saturating_sub(10));
        let _ = self.themed_icon(
            search_left + 27 * scale,
            search_top + search_height / 2,
            27,
            search_icon_size,
        );
        let search_text = if query.is_empty() {
            b"Search apps, files, settings and more...".as_slice()
        } else {
            query
        };
        self.ui_text(
            search_left + 50 * scale,
            (search_top + search_height / 2).saturating_sub(UI_FONT_CELL_HEIGHT / 2),
            search_text,
            if query.is_empty() { 126 } else { 224 },
            if query.is_empty() { 151 } else { 236 },
            if query.is_empty() { 177 } else { 247 },
            1,
        );
        self.text_field_caret(
            search_left + 50 * scale,
            search_top,
            search_height,
            query,
            focus == 0,
            1,
        );
        let shortcut = b"/ SEARCH";
        let shortcut_width = self.ui_text_width(shortcut, 1);
        self.fill_rounded_rect_alpha(
            search_left + search_width.saturating_sub(shortcut_width + 25 * scale),
            search_top + 10 * scale,
            shortcut_width + 14 * scale,
            search_height.saturating_sub(20 * scale),
            6 * scale,
            16,
            37,
            58,
            215,
        );
        self.ui_text(
            search_left + search_width.saturating_sub(shortcut_width + 18 * scale),
            (search_top + search_height / 2).saturating_sub(UI_FONT_CELL_HEIGHT / 2),
            shortcut,
            138,
            166,
            192,
            1,
        );

        self.render_clip = search_clip;
        let visible = crate::ui::app_launcher::launcher_visible_count(query);
        let launcher_clip = self.render_clip;
        self.intersect_render_clip(
            geometry.grid_viewport.x.max(0) as usize,
            geometry.grid_viewport.y.max(0) as usize,
            geometry.grid_viewport.width as usize,
            geometry.grid_viewport.height as usize,
        );
        for visible_index in 0..visible {
            let Some(entry) = crate::ui::app_launcher::launcher_visible_entry(query, visible_index)
            else {
                continue;
            };
            if presentation.drag_moved && presentation.drag_source == Some(visible_index) {
                continue;
            }
            let slot = crate::ui::app_launcher::launcher_display_slot(visible_index, presentation);
            let column = slot % crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let row = slot / crate::ui::app_launcher::LAUNCHER_COLUMNS;
            let cell_left = geometry.grid_left + column * geometry.grid_cell_width;
            let cell_top = (geometry.grid_top + row * geometry.grid_row_height)
                .saturating_sub(presentation.scroll);
            let selected = focus == visible_index + 1;
            let well_size = geometry
                .grid_cell_width
                .min(geometry.grid_row_height)
                .saturating_mul(70)
                / 100;
            let well_left = cell_left + geometry.grid_cell_width.saturating_sub(well_size) / 2;
            let well_top = cell_top + 3 * scale;
            self.fill_rounded_rect_alpha(
                well_left,
                well_top,
                well_size,
                well_size,
                13 * scale,
                7,
                28,
                49,
                186,
            );
            self.outline_rounded_rect(
                well_left,
                well_top,
                well_size,
                well_size,
                13 * scale,
                if selected { 100 } else { 48 },
                if selected { 211 } else { 105 },
                if selected { 255 } else { 146 },
            );
            let _ = self.launcher_icon(
                well_left + well_size / 2,
                well_top + well_size / 2,
                entry.icon_role,
                well_size * 84 / 100,
            );
            self.ui_text_centered_strong(
                cell_left,
                geometry.grid_cell_width,
                well_top + well_size + 8 * scale,
                entry.label(),
                220,
                235,
                245,
                1,
            );
        }
        self.render_clip = launcher_clip;
        let scroll = crate::ui::system_layout::SystemLayout::new(self.width, self.height)
            .app_launcher_scroll_geometry(visible);
        if scroll.maximum_scroll != 0 {
            self.fill_rounded_rect_alpha(
                geometry.scrollbar_track.x.max(0) as usize,
                geometry.scrollbar_track.y.max(0) as usize,
                geometry.scrollbar_track.width as usize,
                geometry.scrollbar_track.height as usize,
                geometry.scrollbar_track.width as usize / 2,
                16,
                43,
                65,
                176,
            );
            self.fill_rounded_rect_alpha(
                scroll.thumb.x.max(0) as usize,
                scroll.thumb.y.max(0) as usize,
                scroll.thumb.width as usize,
                scroll.thumb.height as usize,
                scroll.thumb.width as usize / 2,
                84,
                199,
                249,
                242,
            );
        }
        if presentation.drag_moved && crate::ui::app_launcher::shortcuts::current().drag.is_none() {
            if let Some(source) = presentation.drag_source {
                if let Some(entry) = crate::ui::app_launcher::launcher_visible_entry(query, source)
                {
                    let ghost_size = geometry
                        .grid_cell_width
                        .min(geometry.grid_row_height)
                        .saturating_mul(70)
                        / 100;
                    let center_x = (self.width as i32 * presentation.drag_x / 1000)
                        .clamp(0, self.width.saturating_sub(1) as i32)
                        as usize;
                    let center_y = (self.height as i32 * presentation.drag_y / 1000)
                        .clamp(0, self.height.saturating_sub(1) as i32)
                        as usize;
                    let left = center_x.saturating_sub(ghost_size / 2);
                    let top = center_y.saturating_sub(ghost_size / 2);
                    self.fill_rounded_rect_alpha(
                        left.saturating_sub(5 * scale),
                        top.saturating_sub(5 * scale),
                        ghost_size + 10 * scale,
                        ghost_size + 10 * scale,
                        17 * scale,
                        29,
                        105,
                        151,
                        210,
                    );
                    self.outline_rounded_rect(
                        left,
                        top,
                        ghost_size,
                        ghost_size,
                        13 * scale,
                        112,
                        220,
                        255,
                    );
                    let _ = self.launcher_icon(
                        center_x,
                        center_y,
                        entry.icon_role,
                        ghost_size * 84 / 100,
                    );
                }
            }
        }
        if visible == 0 {
            self.ui_text_centered_strong(
                geometry.grid_left,
                geometry.grid_cell_width * 6,
                (geometry.grid_top + geometry.grid_row_height).saturating_sub(UI_FONT_CELL_HEIGHT / 2),
                b"No matching applications",
                149,
                178,
                201,
                1,
            );
        }

        let divider_y = panel_top + panel_height * 70 / 100;
        self.fill_rect_alpha(
            geometry.category_left,
            divider_y,
            geometry.category_width * 5,
            scale,
            55,
            109,
            143,
            150,
        );
        for (index, entry) in crate::ui::app_launcher::LAUNCHER_CATEGORIES
            .iter()
            .enumerate()
        {
            let left = geometry.category_left + index * geometry.category_width + 6 * scale;
            let width = geometry.category_width.saturating_sub(12 * scale);
            let selected = focus == visible + index + 1;
            self.fill_rounded_rect_alpha(
                left,
                geometry.category_top,
                width,
                geometry.category_height,
                12 * scale,
                7,
                29,
                50,
                220,
            );
            self.outline_rounded_rect(
                left,
                geometry.category_top,
                width,
                geometry.category_height,
                12 * scale,
                if selected { 98 } else { 45 },
                if selected { 212 } else { 99 },
                if selected { 255 } else { 139 },
            );
            let icon_size = geometry.category_height * 48 / 100;
            let _ = self.launcher_icon(
                left + width / 2,
                geometry.category_top + geometry.category_height * 37 / 100,
                entry.icon_role,
                icon_size,
            );
            self.ui_text_centered(
                left,
                width,
                geometry.category_top + geometry.category_height * 68 / 100,
                entry.label,
                197,
                220,
                237,
                1,
            );
        }
        self.render_clip = outer_clip;
    }
}
