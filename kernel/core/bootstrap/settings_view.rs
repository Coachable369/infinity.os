//! Production Settings painters consume typed snapshots without service access.
use super::DisplayDevice;
use super::primitives::UI_FONT_CELL_HEIGHT;
use crate::ui::system_layout::{SettingsWindowState,SettingsWindowGeometry,SettingsRowGeometry};
#[derive(Clone,Copy)]
pub(super) struct BasicSettingsSnapshot<'a> {
 pub chat_enabled:bool,pub chat_model:&'a[u8],pub voice_enabled:bool,pub wake_word:&'a[u8],pub speech_enabled:bool,
 pub preferences:crate::ui::input_preferences::Preferences,pub timeout_minutes:u8,pub timeout_maximum:u8,
}
#[derive(Clone,Copy)]
pub(super) struct SettingsChrome {
 pub geometry:SettingsWindowGeometry,pub primary:(u8,u8,u8),pub outline:(u8,u8,u8),pub selection:(u8,u8,u8),pub opacity:u8,
}
// ------------------------=
// FUNC: supports_basic_section
// DESC: Identifies sections whose complete native body consumes an explicit value snapshot.
// ------------------=
pub(super) const fn supports_basic_section(section:usize)->bool {matches!(section,0|2|3|4|5|9|10)}
// ------------------------=
// FUNC: basic_settings_rows
// DESC: Resolves the same labels and live values for OS snapshots and explicit preview fixtures.
// ------------------=
pub(super) fn basic_settings_rows<'a>(focus:usize,input:&'a[u8],snapshot:BasicSettingsSnapshot<'a>)->[(&'a[u8],&'a[u8]);8] {
 let BasicSettingsSnapshot{chat_enabled,chat_model,voice_enabled,wake_word,speech_enabled,preferences,..}=snapshot;
 match focus {
            10 => core::array::from_fn(|index| {
                (
                    crate::ui::input_preferences::LABELS[index],
                    preferences.value(index),
                )
            }),
            0 => [
                (b"Machine Name", input),
                (b"Language", b"English (US)"),
                (b"Region", b"United States"),
                (b"System Generation", b"Active"),
                (b"Updates", b"Generation based"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            2 => [
                (b"Current User", b"Active"),
                (b"Credential", b"Password"),
                (b"Session", b"Authenticated"),
                (b"Personal Space", b"Private"),
                (b"Profile", b"Persistent"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            3 => [
                (b"AI Provider", b"Local only"),
                (
                    b"Desktop AI Chat",
                    if chat_enabled {
                        b"Enabled"
                    } else {
                        b"Disabled"
                    },
                ),
                (b"Chat Model", chat_model),
                (b"Remote Processing", b"Off"),
                (b"Voice", if voice_enabled { b"Granted" } else { b"Restricted" }),
                (b"Wake Word", wake_word),
                (b"Spoken Replies", if speech_enabled { b"Enabled" } else { b"Disabled" }),
                (b"", b""),
            ],
            4 => [
                (b"Ambient Authority", b"Denied"),
                (b"Microphone", if voice_enabled { b"Granted" } else { b"Restricted" }),
                (b"Remote AI", b"Denied"),
                (b"No Activity Timeout", input),
                (b"Trusted UI", b"Active"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            5 => [
                (b"Display", b"Ready"),
                (b"Keyboard", b"Ready"),
                (b"Pointer", crate::ui::personalization::CURSOR_NAMES[preferences.cursor_style as usize]),
                (b"Audio Input", b"Unavailable"),
                (b"Audio Output", b"Unavailable"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
            _ => [
                (b"InfinityOS", b"Development"),
                (b"Architecture", b"Native"),
                (b"Boot", b"Verified"),
                (b"Identity Format", b"Version 1"),
                (b"Icon Families", b"Installed theme registry"),
                (b"", b""),
                (b"", b""),
                (b"", b""),
            ],
 }
}
impl DisplayDevice {
    // ------------------------=
    // FUNC: paint_settings_chrome
    // DESC: Paints authored window, navigation and headings through native geometry and fonts.
    // ------------------=
    pub(super) fn paint_settings_chrome(&mut self,focus:usize,settings_window:SettingsWindowState,scale:usize)->SettingsChrome {
        let caller_clip=self.render_clip;
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.settings_window_geometry_for_section(settings_window, focus);
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
        let title_height = geometry.title.height as usize;
        let template_console = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Console,
        );
        let settings_template = crate::ui::settings_template::template();
        let authored_rect = |element: crate::ui::installer_template::InstallerTemplateElement<'static>| {
            let console = template_console?;
            Some((
                left + element.frame.x.saturating_sub(console.frame.x) as usize * width
                    / console.frame.width.max(1) as usize,
                top + element.frame.y.saturating_sub(console.frame.y) as usize * height
                    / console.frame.height.max(1) as usize,
                element.frame.width as usize * width / console.frame.width.max(1) as usize,
                element.frame.height as usize * height / console.frame.height.max(1) as usize,
            ))
        };
        let (header_r, header_g, header_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Header);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (primary_r, primary_g, primary_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Widget);
        let (background_opacity, _) = self.active_background_effects();
        let panel_alpha = |base: u8| (u16::from(base) * u16::from(background_opacity) / 100) as u8;
        let authored_header = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Masthead,
        );
        let authored_navigation = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::SectionLabel,
        );
        let authored_content = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Content,
        );
        use crate::ui::{panel_recipe::{self,Material,Surface},installer_template::InstallerTemplateRole as Role};
        let authored_panel=|element:Option<crate::ui::installer_template::InstallerTemplateElement<'static>>|element.map(|mut e|{e.fill[3]=panel_alpha(e.fill[3]);e});
        if let Some(element)=authored_panel(template_console) {
            let mut style=panel_recipe::template_style(Surface::WindowFill,element.surface_material,element.corner_style,element.corner_radius);
            // Exterior geometry remains compositor-owned and rectangular for every native application.
            style.corners=1;style.radius=0;
            self.precision_surface_styled(geometry.window,Surface::WindowFill,Material{fill:(element.fill[0],element.fill[1],element.fill[2]),edge:(0,0,0),opacity:(u16::from(element.fill[3])*u16::from(element.opacity)/100)as u8,edge_opacity:0,focus:80,pressed:false,scale},style);
        }else{self.glass_window_substrate(geometry.window,true);}
        self.settings_template_surface(geometry.title,authored_panel(authored_header),Material{fill:(header_r,header_g,header_b),edge:(0,0,0),opacity:panel_alpha(222),edge_opacity:0,focus:0,pressed:false,scale});
        let title_center_y = top + title_height / 2;
        self.small_infinity_mark(left + 25 * scale, title_center_y, 31 * scale);
        self.ui_text_strong(
            left + 50 * scale,
            title_center_y.saturating_sub(UI_FONT_CELL_HEIGHT / 2),
            b"System Settings",
            241,
            246,
            250,
            1,
        );
        for (index,control) in crate::ui::app_launcher::window_controls::bounds(
            geometry.window,geometry.title.height,scale).into_iter().enumerate() {
            self.window_control(
                control.x.max(0) as usize,
                control.y.max(0) as usize,
                control.width as usize,
                index,
                settings_window.maximized,
            );
        }
        self.settings_template_surface(geometry.navigation,authored_panel(authored_navigation),Material{fill:(primary_r/2,primary_g/2,primary_b/2),edge:(36,58,76),opacity:panel_alpha(214),edge_opacity:0,focus:0,pressed:false,scale});
        let sections: [&[u8]; 11] = [
            b"General",
            b"Themes & Skins",
            b"Users & Accounts",
            b"AI & Voice",
            b"Privacy & Security",
            b"Devices",
            b"Network",
            b"Nodes & Mesh",
            b"Storage",
            b"About",
            b"Input",
        ];
        self.intersect_render_clip(
            geometry.navigation.x.max(0) as usize,
            geometry.navigation.y.max(0) as usize,
            geometry.navigation.width as usize,
            geometry.navigation.height as usize,
        );
        for (index, section) in sections.iter().enumerate() {
            let row = layout.settings_section_geometry_for_section(settings_window, index, focus);
            let (x, y, w, h) = (row.x.max(0) as usize, row.y.max(0) as usize,
                row.width as usize, row.height as usize);
            let navigation_item=crate::ui::settings_template::role_at(focus,Role::SettingsNavigationItem,index);
            self.settings_template_surface(row,authored_panel(navigation_item),Material{fill:(selection_r,selection_g,selection_b),edge:(outline_r,outline_g,outline_b),opacity:if index==focus{235}else{0},edge_opacity:if index==focus{255}else{0},focus:if index==focus{100}else{0},pressed:false,scale});
            if let Some(element)=crate::ui::settings_template::role_at(focus,Role::SettingsNavigationIcon,index) {
                if let Some(bounds)=layout.settings_element_geometry(settings_window,focus,element.id){
                    if !element.hidden&&element.opacity>0 {
                        self.template_element_in_rect(element,settings_template.and_then(|t|t.asset(element.image_asset)),None,crate::ui::installer_layout::InstallerRect{left:bounds.x.max(0)as usize,top:bounds.y.max(0)as usize,width:bounds.width as usize,height:bounds.height as usize});
                    }
                }
            }else{let size=(28*scale).min(h.saturating_sub(8));let _=self.themed_icon(x+28*scale,y+h/2,crate::ui::icon_theme::settings_section_icon(index),size);}
            let label=crate::ui::settings_template::role_at(focus,Role::SettingsNavigationLabel,index);
            let bounds=label.and_then(|e|layout.settings_element_geometry(settings_window,focus,e.id)).unwrap_or(crate::ui::geometry::Rect{x:(x+52*scale)as i32,y:(y+h.saturating_sub(UI_FONT_CELL_HEIGHT)/2)as i32,width:w.saturating_sub(68*scale)as u32,height:UI_FONT_CELL_HEIGHT as u32});
            self.paint_settings_bound_text(label,bounds,label.map(|e|e.text).filter(|t|!t.is_empty()).unwrap_or(section),if index==focus{(240,248,252)}else{(182,204,220)},true,false);
        }
        self.render_clip = caller_clip;
        let content_x = geometry.content.x.max(0) as usize;
        let content_y = geometry.content.y.max(0) as usize;
        if authored_content.is_some(){self.settings_template_surface(geometry.content,authored_panel(authored_content),Material{fill:(primary_r,primary_g,primary_b),edge:(0,0,0),opacity:0,edge_opacity:0,focus:0,pressed:false,scale});}
        if let Some(template) = settings_template {
            if let Some(count) = template.element_count(focus.saturating_add(1) as u8) {
                for index in 0..count {
                    let Some(element) = crate::ui::settings_template::layer_at(focus, index)
                        .filter(|element| !element.hidden && element.opacity > 0)
                    else { continue };
                    if !matches!(
                        element.role,
                        value if value == 0
                            || value == crate::ui::installer_template::InstallerTemplateRole::Image as u8
                            || (value == crate::ui::installer_template::InstallerTemplateRole::SettingsArtwork as u8
                                && !matches!(focus, 6 | 7 | 8))
                    ) {
                        continue;
                    }
                    let Some((x, y, mapped_width, mapped_height)) = authored_rect(element) else {
                        continue;
                    };
                    self.template_element_in_rect(
                        element,
                        if element.kind == 2 { template.asset(element.image_asset) } else { None },
                        None,
                        crate::ui::installer_layout::InstallerRect {
                            left: x, top: y, width: mapped_width, height: mapped_height,
                        },
                    );
                }
            }
        }
        let authored_title = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Title,
        );
        let authored_body = crate::ui::settings_template::element(
            focus,
            crate::ui::installer_template::InstallerTemplateRole::Body,
        );
        let title_bounds=authored_title.and_then(|e|layout.settings_element_geometry(settings_window,focus,e.id)).unwrap_or(crate::ui::geometry::Rect{x:content_x as i32,y:content_y as i32,width:geometry.content.width,height:(56*scale)as u32});
        self.paint_settings_bound_text(authored_title,title_bounds,authored_title.map(|e|e.text).filter(|t|!t.is_empty()).unwrap_or(sections[focus.min(10)]),(238,244,249),true,false);
        if focus!=8 {
            let bounds=authored_body.and_then(|e|layout.settings_element_geometry(settings_window,focus,e.id)).unwrap_or(crate::ui::geometry::Rect{x:content_x as i32,y:(content_y+if focus==7{64}else{36}*scale)as i32,width:geometry.content.width,height:(28*scale)as u32});
            self.paint_settings_bound_text(authored_body,bounds,authored_body.map(|e|e.text).filter(|t|!t.is_empty()).unwrap_or(if focus==10{b"Click a row to change its value. Changes apply immediately."}else{b"Open a row to view its controls and configuration details."}),(143,160,176),false,false);
        }
        SettingsChrome{geometry,primary:(primary_r,primary_g,primary_b),outline:(outline_r,outline_g,outline_b),selection:(selection_r,selection_g,selection_b),opacity:background_opacity}
    }
    // ------------------------=
    // FUNC: paint_settings_bound_text
    // DESC: Honors authored typography and alpha inside exact semantic bounds while retaining native fallback text for missing layers.
    // ------------------=
    pub(super) fn paint_settings_bound_text(&mut self,element:Option<crate::ui::installer_template::InstallerTemplateElement<'_>>,bounds:crate::ui::geometry::Rect,text:&[u8],fallback:(u8,u8,u8),strong:bool,right:bool)->usize {
        if bounds.width==0||bounds.height==0{return 0;}
        let old=self.render_clip;
        self.intersect_render_clip(bounds.x.max(0)as usize,bounds.y.max(0)as usize,bounds.width as usize,bounds.height as usize);
        let mut n=text.len();
        let width=if let Some(element)=element {
            if element.hidden||element.opacity==0{self.render_clip=old;return 0;}
            let pixels=(element.font_size as usize*self.height/1000).clamp(1,256);
            while n>0&&self.template_text_width(&text[..n],pixels,strong)>bounds.width as usize{n-=1;}
            let w=self.template_text_width(&text[..n],pixels,strong);
            self.template_text(bounds.x.max(0)as usize+if right{(bounds.width as usize).saturating_sub(w)}else{0},bounds.y.max(0)as usize,&text[..n],
                element.fill[0],element.fill[1],element.fill[2],(u16::from(element.fill[3])*u16::from(element.opacity)/100)as u8,pixels,strong);w
        } else {
            while n>0&&self.ui_text_width(&text[..n],1)>bounds.width as usize{n-=1;}
            let w=self.ui_text_width(&text[..n],1);let x=bounds.x.max(0)as usize+if right{(bounds.width as usize).saturating_sub(w)}else{0};
            if strong{self.ui_text_strong(x,bounds.y.max(0)as usize,&text[..n],fallback.0,fallback.1,fallback.2,1);}
            else{self.ui_text(x,bounds.y.max(0)as usize,&text[..n],fallback.0,fallback.1,fallback.2,1);}w
        };
        self.render_clip=old;width
    }
    // ------------------------=
    // FUNC: paint_settings_row_summary
    // DESC: Paints authored row materials, child positions, native text and disclosure using shared runtime geometry.
    // ------------------=
    pub(super) fn paint_settings_row_summary(&mut self,focus:usize,index:usize,label:&[u8],value:&[u8],settings_window:SettingsWindowState,scale:usize,chrome:SettingsChrome)->SettingsRowGeometry {
        use crate::ui::{installer_template::InstallerTemplateRole as Role,panel_recipe::Material,geometry::Rect};
        let layout=crate::ui::system_layout::SystemLayout::new(self.width,self.height);
        let row=layout.settings_row_geometry_for_section(settings_window,index,focus);
        let authored_row=crate::ui::settings_template::role_at(focus,Role::Metadata,index);
        if !row.summary.intersects(chrome.geometry.viewport)||authored_row.is_some_and(|e|e.hidden||e.opacity==0){return row;}
        let authored_label=crate::ui::settings_template::role_at(focus,Role::SettingsRowLabel,index);
        let authored_value=crate::ui::settings_template::role_at(focus,Role::SettingsRowValue,index);
        let authored_disclosure=crate::ui::settings_template::role_at(focus,Role::SettingsDisclosure,index);
        let (pr,pg,pb)=chrome.primary;let(er,eg,eb)=chrome.outline;
        let expanded=settings_window.expanded_row==Some(index);
        let alpha=|n:u8|(u16::from(n)*u16::from(chrome.opacity)/100)as u8;
        self.settings_template_surface(row.summary,authored_row.map(|mut e|{e.fill[3]=alpha(e.fill[3]);e}),Material{fill:(pr/2,pg.saturating_mul(3)/4,pb.saturating_mul(3)/4),edge:if expanded{(er,eg,eb)}else{(er/2,eg/2,eb/2)},opacity:alpha(218),edge_opacity:255,focus:0,pressed:false,scale});
        let card=crate::ui::settings_cards::content(row.summary,scale);
        self.fill_rounded_rect_alpha(card.icon.x.max(0)as usize,card.icon.y.max(0)as usize,card.icon.width as usize,card.icon.height as usize,9*scale,4,20,34,alpha(150));
        self.outline_rounded_rect(card.icon.x.max(0)as usize,card.icon.y.max(0)as usize,card.icon.width as usize,card.icon.height as usize,9*scale,35,64,84);
        let _=self.themed_icon((card.icon.x+card.icon.width as i32/2).max(0)as usize,(card.icon.y+card.icon.height as i32/2).max(0)as usize,crate::ui::icon_theme::settings_section_icon(focus),(28*scale).min(card.icon.width as usize));
        self.ui_text_elided_strong(card.description.x.max(0)as usize,card.description.y.max(0)as usize,card.description.width as usize,crate::ui::settings_cards::description(focus,index),141,174,198);
        let label=if focus==1&&index==6{label}else{authored_label.map(|e|e.text).filter(|text|!text.is_empty()).unwrap_or(label)};
        let label_bounds=layout.settings_row_element_geometry(settings_window,focus,index,Role::SettingsRowLabel).unwrap_or(card.label);
        let value_bounds=layout.settings_row_element_geometry(settings_window,focus,index,Role::SettingsRowValue).unwrap_or(card.value);
        self.paint_settings_bound_text(authored_label,label_bounds,label,(190,205,217),true,false);
        let advance=self.paint_settings_bound_text(authored_value,value_bounds,value,(220,232,240),false,true);
        if focus==0&&index==0&&!authored_value.is_some_and(|e|e.hidden||e.opacity==0||e.fill[3]==0) {
            if let Some(element)=authored_value {
                if let Some((true,caret))=crate::ui::text_input::caret(1){
                    let pixels=(element.font_size as usize*self.height/1000).clamp(1,256);
                    let x=value_bounds.x.max(0)as usize+(value_bounds.width as usize).saturating_sub(advance)+self.template_text_width(&value[..caret.min(value.len())],pixels,false);
                    let old=self.render_clip;self.intersect_render_clip(value_bounds.x.max(0)as usize,value_bounds.y.max(0)as usize,value_bounds.width as usize,value_bounds.height as usize);
                    self.fill_rect(x,value_bounds.y.max(0)as usize,scale.max(1),pixels.min(value_bounds.height as usize),112,221,255);self.render_clip=old;
                }
            }else{self.text_field_caret(value_bounds.x.max(0)as usize+(value_bounds.width as usize).saturating_sub(advance),value_bounds.y.max(0)as usize,value_bounds.height as usize,value,true,1);}
        }
        let disclosure=layout.settings_row_element_geometry(settings_window,focus,index,Role::SettingsDisclosure).unwrap_or(Rect{x:row.summary.right()-(30*scale)as i32,y:row.summary.y+(20*scale)as i32,width:(20*scale)as u32,height:(32*scale)as u32});
        let text=if expanded{b"v".as_slice()}else{authored_disclosure.map(|e|e.text).filter(|v|!v.is_empty()).unwrap_or(b">")};
        self.paint_settings_bound_text(authored_disclosure,disclosure,text,(109,220,255),true,false);
        row
    }
    // ------------------------=
    // FUNC: paint_settings_basic_detail
    // DESC: Paints disclosure explanations and trusted action controls from observed state.
    // ------------------=
    pub(super) fn paint_settings_basic_detail(&mut self,focus:usize,index:usize,detail:crate::ui::geometry::Rect,scale:usize,snapshot:BasicSettingsSnapshot<'_>,state:SettingsWindowState) {
        let BasicSettingsSnapshot{chat_enabled,voice_enabled,speech_enabled,..}=snapshot;
        let detail_left=detail.x.max(0)as usize;let detail_top=detail.y.max(0)as usize;let detail_width=detail.width as usize;let detail_height=detail.height as usize;
                    let description: &[u8] = match (focus, index) {
                        (0, 0) => b"Rename this machine through the durable identity service.",
                        (0, 1) => b"The installed interface language is English (US).",
                        (0, 2) => b"Regional formatting follows the installed US locale.",
                        (0, 3) => b"This is the active installed System Generation.",
                        (0, 4) => b"Updates are delivered as a complete System Generation.",
                        (1, 0) => b"Switch between installed, verified InfinityUI skins.",
                        (1, 6) => b"Automatic scale follows the active display density.",
                        (1, 7) => b"Cosmic Horizon is the active packaged desktop wallpaper.",
                        (3, 0) => {
                            b"Choose whether the local provider is strictly required or preferred."
                        }
                        (3, 1) => b"Show or hide your local desktop AI chat widget.",
                        (3, 2) => b"Choose the next installed local model without restarting.",
                        (2, 0) => b"The authenticated user owns this desktop session.",
                        (2, 1) => b"Password credentials are managed by the trusted identity flow.",
                        (2, 2) => b"This session is authenticated; locking preserves your workspace.",
                        (2, 3) => b"Personal Space isolates this user's objects and preferences.",
                        (2, 4) => b"The user profile persists with the installed system.",
                        (3, 3) => b"Remote processing is disabled; local inference stays on this device.",
                        (3, 4) => b"Microphone access follows the current user permission.",
                        (3, 5) => b"Choose the phrase that opens an eight-second command window.",
                        (3, 6) => b"Model operations remain subject to capability enforcement.",
                        (4, 0) => b"Applications receive no implicit authority over your data.",
                        (4, 1) => b"Microphone access is explicitly granted or restricted here.",
                        (4, 2) => b"Remote AI access requires explicit capability authorization.",
                        (4, 3) => {
                            b"Lock this user's session after the selected period without input."
                        }
                        (4, 4) => b"Sensitive decisions use the trusted system interface.",
                        (5, 0) => b"The active display is used by the native desktop compositor.",
                        (5, 1) => b"Open Input to adjust keyboard repeat delay and rate.",
                        (5, 3 | 4) => b"This audio path is unavailable; no device control is exposed.",
                        (9, 0) => b"InfinityOS development System Generation.",
                        (9, 1) => b"The desktop and services execute in the native OS runtime.",
                        (9, 2) => b"Startup uses the installed boot contract and generation checks.",
                        (9, 3) => b"Identity records use the version 1 durable format.",
                        (9, 4) => b"Select an installed icon family in Themes & Skins.",
                        _ => b"This value is read from the active System Generation.",
                    };
                    self.ui_text_elided_strong(
                        detail_left + crate::ui::system_layout::UI_GUTTER * scale,
                        detail_top + crate::ui::system_layout::UI_GUTTER * scale,
                        detail_width.saturating_sub(crate::ui::system_layout::UI_GUTTER * 2 * scale),
                        description,
                        167,
                        188,
                        203,
                    );
                    let layout=crate::ui::system_layout::SystemLayout::new(self.width,self.height);
                    if let Some(action)=layout.settings_action_geometry(state,focus,index) {
                        if !action.visible { return; }
                        let caption=action.action.caption(chat_enabled,voice_enabled,speech_enabled);
                        let previous_clip=self.render_clip;
                        let clip=action.clip.intersection(crate::ui::geometry::Rect{x:0,y:0,width:self.width as u32,height:self.height as u32});
                        self.intersect_render_clip(clip.x.max(0)as usize,clip.y.max(0)as usize,clip.width as usize,clip.height as usize);
                        if let Some(mut element)=action.element {
                            if element.caption_mode==0 {element.text=caption;}
                            let interaction=crate::ui::app_launcher::window_controls::state(action.bounds);
                            self.paint_template_button_state(element,action.bounds,true,interaction!=0,interaction==2);
                        } else {
                            self.polished_button(action.bounds.x.max(0)as usize,action.bounds.y.max(0)as usize,
                                action.bounds.width as usize,action.bounds.height as usize,caption,true,false);
                        }
                        self.render_clip=previous_clip;
                    } else if (focus,index)==(1,0) {
                        self.polished_button(
                            detail_left + crate::ui::system_layout::UI_GUTTER * scale,
                            detail_top
                                + detail_height.saturating_sub(
                                    (crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT + 10)
                                        * scale,
                                ),
                            (240 * scale).min(
                                detail_width.saturating_sub(
                                    crate::ui::system_layout::UI_GUTTER * 2 * scale,
                                ),
                            ),
                            crate::ui::system_layout::UI_COMPACT_ACTION_HEIGHT * scale,
                            b"SWITCH SKIN",
                            true,
                            false,
                        );
                    }
    }
    // ------------------------=
    // FUNC: render_settings_overflow_chrome
    // DESC: Draws the proportional Settings scrollbar and resize grip outside the clipped content viewport.
    // ------------------=
    pub(super) fn render_settings_overflow_chrome(
        &mut self,
        geometry: crate::ui::system_layout::SettingsWindowGeometry,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        outline: (u8, u8, u8),
    ) {
        let (outline_r, outline_g, outline_b) = outline;
        let left = geometry.window.x.max(0) as usize;
        let top = geometry.window.y.max(0) as usize;
        let width = geometry.window.width as usize;
        let height = geometry.window.height as usize;
        if geometry.maximum_scroll > 0 {
            let track = geometry.scrollbar_track;
            let thumb = geometry.scrollbar_thumb;
            self.fill_rounded_rect_alpha(
                track.x.max(0) as usize,
                track.y.max(0) as usize,
                track.width as usize,
                track.height as usize,
                3 * scale,
                6,
                19,
                31,
                190,
            );
            self.fill_rounded_rect_alpha(
                thumb.x.max(0) as usize,
                thumb.y.max(0) as usize,
                thumb.width as usize,
                thumb.height as usize,
                3 * scale,
                outline_r,
                outline_g,
                outline_b,
                235,
            );
        }
    }

    // ------------------------=
    // FUNC: settings_slider
    // DESC: Renders the shared polished track and thumb for a bounded Settings value.
    // ------------------=
    pub(super) fn settings_slider(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        index: usize,
        value: u8,
        maximum: u8,
        section: usize,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.settings_effect_slider_geometry_for_section(
            settings_window,
            index,
            value,
            maximum,
            section,
        );
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        self.fill_rounded_rect_alpha(
            geometry.track.x.max(0) as usize,
            geometry.track.y.max(0) as usize,
            geometry.track.width as usize,
            geometry.track.height as usize,
            4 * scale,
            214,
            228,
            241,
            94,
        );
        self.outline_rounded_rect(
            geometry.track.x.max(0) as usize,
            geometry.track.y.max(0) as usize,
            geometry.track.width as usize,
            geometry.track.height as usize,
            4 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
        self.fill_rounded_rect_alpha(
            geometry.thumb.x.max(0) as usize,
            geometry.thumb.y.max(0) as usize,
            geometry.thumb.width as usize,
            geometry.thumb.height as usize,
            10 * scale,
            239,
            247,
            255,
            255,
        );
        self.outline_rounded_rect(
            geometry.thumb.x.max(0) as usize,
            geometry.thumb.y.max(0) as usize,
            geometry.thumb.width as usize,
            geometry.thumb.height as usize,
            10 * scale,
            outline_r,
            outline_g,
            outline_b,
        );
    }

    // ------------------------=
    // FUNC: render_basic_settings_view
    // DESC: Renders supported complete sections through shared production chrome, row and control painters.
    // ------------------=
    pub(super) fn render_basic_settings_view(&mut self,focus:usize,input:&[u8],state:SettingsWindowState,scale:usize,snapshot:BasicSettingsSnapshot<'_>)->bool {
        if !supports_basic_section(focus){return false;}
        let old=self.render_clip;let chrome=self.paint_settings_chrome(focus,state,scale);let g=chrome.geometry;
        self.paint_basic_settings_body(focus,input,state,scale,snapshot,chrome);
        self.render_clip=old;self.render_settings_overflow_chrome(g,state,scale,chrome.outline);true
    }
    // ------------------------=
    // FUNC: paint_basic_settings_body
    // DESC: Shares the complete supported Settings body between live runtime and the native authoring preview.
    // ------------------=
    pub(super) fn paint_basic_settings_body(&mut self,focus:usize,input:&[u8],state:SettingsWindowState,scale:usize,snapshot:BasicSettingsSnapshot<'_>,chrome:SettingsChrome) {
        let g=chrome.geometry;
        self.intersect_render_clip(g.viewport.x.max(0)as usize,g.viewport.y.max(0)as usize,g.viewport.width as usize,g.viewport.height as usize);
        for(index,(label,value))in basic_settings_rows(focus,input,snapshot).iter().take(state.row_count.clamp(1,8)).enumerate(){
            if crate::ui::settings_template::role_at(focus,crate::ui::installer_template::InstallerTemplateRole::Metadata,index).is_some_and(|e|e.hidden||e.opacity==0){continue;}
            let row=self.paint_settings_row_summary(focus,index,label,value,state,scale,chrome);
            if state.expanded_row==Some(index)&&row.detail.intersects(g.viewport){
                let(r,g,b)=chrome.primary;let(er,eg,eb)=chrome.outline;
                self.app_card_alpha(row.detail,(r/2,g.saturating_mul(2)/3,b.saturating_mul(2)/3),(er/2,eg/2,eb/2),(232u16*u16::from(chrome.opacity)/100)as u8,scale);
                if focus==5&&index==2{self.settings_personalization_panel_from_state(row.detail,scale,true,snapshot.preferences,crate::ui::spatial::backdrop_tint(),crate::ui::personalization::save_status());}
                else if focus==4&&index==3{self.settings_slider(state,scale,index,snapshot.timeout_minutes.saturating_sub(1),snapshot.timeout_maximum.saturating_sub(1),4);}
                else{self.paint_settings_basic_detail(focus,index,row.detail,scale,snapshot,state);}
            }
        }
    }
}
