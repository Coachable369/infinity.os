//! Shared native destructive confirmation and button painters; no installation authority.
use super::DisplayDevice;

impl DisplayDevice {
    // ------------------------=
    // FUNC: installer_confirmation_popup
    // DESC: Draws the modal destructive-action warning and its two explicit choices.
    // ------------------=
    pub(super) fn installer_confirmation_popup(
        &mut self,
        focus: usize,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let scale = self.ui_scale();
        let left = self.width * 25 / 100;
        let top = self.height * 34 / 100;
        let width = self.width * 50 / 100;
        let height = self.height * 32 / 100;
        // Repaint from an opaque glass base on every state change. This keeps
        // alpha highlights deterministic instead of accumulating luminance
        // when the pointer crosses a control boundary.
        use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
        let bounds=Rect{x:left as i32,y:top as i32,width:width as u32,height:height as u32};
        let material=Material{fill:super::access_style::SURFACE,edge:super::access_style::BORDER,
            opacity:255,edge_opacity:255,focus:255,pressed:false,scale};
        self.precision_surface(bounds,Surface::WindowFill,Material{edge_opacity:0,..material});
        self.precision_surface(Rect{x:(left+24*scale) as i32,y:(top+25*scale) as i32,
            width:(42*scale) as u32,height:(42*scale) as u32},Surface::Panel,
            Material{fill:(91,43,39),edge:(255,118,145),focus:0,..material});
        self.access_glyph(left+45*scale,top+46*scale,8,23*scale,false);
        self.ui_text_strong(
            left + 82 * scale,
            top + 26 * scale,
            b"Erase this disk?",
            250,
            247,
            244,
            1,
        );
        self.ui_text(
            left + 82 * scale,
            top + 50 * scale,
            b"This action permanently removes all existing data.",
            188,
            203,
            216,
            1,
        );
        self.ui_text(
            left + 24 * scale,
            top + 88 * scale,
            b"InfinityOS will create a new Infinity Pool on the selected disk.",
            221,
            231,
            239,
            1,
        );
        self.ui_text_strong(
            left + 24 * scale,
            top + 116 * scale,
            b"This cannot be undone.",
            246,
            158,
            139,
            1,
        );
        let cancel_frame = crate::ui::installer_layout::installer_confirmation_button_frame(
            crate::ui::installer_layout::InstallerConfirmationTarget::Cancel,
        );
        self.installer_button(
            cancel_frame.left,
            cancel_frame.top,
            cancel_frame.width,
            cancel_frame.height,
            b"CANCEL",
            b"Keep the disk unchanged",
            false,
            focus == 0,
            cursor_x,
            cursor_y,
            pressed,
        );
        let install_frame = crate::ui::installer_layout::installer_confirmation_button_frame(
            crate::ui::installer_layout::InstallerConfirmationTarget::Install,
        );
        self.installer_button(
            install_frame.left,
            install_frame.top,
            install_frame.width,
            install_frame.height,
            b"ERASE & INSTALL",
            b"Begin installation",
            true,
            focus == 1,
            cursor_x,
            cursor_y,
            pressed,
        );
        self.installer_confirmation_footer(b"Tab / arrows: choose     Enter: confirm     Esc: cancel");
        self.precision_surface(bounds,Surface::Frame,Material{opacity:0,..material});
    }

    // ------------------------=
    // FUNC: installer_confirmation_footer
    // DESC: Fits native Inter instructions wholly inside the confirmation frame, including its scaled glyph cell and inset.
    // ------------------=
    pub(super) fn installer_confirmation_footer(&mut self,hint:&[u8]) {
        let left=self.width/4;let top=self.height*34/100;
        let width=self.width/2;let height=self.height*32/100;
        let padding=(12*self.height/1000).max(4);
        let mut pixels=(16*self.height/1000).max(8);
        while pixels>6 && self.access_text_width(hint,pixels,false)>width.saturating_sub(padding*2) {pixels-=1;}
        let cell=pixels*28/24;
        let x=left+width.saturating_sub(self.access_text_width(hint,pixels,false))/2;
        let y=top+height.saturating_sub(cell+padding);
        let clip=self.render_clip;
        self.intersect_render_clip(left+padding,y,width.saturating_sub(2*padding),cell);
        self.access_text(x,y,hint,137,159,179,255,pixels,false);
        self.render_clip=clip;
    }

