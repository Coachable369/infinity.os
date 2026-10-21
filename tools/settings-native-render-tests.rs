//! Behavioral frame/geometry proof of the production Settings snapshot renderer.
use super::{DisplayDevice,Region};
use crate::settings_view::{BasicSettingsSnapshot,supports_basic_section};
use crate::ui::{geometry::{Rect,Point},system_layout::{SettingsWindowState,SystemLayout},installer_template::InstallerTemplateRole as Role};

// ------------------------=
// FUNC: snapshot
// DESC: Defines explicit preview values without emulating any runtime service.
// ------------------=
fn snapshot()->BasicSettingsSnapshot<'static> {BasicSettingsSnapshot{chat_enabled:true,chat_model:b"Hermes 3 Llama 3.2 3B",voice_enabled:true,wake_word:b"Infinity",speech_enabled:true,preferences:crate::ui::input_preferences::Preferences::defaults(),timeout_minutes:5,timeout_maximum:120}}
// ------------------------=
// FUNC: state
// DESC: Builds bounded native Settings window geometry for a selected section and viewport.
// ------------------=
fn state(section:usize)->SettingsWindowState {SettingsWindowState{x:80,y:100,width:840,height:800,maximized:false,expanded_row:None,scroll_offset:0,control_focus:0,row_count:if section==10{8}else if section==3{7}else{5}}}
// ------------------------=
// FUNC: frame
// DESC: Renders production Settings into a memory framebuffer with optional damage clipping.
// ------------------=
fn frame(width:usize,height:usize,section:usize,state:SettingsWindowState,values:BasicSettingsSnapshot<'_>,clip:Option<Region>)->(Vec<u32>,bool) {
    let mut pixels=vec![0x00182739;width*height];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:clip,fast_motion_frame:false,submissions:0,recording_surface:false};
    let success=display.render_basic_settings_view(section,if section==4{b"5 minutes"}else{b"Studio preview"},state,display.ui_scale(),values);
    (pixels,success)
}
// ------------------------=
// FUNC: damage
// DESC: Computes actual changed-pixel bounds and count without relying on human-readable output.
// ------------------=
fn damage(a:&[u32],b:&[u32],width:usize)->(Rect,usize) {
    let(mut l,mut t,mut r,mut bottom,mut count)=(width,a.len()/width,0,0,0);
    for(i,(x,y))in a.iter().zip(b).enumerate(){if x!=y{let(px,py)=(i%width,i/width);l=l.min(px);t=t.min(py);r=r.max(px+1);bottom=bottom.max(py+1);count+=1;}}
    (Rect{x:l as i32,y:t as i32,width:r.saturating_sub(l)as u32,height:bottom.saturating_sub(t)as u32},count)
}
// ------------------------=
// FUNC: verify
// DESC: Verifies shared native Settings state changes, clipping, scrolling and exact exported semantic geometry across seven sections.
// ------------------=
pub(super) fn verify() {
    crate::ui::text_input::set_presentation(false,false,0,1,false);
    let mut frames=0;
    for(width,height)in[(1280,800),(1600,1000)]{
        let layout=SystemLayout::new(width,height);
        for section in [0,2,3,4,5,9,10]{
            assert!(supports_basic_section(section));let state=state(section);let geometry=layout.settings_window_geometry_for_section(state,section);
            let (whole,ok)=frame(width,height,section,state,snapshot(),None);assert!(ok);assert!(whole.iter().any(|p|*p!=0x00182739));
            let clip=Region{left:geometry.window.x.max(0)as usize+40,top:geometry.window.y.max(0)as usize+20,right:geometry.window.right().max(0)as usize-20,bottom:geometry.window.bottom().max(0)as usize-20};
            let (partial,ok)=frame(width,height,section,state,snapshot(),Some(clip));assert!(ok);
            for y in 0..height{for x in 0..width{assert_eq!(partial[y*width+x],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom{whole[y*width+x]}else{0x00182739});}}
            let element=crate::ui::settings_template::role_at(section,Role::Metadata,0).unwrap();
            assert_eq!(layout.settings_element_geometry(state,section,element.id),Some(layout.settings_row_geometry_for_section(state,0,section).summary));
            for role in [Role::SettingsRowLabel,Role::SettingsRowValue,Role::SettingsDisclosure]{let child=crate::ui::settings_template::role_at(section,role,0).unwrap();assert_eq!(layout.settings_element_geometry(state,section,child.id),layout.settings_row_element_geometry(state,section,0,role));}
            let mut expanded=state;expanded.expanded_row=Some(if section==5{2}else if section==4{3}else{0});
            let(expanded_frame,ok)=frame(width,height,section,expanded,snapshot(),None);assert!(ok);assert!(damage(&whole,&expanded_frame,width).1>10);
            let max=layout.settings_window_geometry_for_section(expanded,section).maximum_scroll;
            if max>0{expanded.scroll_offset=max;let(scrolled,ok)=frame(width,height,section,expanded,snapshot(),None);assert!(ok);assert!(damage(&expanded_frame,&scrolled,width).1>10);}
            frames+=3;
        }
        let section=3;let state=state(section);let(a,_)=frame(width,height,section,state,snapshot(),None);let mut changed=snapshot();changed.chat_enabled=false;let(b,_)=frame(width,height,section,state,changed,None);let(bounds,count)=damage(&a,&b,width);assert!(count>10);
        let row=layout.settings_row_geometry_for_section(state,1,section).summary;assert!(row.contains(Point{x:bounds.x,y:bounds.y}));assert!(row.contains(Point{x:bounds.right()-1,y:bounds.bottom()-1}));
        for unsupported in [1,6,7,8]{let(pixels,ok)=frame(width,height,unsupported,state,snapshot(),None);assert!(!ok);assert!(pixels.iter().all(|p|*p==0x00182739));}
    }
    authored_style_pixels();
    dashboard_frames();
    println!("Native Settings production painter proof: {frames} frames, state damage, clip parity, layer geometry and authored typography/material PASS");
}

