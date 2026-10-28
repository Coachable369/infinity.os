//! Runtime compositor for the exact layer model saved by Installer Studio.

#[path = "reveal.rs"]
mod boot_reveal;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::desktop::ONBOARDING_BMP;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::installer::{
    INSTALLER_MASTHEAD_BMP, INSTALLER_PROGRESS_HERO_BMP, INSTALLER_WELCOME_MASTHEAD_BMP,
};
use super::*;
use super::installer::{INSTALLER_FONT_CELL_HEIGHT, INSTALLER_FONT_NATIVE_SIZE_PX};
use crate::ui::installer_layout::{
    scale_template_rect, CONFIGURATION_TEMPLATE_BYTES, INSTALLER_TEMPLATE_BYTES,
};
use crate::ui::installer_template::{
    template_image_uses_aspect_fill, InstallerTemplate, InstallerTemplateElement,
    InstallerTemplateRole,
};

#[derive(Clone, Copy)]
pub(super) struct ConfigurationViewState<'a> {
    pub input: &'a [u8],
    pub masked: bool,
    pub validation_error: bool,
    pub network: Option<super::onboarding_network_view::NetworkViewState<'a>>,
}

impl DisplayDevice {
    // ------------------------=
    // FUNC: login_template_scene
    // DESC: Shares authored authentication pixels between Studio and the OS while keeping credentials masked and identity native.
    // ------------------=
    pub(crate) fn login_template_scene(&mut self, bytes: &[u8], account: &[u8], input: &[u8], focus: usize) -> bool {
        let Ok(template) = InstallerTemplate::parse_login(bytes) else { return false; };
        let Some(count) = template.element_count(1) else { return false; };
        for index in 0..count {
            let Some(mut element) = template.layer_at(1, index) else { return false; };
            if element.hidden || element.opacity == 0 { continue; }
            let action = crate::ui::installer_template::login_action(&element);
            if action == Some(0) { element.text = account; }
            if action == Some(3) {
                element.text=b"Sign In with Facial Recognition";
                #[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {element.text=crate::runtime::face_setup_host::login_caption();}
            }
            if action == Some(1) {
                let rect = scale_template_rect(element.frame, self.width, self.height);
                self.onboarding_input_field_with_element(Some(element), rect.left, rect.top, rect.width, rect.height,
                    input, true, focus == 1, b"Password");
            } else if element.kind == 3 {
                self.configuration_template_text_layer(element);
            } else if matches!(element.kind, 1 | 4 | 5) {
                // Reuse the shared glass renderer and its native primary/secondary states.
                self.access_template_control_with_focus(element, None, (0, 0), Some(action == Some(focus)));
            } else {
                self.template_element(element, template.asset(element.image_asset), None);
            }
        }
        true
    }