    // ------------------------=
    // FUNC: installer_button
    // DESC: Draws one normalized installer button with hover and keyboard focus styling.
    // ------------------=
    pub(super) fn installer_button(
        &mut self,
        nx: usize,
        ny: usize,
        nw: usize,
        nh: usize,
        title: &[u8],
        subtitle: &[u8],
        primary: bool,
        focused: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        self.installer_button_rect(
            crate::ui::installer_layout::InstallerRect {
                left: self.width * nx / 1000,
                top: self.height * ny / 1000,
                width: self.width * nw / 1000,
                height: self.height * nh / 1000,
            },
            title,
            subtitle,
            primary,
            focused,
            cursor_x,
            cursor_y,
            pressed,
        );
    }

    // ------------------------=
    // FUNC: installer_button_rect
    // DESC: Draws an installer button at a shared pixel-space layout rectangle.
    // ------------------=
    pub(super) fn installer_button_rect(
        &mut self,
        rect: crate::ui::installer_layout::InstallerRect,
        title: &[u8],
        subtitle: &[u8],
        primary: bool,
        focused: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) {
        let left = rect.left;
        let top = rect.top;
        let width = rect.width;
        let height = rect.height;
        let pointer_x = cursor_x * self.width as i32 / 1000;
        let pointer_y = cursor_y * self.height as i32 / 1000;
        let hovered = pointer_x >= left as i32
            && pointer_x <= rect.right() as i32
            && pointer_y >= top as i32
            && pointer_y <= rect.bottom() as i32;
        let depressed = hovered && pressed;
        use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
        self.precision_surface(Rect{x:left as i32,y:top as i32,width:width as u32,height:height as u32},
            if primary {Surface::Primary}else{Surface::Secondary},
            Material{fill:if primary {super::access_style::PRIMARY}else{super::access_style::SURFACE},
                edge:super::access_style::BORDER,opacity:255,edge_opacity:255,
                focus:if focused||hovered {255}else{0},pressed:depressed,scale:self.ui_scale()});
        let scale = self.ui_scale();
        let press_offset = if depressed { 2 * scale } else { 0 };
        let inset=(8*self.height/1000).max(2);
        let available=width.saturating_sub(64*scale);
        let mut title_pixels=(18*self.height/1000).max(8);
        let mut subtitle_pixels=(14*self.height/1000).max(7);
        while title_pixels>6 && self.access_text_width(title,title_pixels,true)>available {title_pixels-=1;}
        while subtitle_pixels>6 && self.access_text_width(subtitle,subtitle_pixels,false)>available {subtitle_pixels-=1;}
        let gap=(3*self.height/1000).max(1);
        while title_pixels>6 && title_pixels*28/24+subtitle_pixels*28/24+gap>height.saturating_sub(inset*2) {
            title_pixels-=1;subtitle_pixels=subtitle_pixels.saturating_sub(1).max(6);
        }
        let title_height=title_pixels*28/24;
        let block_height=title_height+subtitle_pixels*28/24+gap;
        let title_y = top + height.saturating_sub(block_height) / 2 + press_offset;
        let subtitle_y = title_y + title_height + gap;
        let title_width = self.access_text_width(title,title_pixels,true);
        let subtitle_width = self.access_text_width(subtitle,subtitle_pixels,false);
        let title_x = left + width.saturating_sub(title_width) / 2;
        let subtitle_x = left + width.saturating_sub(subtitle_width) / 2;
        let clip=self.render_clip;
        self.intersect_render_clip(left+inset,top+inset,width.saturating_sub(inset*2),height.saturating_sub(inset*2));
        self.access_text(title_x,title_y,title,244,248,255,255,title_pixels,true);
        self.access_text(subtitle_x,subtitle_y,subtitle,137,191,238,255,subtitle_pixels,false);
        self.render_clip=clip;
        if focused || hovered {
            let arrow_x = if left + width / 2 < self.width / 2 {
                (left + 26 * scale) as i32
            } else {
                (left + width.saturating_sub(24 * scale)) as i32
            };
            let arrow_y = (top + height / 2 + press_offset) as i32;
            self.line(
                arrow_x - 8 * scale as i32,
                arrow_y,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
            self.line(
                arrow_x - 4 * scale as i32,
                arrow_y - 4 * scale as i32,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
            self.line(
                arrow_x - 4 * scale as i32,
                arrow_y + 4 * scale as i32,
                arrow_x,
                arrow_y,
                106,
                207,
                255,
            );
        }
    }
}