// ------------------------=
// FUNC: dashboard_frames
// DESC: Exercises every real Network and Nodes dashboard page with typed fixtures, native damage clipping and exact semantic selection bounds.
// ------------------=
fn dashboard_frames() {
    for (w,h) in [(1280,800),(1600,1000)] {
        for section in [6usize,7] {
            let mut state=state(section);state.row_count=6;
            let snapshot=crate::configuration_wizard_render::dashboard_fixture(section).unwrap();
            let mut previous=None;
            for page in 0..if section==6{7}else{5} {
                state.expanded_row=Some(page);
                let mut pixels=vec![0x00182739;w*h];
                let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width:w,height:h,stride:w,format:0,
                    render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
                assert!(display.render_dashboard_settings_view(section,b"Preview",state,display.ui_scale(),snapshot));
                assert!(pixels.iter().any(|pixel|*pixel!=0x00182739));
                if let Some(before)=previous {assert_ne!(pixels,before);}
                let layout=SystemLayout::new(w,h);
                let geometry=layout.settings_window_geometry_for_section(state,section);
                let bounds=if section==6{layout.network_settings_geometry(state)}else{layout.node_settings_geometry(state)};
                let control=crate::ui::settings_template::role_at(section,Role::Metadata,0).unwrap();
                assert_eq!(layout.settings_element_geometry(state,section,control.id),Some(bounds.controls[0]));
                let tab=crate::ui::settings_template::role_at(section,Role::SettingsTab,page).unwrap();
                assert_eq!(layout.settings_element_geometry(state,section,tab.id),Some(bounds.tabs[page]));
                let clip=Region{left:geometry.window.x.max(0)as usize+10,top:geometry.window.y.max(0)as usize+10,
                    right:geometry.window.right().max(0)as usize-10,bottom:geometry.window.bottom().max(0)as usize-10};
                let mut partial=vec![0x00182739;w*h];
                let mut display=DisplayDevice{buffer:partial.as_mut_ptr(),width:w,height:h,stride:w,format:0,
                    render_clip:Some(clip),fast_motion_frame:false,submissions:0,recording_surface:false};
                assert!(display.render_dashboard_settings_view(section,b"Preview",state,display.ui_scale(),snapshot));
                for y in 0..h {for x in 0..w {assert_eq!(partial[y*w+x],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom{pixels[y*w+x]}else{0x00182739});}}
                previous=Some(pixels);
            }
        }
    }
}

// ------------------------=
// FUNC: authored_style_pixels
// DESC: Verifies that authored font size, placement, transparency, hidden state and material produce bounded native pixel changes.
// ------------------=
fn authored_style_pixels() {
    let bounds=Rect{x:48,y:42,width:240,height:70};
    let mut element=crate::ui::settings_template::role_at(0,Role::SettingsRowLabel,0).unwrap();
    let render=|element:crate::ui::installer_template::InstallerTemplateElement<'_>,bounds:Rect| {
        let mut pixels=vec![0x00182739;320*1000];
        let mut target=DisplayDevice{buffer:pixels.as_mut_ptr(),width:320,height:1000,stride:320,format:0,render_clip:Some(Region{left:0,top:0,right:320,bottom:160}),fast_motion_frame:false,submissions:0,recording_surface:false};
        target.paint_settings_bound_text(Some(element),bounds,b"Preview typography",(0,0,0),true,false);pixels
    };
    element.font_size=12;let small=render(element,bounds);element.font_size=24;let large=render(element,bounds);
    let count=|pixels:&[u32]|pixels.iter().filter(|p|**p!=0x00182739).count();assert!(count(&large)>count(&small));
    let empty=vec![0x00182739;320*1000];let changed=damage(&empty,&large,320).0;assert!(bounds.contains(Point{x:changed.x,y:changed.y}));assert!(bounds.contains(Point{x:changed.right()-1,y:changed.bottom()-1}));
    element.opacity=35;let transparent=render(element,bounds);assert!(damage(&large,&transparent,320).1>10);
    element.hidden=true;assert_eq!(render(element,bounds),empty);element.hidden=false;element.opacity=100;
    let shifted=Rect{x:bounds.x+10,y:bounds.y+7,..bounds};let moved=render(element,shifted);let moved_bounds=damage(&empty,&moved,320).0;assert_eq!((moved_bounds.x,moved_bounds.y),(changed.x+10,changed.y+7));
    let render_surface=|style:u8,corners:u8|{
        let mut pixels=vec![0x00182739;320*160];let mut target=DisplayDevice{buffer:pixels.as_mut_ptr(),width:320,height:160,stride:320,format:0,render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
        let mut element=crate::ui::settings_template::role_at(0,Role::Metadata,0).unwrap();element.surface_material=style;element.corner_style=corners;element.corner_radius=30;
        target.settings_template_surface(bounds,Some(element),crate::ui::panel_recipe::Material{fill:(10,20,30),edge:(40,60,80),opacity:255,edge_opacity:255,focus:0,pressed:false,scale:1});pixels
    };
    let glass=render_surface(1,1);let matte=render_surface(2,1);let rounded=render_surface(1,2);
    assert!(damage(&glass,&matte,320).1>100);assert!(damage(&glass,&rounded,320).1>100);
}
