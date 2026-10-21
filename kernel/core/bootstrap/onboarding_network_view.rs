//! Shared native first-boot network view; runtime and Studio supply typed observable state.
use super::DisplayDevice;
use super::access_style as access;
use crate::ui::{installer_layout::InstallerRect,panel_recipe::Surface};

#[derive(Clone,Copy)]
pub(super) struct NetworkViewState<'a> {
    pub selected:usize,
    pub connected:bool,
    pub wired_detail:&'a[u8],
    pub wireless_detail:&'a[u8],
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: onboarding_network_rows_from_state
    // DESC: Paints authoritative network choices with the same native geometry, fonts, glyphs and surfaces in runtime and draft previews.
    // ------------------=
    pub(super) fn onboarding_network_rows_from_state(&mut self,fallback:(usize,usize,usize),
        frames:[Option<InstallerRect>;3],focus:usize,validation_error:bool,state:NetworkViewState<'_>) {
        let (left,top,width)=fallback;
        let scale=self.ui_scale().max(1);
        let background=access::semantic_color(0);let elevated=access::semantic_color(2);
        let border=access::semantic_color(3);let text=access::semantic_color(4);
        let secondary=access::semantic_color(5);let error=access::semantic_color(11);
        let rows:[(&[u8],&[u8],usize);3]=[
            (b"Wired network",state.wired_detail,15),
            (b"Wi-Fi",state.wireless_detail,1),
            (b"Continue offline",b"Set up networking later in Settings",16),
        ];
        for (index,(label,detail,icon)) in rows.iter().enumerate() {
            let authored=frames[index];
            if width==0 && authored.is_none() {continue;}
            let left=authored.map(|frame|frame.left).unwrap_or(left);
            let width=authored.map(|frame|frame.width).unwrap_or(width);
            let row_top=authored.map(|frame|frame.top).unwrap_or(top+index*100*scale);
            let row_height=authored.map(|frame|frame.height).unwrap_or(90*scale);
            let display_height=self.height;
            let unit=|value:usize|if authored.is_some(){(value*display_height/1000).max(1)}else{value*scale};
            let inset=unit(12);let icon_size=unit(24);let label_left=left+unit(46);
            let text_width=width.saturating_sub(unit(46)+inset);
            let selected=state.selected==index;
            self.access_surface(left,row_top,width,row_height,Surface::Panel,
                if selected{elevated}else{background},border,255,focus==index+2);
            self.access_icon(left+inset+icon_size/2,row_top+row_height/2,*icon,icon_size,selected);
            self.access_label((label_left,row_top+unit(6),text_width.saturating_sub(if selected{unit(80)}else{0}),unit(26)),
                label,unit(20),text,false,true);
            let previous_clip=self.render_clip;
            self.intersect_render_clip(label_left,row_top+unit(34),text_width,row_height.saturating_sub(unit(38)));
            self.access_text_wrapped(label_left,row_top+unit(34),text_width,detail,
                secondary.0,secondary.1,secondary.2,255,unit(18),false);
            self.render_clip=previous_clip;
            if selected {
                let status:&[u8]=if state.connected && index<2{b"ACTIVE"}else{b"SELECTED"};
                let status_width=self.access_text_width(status,unit(16),false);
                self.access_text(left+width.saturating_sub(inset+status_width),row_top+unit(10),status,
                    border.0,border.1,border.2,255,unit(16),false);
            }
        }
        if validation_error {
            let last=frames[2];
            self.access_label((last.map(|frame|frame.left).unwrap_or(left),
                last.map(|frame|frame.bottom()+8*scale).unwrap_or(top+178*scale),
                last.map(|frame|frame.width).unwrap_or(width),32*scale),
                b"That connection is unavailable. Connect hardware or choose Offline.",14*scale,error,false,false);
        }
    }
}
