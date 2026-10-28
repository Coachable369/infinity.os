//! Host framebuffer proof of the actual authored first-boot compositor and native Inter painters.
use super::{DisplayDevice, Region};
use super::primitives::FontSize;
use crate::ui::{geometry::Rect, installer_layout::{CONFIGURATION_TEMPLATE_BYTES, scale_template_rect},
    installer_template::{InstallerTemplate, InstallerTemplateRole as Role},
    panel_recipe::{Material, Surface}};

#[path="../kernel/core/bootstrap/access_style.rs"] mod access_style;
#[path="../kernel/core/bootstrap/access_view.rs"] mod access_view;
#[path="../kernel/core/bootstrap/access_input_view.rs"] mod access_input_view;
#[path="../kernel/core/bootstrap/face_setup_view.rs"] mod face_setup_view;
#[path="../kernel/core/bootstrap/face_scan_view.rs"] mod face_scan_view;
#[cfg(studio_native_preview)]
#[path="../kernel/core/bootstrap/access_scale.rs"] mod access_scale;
#[cfg(studio_native_preview)]
#[path="../kernel/core/bootstrap/access_panel.rs"] mod access_panel;
#[path="../kernel/core/bootstrap/onboarding_network_view.rs"] mod onboarding_network_view;
#[path="../kernel/core/bootstrap/template_font.rs"] mod template_font;
#[path="../kernel/core/bootstrap/template_text_view.rs"] mod template_text_view;
#[path="../kernel/core/bootstrap/template_compositor.rs"] mod template_compositor;
#[path="../kernel/core/bootstrap/installer_datetime_view.rs"] mod installer_datetime_view;
#[path="../kernel/core/bootstrap/installer_confirmation_view.rs"] mod installer_confirmation_view;

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
        fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false}
}

// ------------------------=
// FUNC: verify_face
// DESC: Renders unchanged native enrollment painters with no camera frame and verifies bounded animated damage and static reduced motion.
// ------------------=
pub(super) fn verify_face() {
    verify_face_capture_feedback();
    face_scan_view::verify_rigid_geometry();
    let (width,height)=(1920,1080);
    let mut pixels=vec![0x00090f26;width*height];
    let mut d=display(&mut pixels,width,height);
    let state=crate::runtime::face_setup::Setup::new();
    d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
    d.face_setup_frame_state(b"Everyday",1,false,&state,true);
    let base=pixels.clone();let mask=crate::ui::face_setup_layout::Layout::new(width,height).mask;
    d.face_mask_scan(mask,700,false);let first=pixels.clone();
    pixels.copy_from_slice(&base);d.face_mask_scan(mask,1400,false);
    assert_ne!(pixels,first);
    for y in 0..height {for x in 0..width {
        if !mask.contains(crate::ui::geometry::Point{x:x as i32,y:y as i32}) {
            assert_eq!(pixels[y*width+x],base[y*width+x]);
        }
    }}
    std::fs::create_dir_all("builds/validation/face-enrollment").unwrap();
    write_ppm("builds/validation/face-enrollment/native-frame.ppm",&pixels,width,height);
    for (name,t) in [("front",0),("right",3000),("left",9000)] {
        d.face_mask_scan(mask,t,false);
        write_ppm(&format!("builds/validation/face-enrollment/{name}.ppm"),&pixels,width,height);
    }
    pixels.copy_from_slice(&base);d.face_mask_scan(mask,700,true);let reduced=pixels.clone();
    pixels.copy_from_slice(&base);d.face_mask_scan(mask,1400,true);assert_eq!(pixels,reduced);
    // Adjacent frames must move less than a substantial turn, including the yaw wrap.
    let start=std::time::Instant::now();
    for t in [0,2467,4967,7467,9967] {
        d.face_mask_scan(mask,t,false);let a=pixels.clone();
        d.face_mask_scan(mask,t+33,false);let b=pixels.clone();
        d.face_mask_scan(mask,t+1000,false);
        let distance=|other:&[u32]|->u64 {a.iter().zip(other).map(|(&x,&y)| {
            [0,8,16].into_iter().map(|s|((x>>s)&255).abs_diff((y>>s)&255) as u64).sum::<u64>()
        }).sum()};
        assert!(distance(&b)<distance(&pixels));
    }
    println!("15 native animation frames plus pixel comparisons: {:?}",start.elapsed());
    // Camera/status repaints must not publish a front-facing frame between animation ticks.
    for pose in [700,3000,6100,9000] {
        d.face_mask_scan(mask,pose,false);let expected=pixels.clone();
        for _ in 0..3 {
            d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
            d.face_setup_frame_state(b"Everyday",1,false,&state,true);
            assert_eq!(pixels,expected,"full repaint changed the visible face pose");
        }
    }
}