    // ------------------------=
    // FUNC: access_template_panel_glow
    // DESC: Applies the native breathing rim with the authored panel's same geometry, colors and corner coverage.
    // ------------------=
    pub(super) fn access_template_panel_glow(&mut self,element:InstallerTemplateElement<'_>) {
        if element.hidden || element.opacity==0 {return;}
        use crate::ui::panel_recipe::{self,Surface,Material};
        let rect=scale_template_rect(element.frame,self.width,self.height);
        let edge_opacity=(u32::from(element.border[3])*u32::from(element.opacity)
            *u32::from(super::access_style::panel_motion().glow)/100/255)as u8;
        self.precision_surface_styled(crate::ui::geometry::Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},Surface::Panel,
            Material{fill:(element.fill[0],element.fill[1],element.fill[2]),
                edge:(element.border[0],element.border[1],element.border[2]),opacity:0,edge_opacity,
                focus:255,pressed:false,scale:self.ui_scale()},
            panel_recipe::template_style(Surface::Panel,element.surface_material,element.corner_style,element.corner_radius));
    }
    // ------------------------=
    // FUNC: settings_template_surface
    // DESC: Applies saved surface colors, alpha, material and geometry to live Settings controls without changing their service-owned contents.
    // ------------------=
    pub(crate) fn settings_template_surface(&mut self, rect:crate::ui::geometry::Rect,
        element:Option<InstallerTemplateElement<'_>>, fallback:crate::ui::panel_recipe::Material) {
        use crate::ui::panel_recipe::{self,Surface};
        let mut material=fallback;
        let mut style=panel_recipe::surface_style(Surface::Panel);
        if let Some(element)=element {
            if element.hidden {return;}
            material.fill=(element.fill[0],element.fill[1],element.fill[2]);
            material.edge=(element.border[0],element.border[1],element.border[2]);
            material.opacity=(u16::from(element.fill[3])*u16::from(element.opacity)/100) as u8;
            material.edge_opacity=(u16::from(element.border[3])*u16::from(element.opacity)/100) as u8;
            style=panel_recipe::template_style(Surface::Panel,element.surface_material,element.corner_style,element.corner_radius);
        }
        self.precision_surface_styled(rect,Surface::Panel,material,style);
    }
    // ------------------------=
    // FUNC: installer_template_screen
    // DESC: Paints every visible installer layer in the editor's deterministic stacking order.
    // ------------------=
    pub(super) fn installer_template_screen(&mut self, screen: u8) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, screen, None, true, false, None, None)
    }

    // ------------------------=
    // FUNC: installer_template_screen_with_details
    // DESC: Substitutes live data at its saved layer position instead of drawing a second fixed-position overlay.
    // ------------------=
    pub(super) fn installer_template_screen_with_details(&mut self, screen: u8, details: &[u8]) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, screen, None, true, false, Some(details), None)
    }

    // ------------------------=
    // FUNC: installer_template_progress_screen
    // DESC: Composes saved Installing layers in order while substituting only the live progress value and status.
    // ------------------=
    pub(super) fn installer_template_progress_screen(&mut self, percent: usize, label: &[u8]) -> bool {
        self.template_screen(INSTALLER_TEMPLATE_BYTES, 8, None, true, false, None, Some((percent, label)))
    }

    // ------------------------=
    // FUNC: configuration_template_scene_data
    // DESC: Composes live first-boot values at their authored layer positions, including native input, network content and navigation.
    // ------------------=
    pub(super) fn configuration_template_scene_data(&mut self, bytes:&[u8], step:usize,
        focus:usize, state:ConfigurationViewState<'_>) -> bool {
        self.template_scene_data_with_state(bytes,step.saturating_add(1)as u8,
            Some((focus,true,None)),true,false,None,None,true,false,None,Some(state))
    }

    // ------------------------=
    // FUNC: installer_template_navigation
    // DESC: Repaints authored navigation layers while applying current focus and availability state.
    // ------------------=
    pub(super) fn installer_template_navigation(
        &mut self,
        screen: u8,
        focus: usize,
        has_primary: bool,
        cursor_x: i32,
        cursor_y: i32,
        pressed: bool,
    ) -> bool {
        self.template_screen(
            INSTALLER_TEMPLATE_BYTES,
            screen,
            Some((focus, has_primary, Some((cursor_x, cursor_y, pressed)))),
            false,
            false,
            None,
            None,
        )
    }

    // ------------------------=
    // FUNC: template_screen
    // DESC: Resolves and paints either a complete template scene or only its interactive navigation layers.
    // ------------------=
    fn template_screen(
        &mut self,
        bytes: &'static [u8],
        screen: u8,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
        full_scene: bool,
        skip_live_input: bool,
        live_details: Option<&[u8]>,
        progress: Option<(usize, &[u8])>,
    ) -> bool {
        self.template_scene_data(bytes,screen,navigation,full_scene,skip_live_input,live_details,progress,
            bytes.as_ptr()==CONFIGURATION_TEMPLATE_BYTES.as_ptr(),false)
    }

    // ------------------------=
    // FUNC: template_scene_data
    // DESC: Shares the production scene, native glyph and image composition path with validated unsaved Studio documents.
    // ------------------=
    pub(super) fn template_scene_data(&mut self,bytes:&[u8],screen:u8,
        navigation:Option<(usize,bool,Option<(i32,i32,bool)>)>,full_scene:bool,skip_live_input:bool,
        live_details:Option<&[u8]>,progress:Option<(usize,&[u8])>,configuration:bool,settings:bool)->bool {
        self.template_scene_data_with_date_time(bytes,screen,navigation,full_scene,skip_live_input,
            live_details,progress,configuration,settings,None)
    }

    // ------------------------=
    // FUNC: template_scene_data_with_date_time
    // DESC: Substitutes actual installer date/time state at each authored layer position while retaining the shared production compositor.
    // ------------------=
    pub(super) fn template_scene_data_with_date_time(&mut self,bytes:&[u8],screen:u8,
        navigation:Option<(usize,bool,Option<(i32,i32,bool)>)>,full_scene:bool,skip_live_input:bool,
        live_details:Option<&[u8]>,progress:Option<(usize,&[u8])>,configuration:bool,settings:bool,
        date_time:Option<(crate::storage::DateTimeConfiguration,usize)>)->bool {
        self.template_scene_data_with_state(bytes,screen,navigation,full_scene,skip_live_input,
            live_details,progress,configuration,settings,date_time,None)
    }

    // ------------------------=
    // FUNC: template_scene_data_with_state
    // DESC: Uses one deterministic layer walk for authored and service-owned pixels so later layers always retain their stacking order.
    // ------------------=
    fn template_scene_data_with_state(&mut self,bytes:&[u8],screen:u8,
        navigation:Option<(usize,bool,Option<(i32,i32,bool)>)>,full_scene:bool,skip_live_input:bool,
        live_details:Option<&[u8]>,progress:Option<(usize,&[u8])>,configuration:bool,settings:bool,
        date_time:Option<(crate::storage::DateTimeConfiguration,usize)>,
        configuration_state:Option<ConfigurationViewState<'_>>)->bool {
        let parsed=if settings {InstallerTemplate::parse_settings(bytes)}else{InstallerTemplate::parse(bytes)};
        let Ok(template) = parsed else {
            return false;
        };
        let Some(count) = template.element_count(screen) else {
            return false;
        };
        // V10 installer scenes own all background pixels, including when their
        // editable background image is hidden or removed. Never reveal a second
        // implicit brand image underneath transparent authored layers.
        if full_scene && !configuration && !settings && template.version >= 10 {
            self.fill_rect(0, 0, self.width, self.height, 0, 0, 0);
        }
        let mut progress_index=0usize;
        for index in 0..count {
            let Some(element) = template.layer_at(screen, index) else {
                return false;
            };
            if element.hidden || element.opacity == 0 {
                continue;
            }
            // Detail-local children are painted only by the expanded Settings row.
            // A template-only view has no parent detail geometry to resolve them.
            if settings && crate::ui::installer_template::settings_row_child_role(element.role) {
                continue;
            }
            if full_scene
                && skip_live_input
                && element.role == InstallerTemplateRole::Input as u8
            {
                continue;
            }
            let is_navigation = element.role == InstallerTemplateRole::BackButton as u8
                || element.role == InstallerTemplateRole::PrimaryButton as u8
                || element.role == InstallerTemplateRole::Footer as u8;
            if full_scene || is_navigation {
                if let Some(state)=configuration_state {
                    if element.role==InstallerTemplateRole::Input as u8 {
                        let rect=scale_template_rect(element.frame,self.width,self.height);
                        self.onboarding_input_field_with_element(Some(element),rect.left,rect.top,rect.width,rect.height,
                            state.input,state.masked,navigation.is_some_and(|(focus,_,_)|focus==1),element.text);
                        if state.validation_error {
                            let color=super::access_style::semantic_color(11);
                            self.access_label((rect.left,rect.bottom().saturating_add(8*self.ui_scale().max(1)),
                                rect.width,24*self.ui_scale().max(1)),
                                if state.masked {b"Use at least eight characters."}else{b"This field is required before continuing."},
                                14*self.ui_scale().max(1),color,false,false);
                        }
                        continue;
                    }
                }
                if element.kind == 6 {
                    let (percent, label) = progress.unwrap_or((0, element.text));
                    self.template_progress_control(
                        scale_template_rect(element.frame, self.width, self.height),
                        element.fill, element.border, element.opacity, element.corner_radius,
                        element.surface_material, element.corner_style, element.font_size, percent, label,
                    );
                    continue;
                }
                if element.role == InstallerTemplateRole::LiveDetails as u8 {
                    if let Some(text) = live_details {
                        self.template_element(InstallerTemplateElement { text, ..element }, None, navigation);
                    }
                    continue;
                }
                let image = if element.kind == 2 {
                    template.asset(element.image_asset)
                } else {
                    None
                };
                if let Some((value,part))=date_time {
                    if self.installer_date_time_element(element,image,navigation,value,part) {continue;}
                }
                if configuration
                    && matches!(element.kind, 1 | 4 | 5)
                {
                    let progress=(progress_index,screen.saturating_sub(1) as usize);
                    if element.role==InstallerTemplateRole::ProgressSegment as u8 {progress_index+=1;}
                    self.access_template_control(element, navigation, progress);
                } else if configuration
                    && element.kind == 3
                {
                    self.configuration_template_text_layer(element);
                } else {
                    self.template_element(element, image, navigation);
                }
                if element.role==InstallerTemplateRole::Content as u8 {
                    if let Some(state)=configuration_state {
                        if let Some(network)=state.network {
                            let frames=core::array::from_fn(|index|
                                crate::ui::installer_layout::configuration_network_row_rect_in(&template,index,self.width,self.height));
                            self.onboarding_network_rows_from_state((0,0,0),frames,
                                navigation.map(|(focus,_,_)|focus).unwrap_or(0),state.validation_error,network);
                        }
                    }
                }
            }
        }
        true
    }

    // ------------------------=
    // FUNC: configuration_template_text_layer
    // DESC: Uses the authored Inter typography and fits only single-line-height titles so responsive aspect ratios cannot hide their final words.
    // ------------------=
    fn configuration_template_text_layer(&mut self, element: InstallerTemplateElement<'_>) {
        self.with_template_typography(element, |display| display.configuration_template_text_layer_styled(element));
    }

    // ------------------------=
    // FUNC: configuration_template_text_layer_styled
    // DESC: Measures and paints configured text using its scoped native typography.
    // ------------------=
    fn configuration_template_text_layer_styled(&mut self, element: InstallerTemplateElement<'_>) {
        let rect = scale_template_rect(element.frame, self.width, self.height);
        let strong = element.role == InstallerTemplateRole::Title as u8;
        let mut pixels = (element.font_size as usize * self.height / 1000).max(6);
        let line_height = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels)
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT);
        let line_advance = FontSize::new(INSTALLER_FONT_NATIVE_SIZE_PX, pixels)
            .scale_usize(INSTALLER_FONT_CELL_HEIGHT + 4);
        if strong && !element.text.contains(&b'\n') && rect.height < line_height + line_advance {
            while pixels > 6 && self.access_text_width(element.text, pixels, strong) > rect.width {
                pixels -= 1;
            }
        }
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        self.access_text_wrapped(rect.left, rect.top, rect.width, element.text,
            element.fill[0], element.fill[1], element.fill[2],
            (element.fill[3] as u16 * element.opacity as u16 / 100) as u8, pixels, strong);
        self.render_clip = previous_clip;
    }

    // ------------------------=
    // FUNC: access_template_control
    // DESC: Renders saved configuration controls with the approved Access gradient, focus easing and subpixel edges.
    // ------------------=
    pub(super) fn access_template_control(&mut self, element: InstallerTemplateElement<'_>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>, progress:(usize,usize)) {
        self.access_template_control_with_focus(element, navigation, progress, None);
    }

    // ------------------------=
    // FUNC: access_template_control_with_focus
    // DESC: Shares the native control painter while allowing authentication's typed action focus without changing button styles.
    // ------------------=
    fn access_template_control_with_focus(&mut self, element: InstallerTemplateElement<'_>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>, progress:(usize,usize), focus_override: Option<bool>) {
        self.with_template_typography(element, |display| display.access_template_control_styled(element, navigation, progress, focus_override));
    }

    // ------------------------=
    // FUNC: access_template_control_styled
    // DESC: Paints an access control with the selected caption typography scoped to this layer.
    // ------------------=
    fn access_template_control_styled(&mut self, element: InstallerTemplateElement<'_>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>, progress:(usize,usize), focus_override: Option<bool>) {
        use super::access_style as palette;
        use crate::ui::panel_recipe::Surface;
        let rect=scale_template_rect(element.frame,self.width,self.height);
        let focused=focus_override.unwrap_or_else(|| navigation.is_some_and(|(focus,_,_)|
            (element.role==InstallerTemplateRole::BackButton as u8 && focus==0)
            || (element.role==InstallerTemplateRole::PrimaryButton as u8 && focus==1)
            || (element.role==InstallerTemplateRole::Input as u8 && focus==1)));
        let fill=(element.fill[0],element.fill[1],element.fill[2]);
        let border=(element.border[0],element.border[1],element.border[2]);
        let opacity=(element.fill[3] as u16*element.opacity as u16/100) as u8;
        let border_opacity=(element.border[3] as u16*element.opacity as u16/100) as u8;
        let text_opacity=(u16::from(element.caption_color[3])*u16::from(element.opacity)/100) as u8;
        if element.role==InstallerTemplateRole::ProgressSegment as u8 {
            let current=progress.0<=progress.1;
            let kind=if current {Surface::Current}else{Surface::Inactive};
            let style=crate::ui::panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius);
            // Authored circular artwork is aspect-fit; explicit geometry consumes
            // the complete saved box instead of forcing a square silhouette.
            let (left,width,height)=if style.corners==0 {
                let size=rect.height.min(rect.width);
                (rect.left+(rect.width-size)/2,size,size)
            }else{(rect.left,rect.width,rect.height)};
            self.precision_surface_styled(crate::ui::geometry::Rect{x:left as i32,y:rect.top as i32,
                width:width as u32,height:height as u32},kind,crate::ui::panel_recipe::Material{
                    fill,edge:border,opacity,edge_opacity:border_opacity,focus:0,pressed:false,scale:self.ui_scale()},style);
            if progress.0<progress.1 {self.access_glyph(left+width/2,rect.top+height/2,15,width.min(height)*3/4,false);}
            return;
        }
        let kind=if element.kind==5 {
            if element.role==InstallerTemplateRole::PrimaryButton as u8 {Surface::Primary}else{Surface::Secondary}
        } else if element.role==InstallerTemplateRole::Input as u8 {Surface::Field}else{Surface::Panel};
        let style=crate::ui::panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius);
        if element.role==InstallerTemplateRole::Console as u8 {
            self.access_frost_panel_styled(rect.left,rect.top,rect.width,rect.height,fill,border,opacity,border_opacity,style);
        } else {
            self.precision_surface_styled(crate::ui::geometry::Rect{x:rect.left as i32,y:rect.top as i32,
                width:rect.width as u32,height:rect.height as u32},kind,crate::ui::panel_recipe::Material{
                    fill,edge:border,opacity,edge_opacity:border_opacity,
                    focus:if focused {palette::focus_amount()}else{0},
                    pressed:focused && palette::pressed(),scale:self.ui_scale()},style);
        }
        if element.kind==5 {
            let font=(element.font_size as usize*self.height/1000).max(6);
            let width=self.access_text_width(element.text,font,true);
            let previous=self.render_clip;
            self.intersect_render_clip(rect.left,rect.top,rect.width,rect.height);
            self.access_text(rect.left+rect.width.saturating_sub(width)/2,
                rect.top+rect.height.saturating_sub(font*7/6)/2,
                element.text,element.caption_color[0],element.caption_color[1],element.caption_color[2],text_opacity,font,true);
            self.render_clip=previous;
        }
    }

    // ------------------------=
    // FUNC: template_element
    // DESC: Converts one saved layer into framebuffer pixels using its authored geometry and appearance.
    // ------------------=
    fn template_element(
        &mut self,
        element: InstallerTemplateElement<'_>,
        image: Option<&[u8]>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        if element.role == InstallerTemplateRole::PrimaryButton as u8
            && navigation.is_some_and(|(_, available, _)| !available)
        {
            return;
        }
        let rect = scale_template_rect(element.frame, self.width, self.height);
        self.template_element_in_rect(element, image, navigation, rect);
    }

    // ------------------------=
    // FUNC: boot_template_scene
    // DESC: Composes editable boot layers with the same reveal timing, artwork, typography and progress in Studio and both native boot paths.
    // ------------------=
    pub(crate) fn boot_template_scene(&mut self, bytes: &[u8], screen: u8, percent: usize,
        label: &[u8], phase: usize) -> bool {
        let Ok(template) = InstallerTemplate::parse_boot(bytes) else { return false; };
        let Some(count) = template.element_count(screen) else { return false; };
        self.fill_rect(0,0,self.width,self.height,0,0,0);
        for index in 0..count {
            let Some(element) = template.layer_at(screen,index) else { return false; };
            if element.hidden || element.opacity == 0 { continue; }
            let rect=scale_template_rect(element.frame,self.width,self.height);
            if element.kind == 6 {
                self.template_progress_control(rect,element.fill,element.border,element.opacity,
                    element.corner_radius,element.surface_material,element.corner_style,element.font_size,
                    percent,if label.is_empty(){element.text}else{label});
            } else if element.role == InstallerTemplateRole::Masthead as u8 && element.kind == 2 && phase < 2400 {
                let saved=self.render_clip;
                for x in 0..rect.width {
                    self.render_clip=saved;
                    self.intersect_render_clip(rect.left+x,rect.top,1,rect.height);
                    let alpha=i64::from(boot_reveal::opacity(x,rect.width,phase,2400));
                    if alpha == 0 { continue; }
                    self.template_element_in_rect(InstallerTemplateElement{opacity:(i64::from(element.opacity)*alpha/255) as u8,..element},
                        template.asset(element.image_asset),None,rect);
                }
                self.render_clip=saved;
            } else {
                self.template_element_in_rect(element,template.asset(element.image_asset),None,rect);
            }
        }
        true
    }

    // ------------------------=
    // FUNC: template_element_in_rect
    // DESC: Paints an authored layer inside an explicitly mapped runtime rectangle.
    // ------------------=
    pub(crate) fn template_element_in_rect(
        &mut self,
        element: InstallerTemplateElement<'_>,
        image: Option<&[u8]>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
        rect: crate::ui::installer_layout::InstallerRect,
    ) {
        self.with_template_typography(element, |display| display.template_element_in_rect_styled(element, image, navigation, rect));
    }

    // ------------------------=
    // FUNC: template_element_in_rect_styled
    // DESC: Composes a mapped element with native typography already scoped by its caller.
    // ------------------=
    fn template_element_in_rect_styled(&mut self, element: InstallerTemplateElement<'_>, image: Option<&[u8]>,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>, rect: crate::ui::installer_layout::InstallerRect) {
        let opacity = element.opacity as u16;
        let fill_alpha = (element.fill[3] as u16 * opacity / 100) as u8;
        let border_alpha = (element.border[3] as u16 * opacity / 100) as u8;
        let radius = (element.corner_radius as usize * self.height / 1000).max(1);
        match element.kind {
            1 | 4 => self.template_surface_layer(
                element,
                rect,
                radius,
                fill_alpha,
                border_alpha,
                navigation,
            ),
            2 => self.template_image(
                element.image_asset,
                image,
                rect,
                element.crop,
                (255u16 * element.opacity as u16 / 100) as u8,
                match element.image_fit {
                    1 => false,
                    2 => true,
                    _ => template_image_uses_aspect_fill(element.role),
                },
            ),
            3 => self.template_text_layer(element, rect),
            5 => self.template_button_layer(element, rect, navigation),
            6 => self.template_progress_control(
                rect,
                element.fill,
                element.border,
                element.opacity,
                element.corner_radius,
                element.surface_material,
                element.corner_style,
                element.font_size,
                42,
                element.text,
            ),
            _ => {}
        }
    }

    // ------------------------=
    // FUNC: template_surface_layer
    // DESC: Paints UIKit glass depth for panels, consoles, fields, selectors, and badges from authored colors.
    // ------------------=
    fn template_surface_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
        radius: usize,
        fill_alpha: u8,
        border_alpha: u8,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        if element.role == InstallerTemplateRole::ProgressSegment as u8 {
            if element.surface_material!=0 || element.corner_style!=0 {
                let kind=crate::ui::panel_recipe::Surface::Panel;
                self.precision_surface_styled(crate::ui::geometry::Rect{x:rect.left as i32,y:rect.top as i32,
                    width:rect.width as u32,height:rect.height as u32},kind,crate::ui::panel_recipe::Material{
                        fill:(element.fill[0],element.fill[1],element.fill[2]),
                        edge:(element.border[0],element.border[1],element.border[2]),
                        opacity:fill_alpha,edge_opacity:border_alpha,focus:0,pressed:false,scale:self.ui_scale()},
                    crate::ui::panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius));
                return;
            }
            let bar_height = (4 * self.height / 1000).max(2).min(rect.height);
            self.fill_rounded_rect_alpha(
                rect.left,
                rect.top + rect.height.saturating_sub(bar_height) / 2,
                rect.width,
                bar_height,
                radius.min(bar_height / 2),
                element.fill[0],
                element.fill[1],
                element.fill[2],
                fill_alpha,
            );
            return;
        }
        let focused = navigation.is_some_and(|(focus, _, _)| {
            element.role == InstallerTemplateRole::Input as u8 && focus >= 2
                || element.role == InstallerTemplateRole::DateField as u8 && focus==2
                || element.role == InstallerTemplateRole::TimeField as u8 && focus==3
                || element.role == InstallerTemplateRole::TimeZoneSelector as u8 && focus==4
        });
        let is_control = element.role == InstallerTemplateRole::Input as u8
            || element.role == InstallerTemplateRole::DateField as u8
            || element.role == InstallerTemplateRole::TimeField as u8
            || element.role == InstallerTemplateRole::TimeZoneSelector as u8
            || element.role == InstallerTemplateRole::OffsetBadge as u8;
        let kind=if is_control {crate::ui::panel_recipe::Surface::Field}else{crate::ui::panel_recipe::Surface::Panel};
        let style=crate::ui::panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius);
        if element.role==InstallerTemplateRole::Console as u8 {
            self.access_frost_panel_styled(rect.left,rect.top,rect.width,rect.height,
                (element.fill[0],element.fill[1],element.fill[2]),
                (element.border[0],element.border[1],element.border[2]),fill_alpha,border_alpha,style);
            return;
        }
        self.precision_surface_styled(crate::ui::geometry::Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},
            kind,
            crate::ui::panel_recipe::Material{fill:(element.fill[0],element.fill[1],element.fill[2]),
                edge:(element.border[0],element.border[1],element.border[2]),opacity:fill_alpha,
                edge_opacity:border_alpha,focus:if focused {255}else{0},pressed:false,scale:self.ui_scale()},style);
        if is_control && !element.text.is_empty() {
            let font_px = (element.font_size as usize * self.height / 1000).max(6);
            let text_height = (font_px * 7 / 6).max(1);
            self.template_text(
                rect.left + 16 * self.ui_scale().max(1),
                rect.top + rect.height.saturating_sub(text_height) / 2,
                element.text,
                if element.role == InstallerTemplateRole::Input as u8 {
                    119
                } else {
                    241
                },
                if element.role == InstallerTemplateRole::Input as u8 {
                    133
                } else {
                    245
                },
                if element.role == InstallerTemplateRole::Input as u8 {
                    149
                } else {
                    250
                },
                255,
                font_px,
                false,
            );
        }
    }

    // ------------------------=
    // FUNC: template_image
    // DESC: Resolves packaged editor image assets and paints them into their authored frame.
    // ------------------=
    pub(super) fn template_image(
        &mut self,
        name: &[u8],
        packaged: Option<&[u8]>,
        rect: crate::ui::installer_layout::InstallerRect,
        crop: [u8; 4],
        opacity: u8,
        aspect_fill: bool,
    ) {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            #[cfg(studio_native_preview)]
            let packaged=crate::studio_native_render::asset_override(name).or(packaged);
            let bitmap = if let Some(bitmap) = packaged {
                bitmap
            } else if name.ends_with(b"infinity-installer-background-v2.png") {
                #[cfg(any(feature = "installer", studio_native_preview))]
                { include_bytes!("../../../assets/boot/infinity-installer-background-v2.bmp") }
                #[cfg(not(any(feature = "installer", studio_native_preview)))]
                { &[] }
            } else if name.ends_with(b"infinity-installer-masthead-v2.png") {
                INSTALLER_MASTHEAD_BMP
            } else if name.ends_with(b"infinity-installer-masthead-v1.png") {
                INSTALLER_WELCOME_MASTHEAD_BMP
            } else if name.ends_with(b"infinity-onboarding-wallpaper-v1.png") {
                include_bytes!("../../../assets/desktop/infinity-onboarding-wallpaper-v1.bmp")
            } else if name.ends_with(b"eclipse-access-wallpaper-v1.png") {
                ONBOARDING_BMP
            } else if name.ends_with(b"infinity-eclipse-header-v1.bmp") {
                super::bootstrap::SPLASH_BMP
            } else if name.ends_with(b"infinity-time-zone-map-v1.png") {
                super::installer::DATE_TIME_WORLD_BMP
            } else if name.ends_with(b"infinity-installer-progress-hero-v1.png") {
                INSTALLER_PROGRESS_HERO_BMP
            } else {
                &[]
            };
            if !bitmap.is_empty() {
                self.paint_bitmap_template_rect(
                    bitmap,
                    rect.left,
                    rect.top,
                    rect.width,
                    rect.height,
                    crop,
                    opacity,
                    aspect_fill,
                );
            }
        }
    }

    // ------------------------=
    // FUNC: template_progress_control
    // DESC: Draws an authored UIKit progress control with a live label and percentage.
    // ------------------=
    pub(super) fn template_progress_control(
        &mut self,
        rect: crate::ui::installer_layout::InstallerRect,
        fill: [u8; 4],
        border: [u8; 4],
        opacity: u8,
        corner_radius: u8,
        surface_material: u8,
        corner_style: u8,
        font_size: u16,
        percent: usize,
        label: &[u8],
    ) {
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        let alpha = |value: u8| (value as u16 * opacity as u16 / 100) as u8;
        let inset = (18 * self.height / 1000).max(1).min(rect.width / 4);
        let pixels = (usize::from(font_size) * self.height / 1000).max(6);
        let gap = 14 * self.height / 1000;
        let track_height = (16 * self.height / 1000).max(1);
        let line_height = pixels * 28 / 24;
        let group_height = line_height + gap + track_height;
        let top = rect.top + rect.height.saturating_sub(group_height) / 2;
        let track_top = top + line_height + gap;
        let track_width = rect.width.saturating_sub(inset * 2);
        use crate::ui::{geometry::Rect, panel_recipe::{Material,Surface}};
        let material=Material{fill:super::access_style::SURFACE,edge:(border[0],border[1],border[2]),
            opacity:alpha(240),edge_opacity:alpha(border[3]),focus:0,pressed:false,scale:self.ui_scale()};
        self.precision_surface_styled(Rect{x:rect.left as i32,y:rect.top as i32,
            width:rect.width as u32,height:rect.height as u32},Surface::Panel,material,
            crate::ui::panel_recipe::template_style(Surface::Panel,surface_material,corner_style,corner_radius));
        let mut percent_text = [b'0'; 4];
        let value = percent.min(100);
        let digits = if value == 100 {
            percent_text[..3].copy_from_slice(b"100");
            3
        } else if value >= 10 {
            percent_text[0] += (value / 10) as u8;
            percent_text[1] += (value % 10) as u8;
            2
        } else {
            percent_text[0] += value as u8;
            1
        };
        percent_text[digits] = b'%';
        let percent_slice = &percent_text[..digits + 1];
        let percent_width = self.template_text_width(percent_slice, pixels, true);
        self.template_text(rect.left + rect.width.saturating_sub(inset + percent_width), top,
            percent_slice, fill[0], fill[1], fill[2], alpha(fill[3]), pixels, true);
        self.intersect_render_clip(rect.left + inset, top,
            track_width.saturating_sub(percent_width + inset), line_height);
        self.template_text(rect.left + inset, top, label, 241, 245, 250, alpha(255), pixels, true);
        self.render_clip = previous_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        let track=Rect{x:(rect.left+inset) as i32,y:track_top as i32,
            width:track_width as u32,height:track_height as u32};
        self.precision_surface_styled(track,Surface::Field,
            Material{fill:super::access_style::BACKGROUND,opacity:alpha(199),..material},
            crate::ui::panel_recipe::template_style(Surface::Field,surface_material,corner_style,corner_radius));
        self.precision_surface_styled(Rect{width:(track_width*value/100) as u32,..track},Surface::Primary,
            Material{fill:(fill[0],fill[1],fill[2]),opacity:alpha(fill[3]),..material},
            crate::ui::panel_recipe::template_style(Surface::Primary,surface_material,corner_style,corner_radius));
        self.render_clip = previous_clip;
    }

    // ------------------------=
    // FUNC: template_text_layer
    // DESC: Paints authored text, color, size, opacity, wrapping, and clipping within its saved frame.
    // ------------------=
    fn template_text_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
    ) {
        let alpha = (element.fill[3] as u16 * element.opacity as u16 / 100) as u8;
        let previous_clip = self.render_clip;
        self.intersect_render_clip(rect.left, rect.top, rect.width, rect.height);
        self.template_text_wrapped(
            rect.left,
            rect.top,
            rect.width,
            element.text,
            element.fill[0],
            element.fill[1],
            element.fill[2],
            alpha,
            element.font_size as usize * self.height / 1000,
            element.role == InstallerTemplateRole::Title as u8,
        );
        self.render_clip = previous_clip;
    }

    // ------------------------=
    // FUNC: template_button_layer
    // DESC: Paints an authored button and overlays its live focus state without changing saved styling.
    // ------------------=
    fn template_button_layer(
        &mut self,
        element: InstallerTemplateElement<'_>,
        rect: crate::ui::installer_layout::InstallerRect,
        navigation: Option<(usize, bool, Option<(i32, i32, bool)>)>,
    ) {
        let focused = navigation.is_some_and(|(focus, _, _)| {
            (element.role == InstallerTemplateRole::BackButton as u8 && focus == 0)
                || (element.role == InstallerTemplateRole::PrimaryButton as u8 && focus == 1)
        });
        let pointer = navigation.and_then(|(_, _, pointer)| pointer);
        let hovered = pointer.is_some_and(|(x, y, _)| {
            x >= element.frame.x as i32
                && x <= (element.frame.x + element.frame.width) as i32
                && y >= element.frame.y as i32
                && y <= (element.frame.y + element.frame.height) as i32
        });
        let depressed = pointer.is_some_and(|(_, _, is_pressed)| is_pressed) && hovered;
        let primary = element.role == InstallerTemplateRole::PrimaryButton as u8;
        self.paint_template_button_state(element, crate::ui::geometry::Rect {
            x: rect.left as i32, y: rect.top as i32,
            width: rect.width as u32, height: rect.height as u32,
        }, primary, focused || hovered, depressed);
    }

    // ------------------------=
    // FUNC: paint_template_button_state
    // DESC: Paints authored buttons through the same image-backed surface and native text renderer in screen and Settings contexts.
    // ------------------=
    pub(crate) fn paint_template_button_state(
        &mut self, element: InstallerTemplateElement<'_>, rect: crate::ui::geometry::Rect,
        primary: bool, focused: bool, depressed: bool,
    ) {
        self.with_template_typography(element, |display| display.paint_template_button_state_styled(element, rect, primary, focused, depressed));
    }

    // ------------------------=
    // FUNC: paint_template_button_state_styled
    // DESC: Paints the native button and centered caption with its authored font and coverage effects.
    // ------------------=
    fn paint_template_button_state_styled(&mut self, element: InstallerTemplateElement<'_>, rect: crate::ui::geometry::Rect,
        primary: bool, focused: bool, depressed: bool) {
        if element.hidden || element.opacity == 0 || rect.width == 0 || rect.height == 0 { return; }
        let previous_clip = self.render_clip;
        let visible = rect.intersection(crate::ui::geometry::Rect {
            x: 0, y: 0, width: self.width as u32, height: self.height as u32,
        });
        if visible.width == 0 || visible.height == 0 { return; }
        self.intersect_render_clip(visible.x as usize, visible.y as usize, visible.width as usize, visible.height as usize);
        let opacity = element.opacity as u16;
        use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
        let kind=if primary {Surface::Primary}else{Surface::Secondary};
        let style=crate::ui::panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius);
        self.precision_surface_styled(Rect{..rect},
            kind,Material{
                fill:(element.fill[0],element.fill[1],element.fill[2]),
                edge:(element.border[0],element.border[1],element.border[2]),
                opacity:(element.fill[3] as u16*opacity/100) as u8,
                edge_opacity:(element.border[3] as u16*opacity/100) as u8,
                focus:if focused {
                    if element.role==InstallerTemplateRole::SettingsRowAction as u8 {100}
                    else {super::access_style::focus_amount()}
                }else{0},
                pressed:depressed,scale:self.ui_scale()},style);
        let font_px = (element.font_size as usize * self.height / 1000).max(6);
        let text_width = self.template_text_width(element.text, font_px, true);
        let text_height = (font_px * 7 / 6).max(1);
        let text_left = i64::from(rect.x) + (rect.width as usize).saturating_sub(text_width) as i64 / 2;
        let text_top = i64::from(rect.y) + (rect.height as usize).saturating_sub(text_height) as i64 / 2
                + if depressed {
                    (2 * self.ui_scale().max(1)) as i64
                } else {
                    0
                };
        // The text renderer takes unsigned origins; crop the left/top glyph runs through its signed-origin adapter.
        self.template_text_signed(
            text_left as i32,
            text_top as i32,
            element.text,
            element.caption_color[0],
            element.caption_color[1],
            element.caption_color[2],
            (u16::from(element.caption_color[3]) * opacity / 100) as u8,
            font_px,
            true,
        );
        self.render_clip = previous_clip;
    }
}
