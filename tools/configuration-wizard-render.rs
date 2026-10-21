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
#[path="../kernel/core/bootstrap/installer_datetime_view.rs"] mod installer_datetime_view;

mod bootstrap {
    pub const SPLASH_BMP: &[u8] = include_bytes!("../assets/boot/infinity-eclipse-header-v1.bmp");
}
mod desktop {
    pub const ONBOARDING_BMP: &[u8] = include_bytes!("../assets/ui-design-kit/default/eclipse-access-wallpaper-v1.bmp");
}
mod installer {
    pub(super) use super::super::installer_text_view::*;
    pub const DATE_TIME_WORLD_BMP: &[u8] = include_bytes!("../assets/boot/infinity-time-zone-map-v1.bmp");
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
        (Role::Input,Surface::Field)] {
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
// FUNC: frost_pixels
// DESC: Exercises actual outer-panel composition for backdrop transmission, blur, alpha and unchanged exterior pixels.
// ------------------=
fn frost_pixels() {
    let (w,h)=(400,300);
    let mut a=vec![0x101010;w*h];let mut b=vec![0x909090;w*h];
    for pixels in [&mut a,&mut b] {
        display(pixels,w,h).access_frost_panel(20,20,360,260,(16,27,58),(134,199,255),235,255);
    }
    assert_ne!(a[150*w+200],b[150*w+200]);
    assert_eq!(a[0],0x101010);assert_eq!(b[0],0x909090);
    let mut patterned=(0..w*h).map(|at|if (at%w+at/w)%2==0 {0xffffff}else{0}).collect::<Vec<_>>();
    let original=patterned.clone();
    display(&mut patterned,w,h).access_frost_panel(20,20,360,260,(16,27,58),(134,199,255),235,255);
    assert!((patterned[150*w+200]&255).abs_diff(patterned[150*w+201]&255)<3);
    let mut clear=original.clone();
    display(&mut clear,w,h).access_frost_panel(20,20,360,260,(16,27,58),(134,199,255),0,0);
    assert_eq!(clear,original);
}

// ------------------------=
// FUNC: assert_partial_pixels
// DESC: Compares reconstructed pixels with a clean full paint and proves all preexisting exterior foreground pixels remain untouched.
// ------------------=
fn assert_partial_pixels(actual:&[u32],expected:&[u32],before:&[u32],width:usize,clip:Region) {
    for (at,pixel) in actual.iter().enumerate() {
        let (x,y)=(at%width,at/width);
        let inside=x>=clip.left && x<clip.right && y>=clip.top && y<clip.bottom;
        assert_eq!(*pixel,if inside {expected[at]}else{before[at]},"partial pixel ({x},{y})");
    }
}

// ------------------------=
// FUNC: retained_frost_pixels
// DESC: Verifies full-source blur reuse over restored damage, exterior ink preservation, and refresh after background or framebuffer geometry changes.
// ------------------=
fn retained_frost_pixels() {
    for (w,h) in [(400,300),(517,337)] {
        let mut actual=vec![0;w*h];
        for variant in 0..2usize {
            let source=(0..w*h).map(|at|{
                let (x,y)=(at%w,at/w);
                let r=((x*19+y*3+variant*97)%256) as u32;
                let g=((x/9+y/7+variant)%2*180+20) as u32;
                let b=((x*2+y*23+variant*31)%256) as u32;
                r|(g<<8)|(b<<16)
            }).collect::<Vec<_>>();
            let mut expected=source.clone();
            display(&mut expected,w,h).access_frost_panel(20,20,w-40,h-40,(16,27,58),(134,199,255),218,255);
            actual.copy_from_slice(&source);
            display(&mut actual,w,h).access_frost_panel(20,20,w-40,h-40,(16,27,58),(134,199,255),218,255);
            let clip=Region{left:31,top:35,right:w/2+13,bottom:h/3+17};
            actual[(h-29)*w+w-29]=0x00ff21;
            for _ in 0..2 {
                let before=actual.clone();
                for y in clip.top..clip.bottom {for x in clip.left..clip.right {
                    actual[y*w+x]=source[y*w+x];
                }}
                let mut target=display(&mut actual,w,h);target.render_clip=Some(clip);
                target.access_frost_panel(20,20,w-40,h-40,(16,27,58),(134,199,255),218,255);
                assert_partial_pixels(&actual,&expected,&before,w,clip);
            }
        }
    }
}

// ------------------------=
// FUNC: partial_scene_pixels
// DESC: Replays completed configuration field and installer progress updates against full production frames without changing pixels outside caller damage.
// ------------------=
fn partial_scene_pixels() {
    let (w,h)=(1280,800);
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    for step in [1usize,4] {
        access_style::animate(((step as u32+1)<<8)|1,1000,false,true);
        let input=template.element(step as u8+1,Role::Input).unwrap();
        let r=scale_template_rect(input.frame,w,h);
        let clip=Region{left:r.left,top:r.top,right:r.left+r.width,bottom:r.top+r.height};
        let mut expected=vec![0;w*h];
        let mut target=display(&mut expected,w,h);
        assert!(target.configuration_template_screen(step));
        target.onboarding_input_field(step,r.left,r.top,r.width,r.height,b"Updated account",step==4,true,input.text);
        assert!(target.configuration_template_navigation(step,1));
        let mut actual=vec![0;w*h];
        let mut target=display(&mut actual,w,h);
        assert!(target.configuration_template_screen(step));
        target.onboarding_input_field(step,r.left,r.top,r.width,r.height,b"Old account",step==4,true,input.text);
        assert!(target.configuration_template_navigation(step,1));
        let card=scale_template_rect(template.element(step as u8+1,Role::Console).unwrap().frame,w,h);
        actual[(card.top+8)*w+card.left+8]=0x00ff21;
        let before=actual.clone();
        let mut target=display(&mut actual,w,h);target.render_clip=Some(clip);
        assert!(target.configuration_template_screen(step));
        target.onboarding_input_field(step,r.left,r.top,r.width,r.height,b"Updated account",step==4,true,input.text);
        assert_partial_pixels(&actual,&expected,&before,w,clip);
    }
    let template=InstallerTemplate::parse(crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES).unwrap();
    let r=scale_template_rect(template.element(8,Role::ProgressBar).unwrap().frame,w,h);
    let clip=Region{left:r.left,top:r.top,right:r.left+r.width,bottom:r.top+r.height};
    let wallpaper=include_bytes!("../assets/boot/infinity-installer-background-v2.bmp");
    let mut expected=vec![0;w*h];
    let mut target=display(&mut expected,w,h);
    target.paint_bitmap_cover_rect(wallpaper,0,0,w,h);
    assert!(target.installer_template_progress_screen(73,b"Verifying installation"));
    let mut actual=vec![0;w*h];
    let mut target=display(&mut actual,w,h);
    target.paint_bitmap_cover_rect(wallpaper,0,0,w,h);
    assert!(target.installer_template_progress_screen(12,b"Copying installation"));
    let card=scale_template_rect(template.element(8,Role::Console).unwrap().frame,w,h);
    actual[(card.top+8)*w+card.left+8]=0x00ff21;
    let before=actual.clone();
    let mut target=display(&mut actual,w,h);target.render_clip=Some(clip);
    target.paint_bitmap_cover_rect(wallpaper,0,0,w,h);
    assert!(target.installer_template_progress_screen(73,b"Verifying installation"));
    assert_partial_pixels(&actual,&expected,&before,w,clip);
}

// ------------------------=
// FUNC: authored_button_pixels
// DESC: Verifies compiled installer and setup palettes through real rendered button interiors at each interaction state.
// ------------------=
fn authored_button_pixels(bytes:&[u8],screens:u8) {
    let template=InstallerTemplate::parse(bytes).unwrap();
    for screen in 1..=screens {
        for role in [Role::PrimaryButton,Role::BackButton] {
            let Some(mut element)=template.element(screen,role).filter(|e|!e.hidden && e.opacity>0) else{continue;};
            element.text=b"";
            for (focus,pressed) in [(2,false),(1,false),(1,true)] {
                access_style::animate(1,1000,pressed,true);
                let (w,h)=(1280,800);let mut pixels=vec![0;w*h];
                let r=scale_template_rect(element.frame,w,h);
                display(&mut pixels,w,h).template_element_in_rect(element,None,
                    Some((focus,true,Some((element.frame.x as i32+1,element.frame.y as i32+1,pressed)))),r);
                let p=pixels[(r.top+r.height/2)*w+r.left+r.width/2];
                let red=p&255;let green=(p>>8)&255;let blue=(p>>16)&255;
                assert!(blue>green+12 && blue>red+12,"non-blue control: {screen} {role:?} {p:06x}");
                if role==Role::BackButton {assert!(blue<100 && green<65,"secondary too bright");}
            }
        }
    }
}

// ------------------------=
// FUNC: installer_scenes
// DESC: Captures every authored installation page through production rendering, including focused and pressed review actions.
// ------------------=
fn installer_scenes(output:&str) {
    use crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES;
    authored_button_pixels(INSTALLER_TEMPLATE_BYTES,11);
    let template=InstallerTemplate::parse(INSTALLER_TEMPLATE_BYTES).unwrap();
    for (w,h) in [(1280,800),(1920,1080)] {for screen in 1..=11u8 {
        for state in 0..if screen==6 {3}else{1} {
            let pressed=state==2;access_style::animate(1,1000,pressed,true);
            let mut pixels=vec![0;w*h];let mut target=display(&mut pixels,w,h);
            target.paint_bitmap_cover_rect(include_bytes!("../assets/boot/infinity-installer-background-v2.bmp"),0,0,w,h);
            if screen==5 {
                let layout=crate::ui::installer_layout::installer_wizard_layout(5,w,h);
                let m=layout.masthead;
                target.paint_bitmap_fit_rect(installer::INSTALLER_MASTHEAD_BMP,m.left,m.top,m.width,m.height);
                let r=layout.panel;
                target.installer_date_time_panel(r.left,r.top,r.width,r.height,
                    crate::storage::DateTimeConfiguration{year:2026,month:10,day:21,hour:11,minute:25,second:0,
                        time_zone_id:3,utc_offset_minutes:-360},1,2,125,500,false);
            }
            else if screen==8 {assert!(target.installer_template_progress_screen(73,b"Verifying installation"));}
            else {assert!(target.installer_template_screen_with_details(screen,
                b"Disk: VirtualBox UEFI Virtual Disk\nCreates: EFI boot area + Infinity Container\nPool areas: System | Personal | Applications | Recovery\nLocal time: 2026-10-21 05:33\nTime zone: UTC+00:00 Universal"));}
            let element=template.element(screen,Role::PrimaryButton);
            let (x,y)=element.map(|e|(e.frame.x as i32+1,e.frame.y as i32+1)).unwrap_or((-1,-1));
            assert!(target.installer_template_navigation(screen,if state==0 {2}else{1},true,
                if state==0 {-1}else{x},if state==0 {-1}else{y},pressed));
            write_ppm(&format!("{output}/installer-{screen}-state-{state}-{w}x{h}.ppm"),&pixels,w,h);
        }
    }}
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
    frost_pixels();
    retained_frost_pixels();
    partial_scene_pixels();
    authored_button_pixels(CONFIGURATION_TEMPLATE_BYTES,8);
    installer_scenes(output);
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
        "{{\"semantic_control_pixel_comparisons\":{comparisons},\"configuration_scene_pixels\":{screen_pixels},\"configuration_scenes\":14,\"installer_template_scenes\":26,\"native_compositor\":true,\"native_fonts\":true,\"installed_verified\":false,\"dynamic_network_scene_verified\":false}}\n")).unwrap();
}