// ------------------------=
// FUNC: verify_face_capture_feedback
// DESC: Checks production thumbnail pixels through capture, analysis, rejection and acceptance; synthetic RGB is not recognition evidence.
// ------------------=
fn verify_face_capture_feedback() {
    use crate::runtime::face_setup::{Setup,PhotoStatus,PREVIEW_BYTES,wire as p};
    for (width,height) in [(1280,800),(1920,1080)] {
        let mut pixels=vec![0;width*height];let mut d=display(&mut pixels,width,height);
        let l=crate::ui::face_setup_layout::Layout::new(width,height);
        let left=l.preview.x as usize+l.preview.width as usize+l.pixels;
        let slot_width=(l.field.x as usize+l.field.width as usize-left-l.pixels*2)/3;
        let center=(l.preview.y as usize+slot_width*3/8)*width+left+slot_width/2;
        let mut state=Setup::new();state.begin(1,2,[3;32],"Everyday",100).unwrap();
        state.accept(p::Report::new(state.control().command_id,p::READY,0,0,0),101).unwrap();
        state.capture(102).unwrap();
        d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
        d.face_setup_frame_state(b"Everyday",1,false,&state,true);
        assert_ne!(pixels[center],0x002a2a2a);
        for attempt in 0..2 {
            let id=state.control().command_id;
            for offset in (0..PREVIEW_BYTES).step_by(3840) {state.write_preview(id,offset,&[42;3840],103+attempt*10).unwrap();}
            assert_eq!(state.photo_status(0),PhotoStatus::Checking);
            d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
            d.face_setup_frame_state(b"Everyday",1,false,&state,true);
            assert_eq!(pixels[center],0x002a2a2a);
            if attempt==0 {
                state.accept(p::Report::new(id,p::REJECTED,214,0,0),104).unwrap();
                d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
                d.face_setup_frame_state(b"Everyday",1,false,&state,true);
                assert_ne!(pixels[center],0x002a2a2a);
                state.capture(112).unwrap();
            } else {
                state.accept(p::Report::new(id,p::READY,0,1,0),114).unwrap();
                d.paint_bitmap_cover_rect(desktop::ONBOARDING_BMP,0,0,width,height);
                d.face_setup_frame_state(b"Everyday",1,false,&state,true);
                assert_eq!(state.photo_status(0),PhotoStatus::Accepted);
                assert_eq!(pixels[center],0x002a2a2a);
            }
        }
    }
}

#[derive(Clone,Copy)]
pub(crate) struct DraftServiceState {
    pub simulated:bool,pub confirmation:bool,pub percent:u8,pub zone:u8,pub date_focus:u8,pub date_part:u8,
    pub expanded:Option<usize>,pub scroll:usize,pub control:usize,
}
impl Default for DraftServiceState {
    // ------------------------=
    // FUNC: default
    // DESC: Defines explicitly simulated service values; preview rendering never reads or changes clock, identity or installation services.
    // ------------------=
    fn default()->Self {Self{simulated:false,confirmation:false,percent:42,zone:3,date_focus:2,date_part:0,
        expanded:None,scroll:0,control:0}}
}

// ------------------------=
// FUNC: dashboard_fixture
// DESC: Supplies visibly simulated immutable dashboard values without creating network or node services in the renderer.
// ------------------=
pub(crate) fn dashboard_fixture(section:usize)->Option<crate::settings_dashboard_view::DashboardSnapshot> {
    use crate::settings_dashboard_view::*;
    match section {
        6=>Some(DashboardSnapshot::Network(NetworkDashboardSnapshot{
            status:NetworkStatusSnapshot{active_profile:1,link_only:false,interfaces:1,addresses:1,routes:1,policies:2},
            active_connections:3,setup:NetworkSetupSnapshot{selected:SetupMode::Automatic,wired_available:true,wireless_available:false},
            interface:Some(NetworkInterfaceSnapshot{enabled:true,link:LinkState::Up,hardware_address:true,maximum_frame_size:1500}),
            static_address:Some(AddressSnapshot{address:Some([192,0,2,10]),prefix_length:24}),
            default_route:Some(RouteSnapshot{next_hop:Some([192,0,2,1]),metric:100,is_static:true}),
            resolver_enabled:true,resolver_primary:Some([192,0,2,53]),resolver_secondary:None,default_policy:PolicyAction::Ask})),
        7=>Some(DashboardSnapshot::Nodes(NodeDashboardSnapshot{identity_ready:true,counts:[3,2,1,1,2,1,2,4],selected:true,
            verification:Some(NodeVerificationSnapshot{code:123456,fingerprint:[0x5a;32],remaining:90,state:VerificationState::Local}),
            policy_offset:0,policy:Some([0;12])})),
        _=>None,
    }
}

