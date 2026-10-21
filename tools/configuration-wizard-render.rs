//! Host framebuffer proof of the actual authored first-boot compositor and native Inter painters.
use super::{DisplayDevice, Region};
use super::primitives::FontSize;
use crate::ui::{geometry::Rect, installer_layout::{CONFIGURATION_TEMPLATE_BYTES, scale_template_rect},
    installer_template::{InstallerTemplate, InstallerTemplateRole as Role},
    panel_recipe::{Material, Surface}};

#[path="../kernel/core/bootstrap/access_style.rs"] mod access_style;
#[path="../kernel/core/bootstrap/access_view.rs"] mod access_view;
#[path="../kernel/core/bootstrap/access_input_view.rs"] mod access_input_view;
#[path="../kernel/core/bootstrap/template_font.rs"] mod template_font;
#[path="../kernel/core/bootstrap/template_text_view.rs"] mod template_text_view;
#[path="../kernel/core/bootstrap/template_compositor.rs"] mod template_compositor;

mod bootstrap {
    pub const SPLASH_BMP: &[u8] = include_bytes!("../assets/boot/infinity-eclipse-header-v1.bmp");
}
mod desktop {
    pub const ONBOARDING_BMP: &[u8] = include_bytes!("../assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp");
}
mod installer {
    pub const INSTALLER_FONT_NATIVE_SIZE_PX: usize = 24;
    pub const INSTALLER_FONT_CELL_WIDTH: usize = 24;
    pub const INSTALLER_FONT_CELL_HEIGHT: usize = 28;
    pub const INSTALLER_FONT_ATLAS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.atlas");
    pub const INSTALLER_FONT_SEMIBOLD_ATLAS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.atlas");
    pub const INSTALLER_FONT_METRICS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.metrics");
    pub const INSTALLER_FONT_SEMIBOLD_METRICS: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.metrics");
    pub const INSTALLER_FONT_KERN: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Regular-24.kern");
    pub const INSTALLER_FONT_SEMIBOLD_KERN: &[u8] = include_bytes!("../assets/fonts/InfinityInstaller-Semibold-24.kern");
    pub const INSTALLER_MASTHEAD_BMP: &[u8] = include_bytes!("../assets/boot/infinity-installer-masthead-v2.bmp");
    pub const INSTALLER_WELCOME_MASTHEAD_BMP: &[u8] = include_bytes!("../assets/boot/infinity-installer-masthead-v1.bmp");
    pub const INSTALLER_PROGRESS_HERO_BMP: &[u8] = include_bytes!("../assets/boot/infinity-installer-progress-hero-v1.bmp");
}

// ------------------------=
// FUNC: display
// DESC: Supplies only a memory framebuffer transport to unchanged production display methods.
// ------------------=
fn display(pixels: &mut [u32], width: usize, height: usize) -> DisplayDevice {
    DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:None,
        fast_motion_frame:false,submissions:0,recording_surface:false}
}

// ------------------------=
// FUNC: role_pixels
// DESC: Checks actual template-control pixels against explicit semantic surfaces despite arbitrary colors and extreme aspect ratios.
// ------------------=
fn role_pixels() -> usize {
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let mut comparisons=0;
    access_style::animate(0x400,0,false,true);
    for (role,kind) in [(Role::BackButton,Surface::Secondary),(Role::PrimaryButton,Surface::Primary),
        (Role::Input,Surface::Field),(Role::Console,Surface::Panel)] {
        for (w,h) in [(280,38),(70,100)] {
            let mut element=template.element(5,role).unwrap();
            element.frame=crate::ui::installer_template::InstallerTemplateRect{x:20,y:20,width:w,height:h};
            element.fill=[42,71,102,219];element.border=[134,199,255,255];
            element.opacity=100;element.text=b"";
            let rect=scale_template_rect(element.frame,400,300);
            let mut actual=vec![0x251407;400*300];
            let mut expected=actual.clone();
            display(&mut actual,400,300).access_template_control(element,None,(0,0));
            display(&mut expected,400,300).precision_surface(Rect{x:rect.left as i32,y:rect.top as i32,
                width:rect.width as u32,height:rect.height as u32},kind,Material{fill:(42,71,102),
                edge:(134,199,255),opacity:219,edge_opacity:255,focus:0,pressed:false,scale:1});
            assert_eq!(actual,expected);
            comparisons+=actual.len();
        }
    }
    comparisons
}

// ------------------------=
// FUNC: write_ppm
// DESC: Records untouched production framebuffer pixels for human visual review.
// ------------------=
fn write_ppm(path:&str,pixels:&[u32],width:usize,height:usize) {
    let mut bytes=format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in pixels {bytes.extend_from_slice(&[(*pixel&255) as u8,((*pixel>>8)&255) as u8,((*pixel>>16)&255) as u8]);}
    std::fs::write(path,bytes).unwrap();
}

// ------------------------=
// FUNC: verify
// DESC: Renders authored first-boot scenes with actual compositor, bitmap sampling, Inter, image-backed controls and semantic state assertions.
// ------------------=
pub(super) fn verify() {
    let output="builds/validation/eclipse-precision/wizard";
    std::fs::create_dir_all(output).unwrap();
    let comparisons=role_pixels();
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let mut screen_pixels=0;
    for (width,height) in [(1280,800),(1920,1080)] {
        for step in [0usize,1,2,3,4,5,7] {
            access_style::animate(((step as u32+1)<<8)|1,1000,false,true);
            let mut pixels=vec![0; width*height];
            let mut target=display(&mut pixels,width,height);
            assert!(target.configuration_template_screen(step));
            if let Some(input)=template.element(step as u8+1,Role::Input).filter(|e|!e.hidden) {
                let r=scale_template_rect(input.frame,width,height);
                target.onboarding_input_field(step,r.left,r.top,r.width,r.height,b"",step==4,false,input.text);
            }
            assert!(target.configuration_template_navigation(step,1));
            if let Some(card)=template.element(step as u8+1,Role::Console) {
                let r=scale_template_rect(card.frame,width,height);
                target.access_panel_glow(r.left,r.top,r.width,r.height);
            }
            assert!(pixels.iter().filter(|p|**p!=0).count()>width*height*9/10);
            write_ppm(&format!("{output}/step-{step}-{width}x{height}.ppm"),&pixels,width,height);
            screen_pixels+=pixels.len();
        }
    }
    std::fs::write(format!("{output}/evidence.json"),format!(
        "{{\"semantic_control_pixel_comparisons\":{comparisons},\"production_scene_pixels\":{screen_pixels},\"scenes\":14,\"native_compositor\":true,\"native_fonts\":true,\"installed_verified\":false,\"dynamic_network_scene_verified\":false}}\n")).unwrap();
}