// ------------------------=
// FUNC: settings_preview_window
// DESC: Derives a simulated native Settings window from the draft console and its authored row count.
// ------------------=
#[cfg(studio_native_preview)]
fn settings_preview_window(template:&InstallerTemplate<'_>,screen:u8,state:DraftServiceState)->Result<crate::ui::system_layout::SettingsWindowState,String> {
    let console=template.element(screen,Role::Console).ok_or("Settings preview requires a Console role")?;
    let rows=template.element_count(screen).unwrap_or(0);
    let row_count=(0..rows).filter(|index|template.element_at(screen,*index)
        .is_some_and(|element|element.role==Role::SettingsRowLabel as u8)).count();
    Ok(crate::ui::system_layout::SettingsWindowState{x:i32::from(console.frame.x),y:i32::from(console.frame.y),
        width:i32::from(console.frame.width),height:i32::from(console.frame.height),maximized:false,
        expanded_row:state.expanded,scroll_offset:state.scroll,control_focus:state.control,row_count})
}

// ------------------------=
// FUNC: draft_geometry
// DESC: Exports production-computed Settings layer bounds so Studio selections cannot drift from remapped native controls.
// ------------------=
#[cfg(studio_native_preview)]
pub(crate) fn draft_geometry(bytes:&[u8],kind:u8,screen:u8,width:usize,height:usize,state:DraftServiceState)->Result<String,String> {
    if kind!=2 || !state.simulated || !(crate::settings_view::supports_basic_section(usize::from(screen-1)) || matches!(screen,7|8)) {
        return Ok("[]".into());
    }
    let template=InstallerTemplate::parse_settings(bytes).map_err(|_|"Invalid Settings template")?;
    let window=settings_preview_window(&template,screen,state)?;
    let layout=crate::ui::system_layout::SystemLayout::new(width,height);
    let mut result=String::from("[");
    for index in 0..template.element_count(screen).unwrap_or(0) {
        let element=template.element_at(screen,index).ok_or("Invalid Settings layer")?;
        let Some(r)=layout.settings_element_geometry(window,usize::from(screen-1),element.id) else{continue;};
        if result.len()>1 {result.push(',');}
        let id=element.id.iter().enumerate().map(|(index,byte)|format!("{}{:02x}",
            if [4,6,8,10].contains(&index){"-"}else{""},byte)).collect::<String>();
        result.push_str(&format!("{{\"id\":\"{id}\",\"x\":{},\"y\":{},\"width\":{},\"height\":{}",r.x,r.y,r.width,r.height));
        if crate::ui::installer_template::settings_row_child_role(element.role) {
            if let Some((_,row))=crate::ui::settings_template::element_identity(usize::from(screen-1),element.parent_id) {
                let section=usize::from(screen-1);
                let parent=if element.role==Role::SettingsRowAction as u8 {
                    layout.settings_action_geometry(window,section,row).map(|action|action.parent)
                }else{
                    let role=match element.role{34=>Role::SettingsSummaryDescription,35=>Role::SettingsDetailDescription,
                        36=>Role::SettingsDetailSurface,37=>Role::SettingsSliderTrack,_=>Role::SettingsSliderThumb};
                    layout.settings_row_child_geometry(window,section,row,role,14,119).map(|child|child.parent)
                };
                if let Some(parent)=parent {
                    result.push_str(&format!(",\"parentBounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",parent.x,parent.y,parent.width,parent.height));
                    let geometry=layout.settings_window_geometry_for_section(window,section);
                    let viewport=geometry.viewport;
                    result.push_str(&format!(",\"viewportBounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"revealScroll\":{},\"maximumScroll\":{}",
                        viewport.x,viewport.y,viewport.width,viewport.height,
                        layout.settings_reveal_scroll(window,section),geometry.maximum_scroll));
                }
            }
        }
        result.push('}');
    }
    result.push(']');Ok(result)
}

// ------------------------=
// FUNC: render_draft
// DESC: Produces an unsaved editor frame through the same production template, image, native font and surface painters used at boot.
// ------------------=
#[cfg(studio_native_preview)]
pub(super) fn render_draft(bytes:&[u8],kind:u8,screen:u8,width:usize,height:usize,
    background:&[u8],focus:usize,pressed:bool,phase:Option<u64>,state:DraftServiceState)->Result<Vec<u8>,String> {
    let template=if kind==5 {InstallerTemplate::parse_desktop(bytes)}else if kind==4 {InstallerTemplate::parse_login(bytes)}else if kind==3 {InstallerTemplate::parse_boot(bytes)}else if kind==2 {InstallerTemplate::parse_settings(bytes)}else{InstallerTemplate::parse(bytes)}
        .map_err(|error|format!("Invalid native template: {error:?}"))?;
    if template.element_count(screen).is_none() {return Err("Unknown template screen".into());}
    let motion_key=(u32::from(screen)<<8)|focus as u32;
    access_style::animate(motion_key,0,pressed,phase.is_none());
    access_style::animate(motion_key,phase.unwrap_or(1000),pressed,phase.is_none());
    let mut pixels=vec![0u32;width*height];
    let mut target=display(&mut pixels,width,height);
    target.paint_bitmap_cover_rect(background,0,0,width,height);
    let pointer=template.element(screen,Role::PrimaryButton)
        .map(|element|(i32::from(element.frame.x)+1,i32::from(element.frame.y)+1,pressed));
    let navigation=Some((focus,true,if pressed {pointer}else{None}));
    let live_details=template.element(screen,Role::LiveDetails).map(|element|element.text);
    let offsets=[0,-480,-420,-360,-300,-240,0,60,330,480,540,600];
    let date_time=if kind==0 && screen==5 && state.simulated {
        Some((crate::storage::DateTimeConfiguration{year:2026,month:10,day:21,hour:14,minute:30,second:0,
            time_zone_id:u16::from(state.zone),utc_offset_minutes:offsets[usize::from(state.zone)]},usize::from(state.date_part)))
    }else{None};
    let navigation=if date_time.is_some() {Some((usize::from(state.date_focus),true,None))}else{navigation};
    let mut settings_rendered=false;
    if kind==2 && state.simulated && crate::settings_view::supports_basic_section(usize::from(screen-1)) {
        let window=settings_preview_window(&template,screen,state)?;
        let layout=crate::ui::system_layout::SystemLayout::new(width,height);
        let interaction=window.expanded_row.and_then(|row|layout.settings_action_geometry(window,usize::from(screen-1),row))
            .filter(|action|action.visible&&action.clip.intersects(action.bounds)&&focus==1)
            .map(|action|crate::ui::app_launcher::window_controls::Highlight{rect:action.bounds,pressed});
        crate::ui::app_launcher::window_controls::publish(interaction);
        let preview_value:&[u8]=if screen==5{b"15 minutes"}else{b"Preview machine"};
        settings_rendered=target.render_basic_settings_view(usize::from(screen-1),preview_value,window,target.ui_scale(),
            crate::settings_view::BasicSettingsSnapshot{local_only:true,output_voice:b"Heart (female)",face_login_enabled:true,chat_enabled:true,chat_model:b"Hermes 3 Llama 3.2 3B",
                voice_enabled:true,wake_word:b"Infinity",speech_enabled:true,
                preferences:crate::ui::input_preferences::Preferences::defaults(),timeout_minutes:15,timeout_maximum:120});
        crate::ui::app_launcher::window_controls::publish(None);
        let geometry=crate::ui::system_layout::SystemLayout::new(width,height)
            .settings_window_geometry_for_section(window,usize::from(screen-1));
        target.precision_window_frame(geometry.window,true,target.ui_scale());
    }
    if kind==2 && state.simulated {
        if let Some(snapshot)=dashboard_fixture(usize::from(screen-1)) {
            let window=settings_preview_window(&template,screen,state)?;
            settings_rendered=target.render_dashboard_settings_view(usize::from(screen-1),b"Preview",window,target.ui_scale(),snapshot);
            let geometry=crate::ui::system_layout::SystemLayout::new(width,height)
                .settings_window_geometry_for_section(window,usize::from(screen-1));
            target.precision_window_frame(geometry.window,true,target.ui_scale());
        }
    }
    if kind==5 {
        if !target.desktop_template_scene(bytes,screen,&[],None,true,if screen==3 {&[65,66,67,68,69,70,71]}else{&[]}) {
            return Err("Native desktop scene rejected the template".into());
        }
        if screen==3 {
            let geometry=crate::ui::system_layout::SystemLayout::new(width,height).ai_chat_geometry(false);
            let scale=target.ui_scale();
            target.ai_widget_header(geometry,b"Say Infinity",false,false,scale,(134,199,255));
            target.ai_widget_model(geometry.model,b"Hermes 3 Llama 3.2 3B",scale);
            target.desktop_bound_surface(67,geometry.timeline);
            target.ai_widget_empty_timeline(geometry.timeline,scale);
            target.ai_widget_composer(geometry.composer,b"",0,None,scale,(134,199,255));
            target.ai_widget_send(geometry.send,false,scale);
        }
    } else if kind==4 {
        if !target.login_template_scene_motion(bytes, b"Select an account", b"", focus,
            Some(crate::ui::authentication_motion::presentation_at(phase.unwrap_or(0).min(2400) as u16))) {
            return Err("Native login scene rejected the template".into());
        }
    } else if kind==3 {
        if !target.boot_template_scene(bytes,screen,if state.simulated {usize::from(state.percent)}else{100},b"",phase.unwrap_or(2400) as usize) {
            return Err("Native boot scene rejected the template".into());
        }
    } else if kind==1 {
        let network=if screen==7 {Some(onboarding_network_view::NetworkViewState{selected:0,connected:true,
            wired_detail:b"Connected link detected",wireless_detail:b"No wireless adapter detected"})}else{None};
        if !target.configuration_template_scene_data(bytes,usize::from(screen-1),focus,
            template_compositor::ConfigurationViewState{input:b"",masked:false,validation_error:false,network}) {
            return Err("Native configuration composition rejected the scene".into());
        }
    } else if !settings_rendered && !target.template_scene_data_with_date_time(bytes,screen,navigation,true,false,live_details,
        if state.simulated {Some((usize::from(state.percent),b"Simulated installation progress"))}else{None},kind==1,kind==2,date_time) {
        return Err("Native template composition rejected the scene".into());
    }
    if kind==0 && screen==7 && state.simulated && state.confirmation {
        target.installer_confirmation_popup(focus,if pressed {700}else{-1},if pressed {568}else{-1},pressed);
    }
    if kind==1 {
        if let Some(card)=template.element(screen,Role::Console).filter(|element|!element.hidden && element.opacity>0) {
            target.access_template_panel_glow(card);
            let frame=scale_template_rect(card.frame,width,height);
            target.access_panel_entrance((frame.left,frame.top,frame.width,frame.height),true);
        }
    }
    let mut rgba=Vec::with_capacity(width*height*4);
    for pixel in pixels {rgba.extend_from_slice(&[(pixel&255)as u8,((pixel>>8)&255)as u8,((pixel>>16)&255)as u8,255]);}
    Ok(rgba)
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
// FUNC: authored_progress_pixels
// DESC: Proves saved font, fill, rim, opacity and bounds affect real installation progress pixels rather than an unrelated preview control.
// ------------------=
fn authored_progress_pixels() {
    let template=InstallerTemplate::parse(crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES).unwrap();
    let mut element=template.element(8,Role::ProgressBar).unwrap();
    element.frame=crate::ui::installer_template::InstallerTemplateRect{x:100,y:100,width:800,height:300};
    element.font_size=18;element.fill=[62,111,255,255];element.border=[134,199,255,255];
    element.opacity=100;element.corner_style=1;element.surface_material=1;element.text=b"Preparing installation";
    let (width,height)=(640,500);
    let rect=scale_template_rect(element.frame,width,height);
    let render=|element| {
        let mut pixels=vec![0x251407;width*height];
        display(&mut pixels,width,height).template_element_in_rect(element,None,None,rect);
        pixels
    };
    let original=render(element);
    let mut large=element;large.font_size=48;
    let larger=render(large);
    let white_pixels=|pixels:&[u32]|pixels.iter().filter(|pixel|
        (**pixel&255)>220 && ((**pixel>>8)&255)>220 && ((**pixel>>16)&255)>220).count();
    assert!(white_pixels(&larger)>white_pixels(&original)+20);
    let mut colored=element;colored.fill=[229,48,65,255];
    let recolored=render(colored);
    let lower_changes=(rect.top+rect.height/2..rect.bottom()).flat_map(|y|
        (rect.left..rect.right()).map(move|x|y*width+x))
        .filter(|at|original[*at]!=recolored[*at]).count();
    assert!(lower_changes>20);
    let mut rim=element;rim.border=[238,180,37,255];
    let reoutlined=render(rim);
    let rim_changes=(rect.top..rect.bottom()).flat_map(|y|
        (rect.left..rect.right()).filter(move|x|*x<rect.left+8||*x>=rect.right()-8||y<rect.top+8||y>=rect.bottom()-8)
            .map(move|x|y*width+x)).filter(|at|original[*at]!=reoutlined[*at]).count();
    assert!(rim_changes>20);
    let mut transparent=element;transparent.opacity=0;
    assert!(render(transparent).iter().all(|pixel|*pixel==0x251407));
    for (at,pixel) in recolored.iter().enumerate() {
        let (x,y)=(at%width,at/width);
        if x<rect.left||x>=rect.right()||y<rect.top||y>=rect.bottom() {assert_eq!(*pixel,0x251407);}
    }
    write_ppm("builds/validation/eclipse-precision/wizard/progress-authored-style.ppm",&larger,width,height);
}

// ------------------------=
// FUNC: authored_progress_marker_pixels
// DESC: Verifies marker style edits reach shared native pixels, preserve authored circles and honor rectangular override bounds without exterior damage.
// ------------------=
fn authored_progress_marker_pixels() {
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let mut element=template.element(5,Role::ProgressSegment).unwrap();
    element.frame=crate::ui::installer_template::InstallerTemplateRect{x:100,y:100,width:500,height:200};
    element.fill=[62,111,255,219];element.border=[134,199,255,231];element.opacity=87;
    let (w,h)=(400,300);let rect=scale_template_rect(element.frame,w,h);
    for current in [false,true] {
        let mut renders=Vec::new();
        for (material,corners,radius) in [(0,0,0),(2,0,0),(1,1,0),(1,2,4),(2,2,28)] {
            element.surface_material=material;element.corner_style=corners;element.corner_radius=radius;
            let kind=if current{Surface::Current}else{Surface::Inactive};
            let mut actual=vec![0x251407;w*h];let mut expected=actual.clone();
            display(&mut actual,w,h).access_template_control(element,None,if current{(0,0)}else{(1,0)});
            let size=rect.width.min(rect.height);
            let frame=if corners==0 {Rect{x:(rect.left+(rect.width-size)/2)as i32,y:rect.top as i32,width:size as u32,height:size as u32}}
                else{Rect{x:rect.left as i32,y:rect.top as i32,width:rect.width as u32,height:rect.height as u32}};
            display(&mut expected,w,h).precision_surface_styled(frame,kind,Material{fill:(62,111,255),edge:(134,199,255),
                opacity:(219u16*87/100)as u8,edge_opacity:(231u16*87/100)as u8,focus:0,pressed:false,scale:1},
                crate::ui::panel_recipe::template_style(kind,material,corners,radius));
            assert_eq!(actual,expected);
            for (at,pixel) in actual.iter().enumerate() {
                let (x,y)=(at%w,at/w);
                if x<rect.left||x>=rect.right()||y<rect.top||y>=rect.bottom(){assert_eq!(*pixel,0x251407);}
            }
            renders.push(actual);
        }
        for pair in renders.windows(2) {assert_ne!(pair[0],pair[1]);}
        assert_eq!(renders[0][(rect.top+rect.height/2)*w+rect.left+8],0x251407);
        assert_ne!(renders[2][(rect.top+rect.height/2)*w+rect.left+8],0x251407);
    }
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
        assert!(target.configuration_template_scene_data(CONFIGURATION_TEMPLATE_BYTES,step,1,
            template_compositor::ConfigurationViewState{input:b"Updated account",masked:step==4,validation_error:false,network:None}));
        let mut actual=vec![0;w*h];
        let mut target=display(&mut actual,w,h);
        assert!(target.configuration_template_scene_data(CONFIGURATION_TEMPLATE_BYTES,step,1,
            template_compositor::ConfigurationViewState{input:b"Old account",masked:step==4,validation_error:false,network:None}));
        let card=scale_template_rect(template.element(step as u8+1,Role::Console).unwrap().frame,w,h);
        actual[(card.top+8)*w+card.left+8]=0x00ff21;
        let before=actual.clone();
        let mut target=display(&mut actual,w,h);target.render_clip=Some(clip);
        assert!(target.configuration_template_scene_data(CONFIGURATION_TEMPLATE_BYTES,step,1,
            template_compositor::ConfigurationViewState{input:b"Updated account",masked:step==4,validation_error:false,network:None}));
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
// FUNC: ordered_live_layer_pixels
// DESC: Proves authored layers above live input and navigation stay above them, including damage-clipped input updates.
// ------------------=
fn ordered_live_layer_pixels() {
    let (w,h)=(1280,800);
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let cover=template.element(2,Role::Content).unwrap();
    for role in [Role::Input,Role::PrimaryButton] {
        let field=template.element(2,role).unwrap();
        let mut bytes=CONFIGURATION_TEMPLATE_BYTES.to_vec();
        let start=bytes.windows(16).position(|id|id==cover.id).unwrap();
        let kind=start+17+usize::from(bytes[start+16]);
        bytes[kind+3]=(bytes[kind+3]&1)|(2<<2)|(1<<4);
        bytes[kind+4..kind+6].copy_from_slice(&32767i16.to_le_bytes());
        for (index,value) in [field.frame.x,field.frame.y,field.frame.width,field.frame.height].iter().enumerate() {
            bytes[kind+6+index*2..kind+8+index*2].copy_from_slice(&value.to_le_bytes());
        }
        bytes[kind+14..kind+18].copy_from_slice(&[230,17,85,255]);
        bytes[kind+18..kind+22].fill(0);bytes[kind+22]=100;bytes[kind+23]=0;
        InstallerTemplate::parse(&bytes).unwrap();
        let r=scale_template_rect(field.frame,w,h);
        let clip=Region{left:r.left,top:r.top,right:r.left+r.width,bottom:r.top+r.height};
        let mut first=vec![0;w*h];let mut second=first.clone();
        for (pixels,input,focus) in [(&mut first,b"First".as_slice(),0),(&mut second,b"Updated name".as_slice(),1)] {
            assert!(display(pixels,w,h).configuration_template_scene_data(&bytes,1,focus,
                template_compositor::ConfigurationViewState{input,masked:false,validation_error:false,network:None}));
        }
        for y in r.top+4..r.bottom()-4 {for x in r.left+4..r.right()-4 {
            assert_eq!(first[y*w+x],second[y*w+x],"Later authored layer must occlude live control");
        }}
        let before=first.clone();
        let mut target=display(&mut first,w,h);target.render_clip=Some(clip);
        assert!(target.configuration_template_scene_data(&bytes,1,1,
            template_compositor::ConfigurationViewState{input:b"Updated name",masked:false,validation_error:false,network:None}));
        assert_partial_pixels(&first,&second,&before,w,clip);
        bytes[kind+3]|=2;
        let mut exposed=vec![0;w*h];
        assert!(display(&mut exposed,w,h).configuration_template_scene_data(&bytes,1,1,
            template_compositor::ConfigurationViewState{input:b"Updated name",masked:false,validation_error:false,network:None}));
        assert!((r.top+4..r.bottom()-4).any(|y|(r.left+4..r.right()-4).any(|x|exposed[y*w+x]!=second[y*w+x])));
    }
}

// ------------------------=
// FUNC: input_opacity_pixels
// DESC: Verifies native input text and caret respect authored opacity and hidden state independently of transparent field surfaces.
// ------------------=
fn input_opacity_pixels() {
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let mut element=template.element(2,Role::Input).unwrap();
    element.fill[3]=0;element.border[3]=0;
    let (w,h)=(640,480);let r=scale_template_rect(element.frame,w,h);
    crate::ui::text_input::set_presentation(true,true,2,1,false);
    let mut samples=Vec::new();
    for opacity in [100,50,0] {
        element.opacity=opacity;let mut pixels=vec![0;w*h];
        display(&mut pixels,w,h).onboarding_input_field_with_element(Some(element),r.left,r.top,r.width,r.height,
            b"Alpha",false,true,b"");
        samples.push(pixels);
    }
    assert!(samples[0].iter().any(|pixel|*pixel!=0));
    assert_ne!(samples[0],samples[1]);
    assert!(samples[2].iter().all(|pixel|*pixel==0));
    for (full,half) in samples[0].iter().zip(&samples[1]) {
        for shift in [0,8,16] {assert!(((half>>shift)&255)<=((full>>shift)&255));}
    }
    element.opacity=100;element.hidden=true;
    let mut hidden=vec![0;w*h];
    display(&mut hidden,w,h).onboarding_input_field_with_element(Some(element),r.left,r.top,r.width,r.height,
        b"Alpha",false,true,b"");
    assert!(hidden.iter().all(|pixel|*pixel==0));
    crate::ui::text_input::set_presentation(false,false,0,1,false);
}

// ------------------------=
// FUNC: network_state_pixels
// DESC: Verifies real network selection state changes pixels only inside shared authored choice bounds.
// ------------------=
fn network_state_pixels() {
    let (w,h)=(1280,800);
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let frames=core::array::from_fn(|index|crate::ui::installer_layout::configuration_network_row_rect_in(&template,index,w,h));
    let mut before=vec![0;w*h];let mut after=before.clone();
    for (pixels,selected) in [(&mut before,0),(&mut after,2)] {
        display(pixels,w,h).onboarding_network_rows_from_state((0,0,0),frames,0,false,
            onboarding_network_view::NetworkViewState{selected,connected:true,
                wired_detail:b"Connected link detected",wireless_detail:b"No wireless adapter detected"});
    }
    assert_ne!(before,after);
    for (at,(first,second)) in before.iter().zip(&after).enumerate() {
        let (x,y)=(at%w,at/w);
        if !frames.iter().flatten().any(|r|x>=r.left&&x<r.right()&&y>=r.top&&y<r.bottom()) {
            assert_eq!(first,second);
            assert_eq!(*first,0);
        }
    }
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
                let scale=display(&mut pixels,w,h).ui_scale();
                let r=scale_template_rect(element.frame,w,h);
                display(&mut pixels,w,h).template_element_in_rect(element,None,
                    Some((focus,true,Some((element.frame.x as i32+1,element.frame.y as i32+1,pressed)))),r);
                let p=pixels[(r.top+r.height/2)*w+r.left+r.width/2];
                // Saved scenes and UIKit recipes may intentionally use any
                // palette. Verify their actual contract instead of requiring
                // every user-authored button to remain factory blue.
                use crate::ui::panel_recipe::{self,Surface,Material};
                let kind=if role==Role::PrimaryButton {Surface::Primary}else{Surface::Secondary};
                let (red,green,blue,alpha)=panel_recipe::pixel_styled(kind,r.width,r.height,r.width/2,r.height/2,
                    Material{fill:(element.fill[0],element.fill[1],element.fill[2]),
                        edge:(element.border[0],element.border[1],element.border[2]),
                        opacity:(u16::from(element.fill[3])*u16::from(element.opacity)/100)as u8,
                        edge_opacity:(u16::from(element.border[3])*u16::from(element.opacity)/100)as u8,
                        focus:255,pressed,scale},
                    panel_recipe::template_style(kind,element.surface_material,element.corner_style,element.corner_radius));
                let channel=|c:u8|u32::from(c)*u32::from(alpha)/255;
                assert_eq!(p,channel(red)|(channel(green)<<8)|(channel(blue)<<16));
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
                assert!(target.template_scene_data_with_date_time(
                    crate::ui::installer_layout::INSTALLER_TEMPLATE_BYTES,5,Some((2,true,None)),true,false,None,None,false,false,Some((
                    crate::storage::DateTimeConfiguration{year:2026,month:10,day:21,hour:11,minute:25,second:0,
                        time_zone_id:3,utc_offset_minutes:-360},1))));
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
// FUNC: confirmation_text_pixels
// DESC: Checks scaled title/subtitle and footer glyph damage remains inside native control bounds at every Studio viewport.
// ------------------=
fn confirmation_text_pixels() {
    for (w,h) in [(1280,800),(1600,1000),(1920,1080),(2560,1440)] {
        for target in [crate::ui::installer_layout::InstallerConfirmationTarget::Cancel,
            crate::ui::installer_layout::InstallerConfirmationTarget::Install] {
            let frame=crate::ui::installer_layout::installer_confirmation_button_frame(target);
            let rect=crate::ui::installer_layout::InstallerRect{left:w*frame.left/1000,top:h*frame.top/1000,
                width:w*frame.width/1000,height:h*frame.height/1000};
            let mut empty=vec![0x00182739;w*h];let mut text=empty.clone();
            display(&mut empty,w,h).installer_button_rect(rect,b"",b"",false,false,-1,-1,false);
            display(&mut text,w,h).installer_button_rect(rect,b"ERASE & INSTALL",b"Keep the disk unchanged",false,false,-1,-1,false);
            let inset=(8*h/1000).max(2);let mut count=0;
            for y in 0..h {for x in 0..w {if text[y*w+x]!=empty[y*w+x] {
                assert!(x>=rect.left+inset && x<rect.right()-inset && y>=rect.top+inset && y<rect.bottom()-inset);count+=1;
            }}}
            assert!(count>100);
        }
        let mut footer=vec![0x00182739;w*h];
        display(&mut footer,w,h).installer_confirmation_footer(b"Tab / arrows: choose     Enter: confirm     Esc: cancel");
        let margin=(12*h/1000).max(4);let bottom=h*34/100+h*32/100;
        let mut count=0;
        for y in 0..h {for x in 0..w {if footer[y*w+x]!=0x00182739 {
            assert!(x>=w/4+margin && x<w*3/4-margin && y<bottom-margin && y>bottom-h/20);count+=1;
        }}}
        assert!(count>100);
    }
}

// ------------------------=
// FUNC: boot_scene_clip_pixels
// DESC: Proves clipped reveal frames match full production frames without painting outside the requested damage region.
// ------------------=
fn boot_scene_clip_pixels() {
    let bytes=include_bytes!("../assets/boot/boot-screens.iuit");
    for (w,h) in [(640,480),(1024,768)] {
        for screen in 1..=2 {
            let mut full=vec![0;w*h];
            assert!(display(&mut full,w,h).boot_template_scene(bytes,screen,50,b"",1200));
            let mut clipped=vec![0x123456;w*h];
            let (left,top,right,bottom)=(w/3,h/5,w*2/3,h*4/5);
            let mut target=display(&mut clipped,w,h);
            target.set_render_clip(left,top,right-left,bottom-top);
            assert!(target.boot_template_scene(bytes,screen,50,b"",1200));
            for y in 0..h { for x in 0..w {
                assert_eq!(clipped[y*w+x],if x>=left&&x<right&&y>=top&&y<bottom {full[y*w+x]}else{0x123456});
            }}
        }
    }
}

// ------------------------=
// FUNC: verify
// DESC: Renders authored first-boot scenes with actual compositor, bitmap sampling, Inter, image-backed controls and semantic state assertions.
// ------------------=
pub(super) fn verify() {
    boot_scene_clip_pixels();
    let output="builds/validation/eclipse-precision/wizard";
    std::fs::create_dir_all(output).unwrap();
    let comparisons=role_pixels();
    authored_progress_pixels();
    authored_progress_marker_pixels();
    frost_pixels();
    retained_frost_pixels();
    partial_scene_pixels();
    ordered_live_layer_pixels();
    input_opacity_pixels();
    confirmation_text_pixels();
    network_state_pixels();
    authored_button_pixels(CONFIGURATION_TEMPLATE_BYTES,8);
    installer_scenes(output);
    let template=InstallerTemplate::parse(CONFIGURATION_TEMPLATE_BYTES).unwrap();
    let mut screen_pixels=0;
    for (width,height) in [(1280,800),(1920,1080)] {
        for step in 0usize..8 {
            access_style::animate(((step as u32+1)<<8)|1,1000,false,true);
            let mut pixels=vec![0; width*height];
            let mut target=display(&mut pixels,width,height);
            let network=if step==6 {Some(onboarding_network_view::NetworkViewState{selected:0,connected:true,
                wired_detail:b"Connected link detected",wireless_detail:b"No wireless adapter detected"})}else{None};
            assert!(target.configuration_template_scene_data(CONFIGURATION_TEMPLATE_BYTES,step,1,
                template_compositor::ConfigurationViewState{input:b"",masked:step==4,validation_error:false,network}));
            if let Some(card)=template.element(step as u8+1,Role::Console) {
                target.access_template_panel_glow(card);
            }
            assert!(pixels.iter().filter(|p|**p!=0).count()>width*height*9/10);
            write_ppm(&format!("{output}/step-{step}-{width}x{height}.ppm"),&pixels,width,height);
            screen_pixels+=pixels.len();
        }
    }
    std::fs::write(format!("{output}/evidence.json"),format!(
        "{{\"semantic_control_pixel_comparisons\":{comparisons},\"configuration_scene_pixels\":{screen_pixels},\"configuration_scenes\":16,\"installer_template_scenes\":26,\"native_compositor\":true,\"native_fonts\":true,\"installed_verified\":false,\"dynamic_network_scene_verified\":true}}\n")).unwrap();
}
