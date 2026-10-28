//! Behavioral frame/geometry proof of the production Settings snapshot renderer.
use super::{DisplayDevice,Region};
use crate::settings_view::{BasicSettingsSnapshot,supports_basic_section};
use crate::ui::{geometry::{Rect,Point},system_layout::{SettingsWindowState,SystemLayout},installer_template::InstallerTemplateRole as Role};

// ------------------------=
// FUNC: snapshot
// DESC: Defines explicit preview values without emulating any runtime service.
// ------------------=
fn snapshot()->BasicSettingsSnapshot<'static> {BasicSettingsSnapshot{local_only:true,output_voice:b"Heart (female)",face_login_enabled:true,chat_enabled:true,chat_model:b"Hermes 3 Llama 3.2 3B",voice_enabled:true,wake_word:b"Infinity",speech_enabled:true,preferences:crate::ui::input_preferences::Preferences::defaults(),timeout_minutes:5,timeout_maximum:120}}
// ------------------------=
// FUNC: state
// DESC: Builds bounded native Settings window geometry for a selected section and viewport.
// ------------------=
fn state(section:usize)->SettingsWindowState {SettingsWindowState{x:80,y:100,width:840,height:800,maximized:false,expanded_row:None,scroll_offset:0,control_focus:0,row_count:if matches!(section,3|10){8}else{5}}}
// ------------------------=
// FUNC: frame
// DESC: Renders production Settings into a memory framebuffer with optional damage clipping.
// ------------------=
fn frame(width:usize,height:usize,section:usize,state:SettingsWindowState,values:BasicSettingsSnapshot<'_>,clip:Option<Region>)->(Vec<u32>,bool) {
    let mut pixels=vec![0x00182739;width*height];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,render_clip:clip,fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
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
        let mut focused=state;focused.control_focus=crate::ui::controls::SETTINGS_ROW_FOCUS+1;
        let(focused_frame,ok)=frame(width,height,section,focused,snapshot(),None);assert!(ok);
        let(bounds,count)=damage(&a,&focused_frame,width);assert!(count>10);
        assert!(row.contains(Point{x:bounds.x,y:bounds.y}));
        assert!(row.contains(Point{x:bounds.right()-1,y:bounds.bottom()-1}));
        let mut policy=snapshot();policy.local_only=false;
        let(policy_frame,ok)=frame(width,height,section,state,policy,None);assert!(ok);
        let(bounds,count)=damage(&a,&policy_frame,width);assert!(count>10);
        let policy_row=layout.settings_row_geometry_for_section(state,0,section).summary;
        assert!(policy_row.contains(Point{x:bounds.x,y:bounds.y}));
        assert!(policy_row.contains(Point{x:bounds.right()-1,y:bounds.bottom()-1}));
        for unsupported in [1,6,7,8]{let(pixels,ok)=frame(width,height,unsupported,state,snapshot(),None);assert!(!ok);assert!(pixels.iter().all(|p|*p==0x00182739));}
    }
    authored_style_pixels();
    let layout = SystemLayout::new(1280, 800);
    let mut voice_state = state(3);
    voice_state.expanded_row = Some(7);
    voice_state.scroll_offset = layout.settings_reveal_scroll(voice_state, 3);
    let (default_voice, ok) = frame(1280, 800, 3, voice_state, snapshot(), None);
    assert!(ok);
    let mut selected = snapshot(); selected.output_voice = b"Michael (male)";
    let (selected_voice, ok) = frame(1280, 800, 3, voice_state, selected, None);
    assert!(ok && damage(&default_voice, &selected_voice, 1280).1 > 10);
    let action = layout.settings_action_geometry(voice_state, 3, 7).unwrap();
    assert!(action.visible && action.clip.intersects(action.bounds));
    let mut ppm = b"P6\n1280 800\n255\n".to_vec();
    for pixel in selected_voice { ppm.extend_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]); }
    std::fs::write("build/voice-settings-proof.ppm", ppm).unwrap();
    authored_action_pixels();
    action_scene_pixels();
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
                    render_clip:None,fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
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
                    render_clip:Some(clip),fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
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
        let mut target=DisplayDevice{buffer:pixels.as_mut_ptr(),width:320,height:1000,stride:320,format:0,render_clip:Some(Region{left:0,top:0,right:320,bottom:160}),fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
        target.paint_settings_bound_text(Some(element),bounds,b"Preview typography",(0,0,0),true,false);pixels
    };
    element.font_size=12;let small=render(element,bounds);element.font_size=24;let large=render(element,bounds);
    let count=|pixels:&[u32]|pixels.iter().filter(|p|**p!=0x00182739).count();assert!(count(&large)>count(&small));
    let empty=vec![0x00182739;320*1000];let changed=damage(&empty,&large,320).0;assert!(bounds.contains(Point{x:changed.x,y:changed.y}));assert!(bounds.contains(Point{x:changed.right()-1,y:changed.bottom()-1}));
    element.opacity=35;let transparent=render(element,bounds);assert!(damage(&large,&transparent,320).1>10);
    element.hidden=true;assert_eq!(render(element,bounds),empty);element.hidden=false;element.opacity=100;
    let shifted=Rect{x:bounds.x+10,y:bounds.y+7,..bounds};let moved=render(element,shifted);let moved_bounds=damage(&empty,&moved,320).0;assert_eq!((moved_bounds.x,moved_bounds.y),(changed.x+10,changed.y+7));
    let render_surface=|style:u8,corners:u8|{
        let mut pixels=vec![0x00182739;320*160];let mut target=DisplayDevice{buffer:pixels.as_mut_ptr(),width:320,height:160,stride:320,format:0,render_clip:None,fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
        let mut element=crate::ui::settings_template::role_at(0,Role::Metadata,0).unwrap();element.surface_material=style;element.corner_style=corners;element.corner_radius=30;
        target.settings_template_surface(bounds,Some(element),crate::ui::panel_recipe::Material{fill:(10,20,30),edge:(40,60,80),opacity:255,edge_opacity:255,focus:0,pressed:false,scale:1});pixels
    };
    let glass=render_surface(1,1);let matte=render_surface(2,1);let rounded=render_surface(1,2);
    assert!(damage(&glass,&matte,320).1>100);assert!(damage(&glass,&rounded,320).1>100);
}

// ------------------------=
// FUNC: action_frame
// DESC: Runs the production authored-button painter over known native pixels with an optional damage clip.
// ------------------=
fn action_frame(element:crate::ui::installer_template::InstallerTemplateElement<'_>,bounds:Rect,clip:Option<Region>)->Vec<u32> {
    let mut pixels=vec![0x00182739;640*1000];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width:640,height:1000,stride:640,format:0,
        render_clip:clip,fast_motion_frame:false,submissions:0,template_typography: [0; 3], recording_surface: false};
    display.paint_template_button_state(element,bounds,true,false,false);
    pixels
}

// ------------------------=
// FUNC: authored_action_pixels
// DESC: Checks actual button pixels for authored movement, resizing, materials, captions, hidden state, transparency and damage clipping without a second drawing implementation.
// ------------------=
fn authored_action_pixels() {
    let mut element=crate::ui::settings_template::row_action(0,0).unwrap();
    element.hidden=false;element.opacity=100;element.font_size=18;element.text=b"ACTION";
    element.caption_color=[255,255,255,255];
    element.fill=[12,31,66,225];element.border=[112,181,246,255];
    element.surface_material=1;element.corner_style=2;element.corner_radius=12;
    let bounds=Rect{x:64,y:64,width:280,height:64};
    let empty=vec![0x00182739;640*1000];
    let normal=action_frame(element,bounds,None);
    let (painted,count)=damage(&empty,&normal,640);
    assert!(count>100);
    assert_eq!(painted.intersection(bounds),painted);

    let moved_bounds=Rect{x:bounds.x+13,y:bounds.y+9,..bounds};
    let moved=action_frame(element,moved_bounds,None);
    let shifted=damage(&empty,&moved,640).0;
    assert_eq!(shifted,Rect{x:painted.x+13,y:painted.y+9,..painted});
    for y in 0..bounds.height as usize {for x in 0..bounds.width as usize {
        assert_eq!(normal[(bounds.y as usize+y)*640+bounds.x as usize+x],
            moved[(moved_bounds.y as usize+y)*640+moved_bounds.x as usize+x]);
    }}
    let resized_bounds=Rect{width:410,height:92,..bounds};
    let resized=action_frame(element,resized_bounds,None);
    let (resized_damage,resized_count)=damage(&empty,&resized,640);
    assert!(resized_count>count);
    assert_eq!(resized_damage.intersection(resized_bounds),resized_damage);

    let mut matte=element;matte.surface_material=2;
    assert!(damage(&normal,&action_frame(matte,bounds,None),640).1>100);
    let mut flat=element;flat.surface_material=3;
    assert!(damage(&normal,&action_frame(flat,bounds,None),640).1>100);
    let mut square=element;square.corner_style=1;
    assert!(damage(&normal,&action_frame(square,bounds,None),640).1>10);
    let mut recolored=element;recolored.fill=[108,27,92,225];recolored.border=[245,176,71,255];
    assert!(damage(&normal,&action_frame(recolored,bounds,None),640).1>100);
    let mut caption=element;caption.text=b"MMMMMMMM";
    let caption_frame=action_frame(caption,bounds,None);
    let (caption_damage,caption_count)=damage(&normal,&caption_frame,640);
    assert!(caption_count>10);
    assert_eq!(caption_damage.intersection(bounds),caption_damage);
    let mut caption_tint=element;caption_tint.caption_color=[255,96,48,255];
    let (tint_damage,tint_count)=damage(&normal,&action_frame(caption_tint,bounds,None),640);
    assert!(tint_count>10);
    assert_eq!(tint_damage.intersection(bounds),tint_damage);
    let mut caption_alpha=element;caption_alpha.caption_color[3]=0;
    let no_caption=action_frame(caption_alpha,bounds,None);
    let mut empty_caption=element;empty_caption.text=b"";
    assert_eq!(no_caption,action_frame(empty_caption,bounds,None));
    let mut larger=element;larger.font_size=26;
    assert!(damage(&normal,&action_frame(larger,bounds,None),640).1>10);
    let mut faded=element;faded.opacity=35;
    assert!(damage(&normal,&action_frame(faded,bounds,None),640).1>100);
    let mut hidden=element;hidden.hidden=true;
    assert_eq!(action_frame(hidden,bounds,None),empty);
    hidden.hidden=false;hidden.opacity=0;
    assert_eq!(action_frame(hidden,bounds,None),empty);

    let clip=Region{left:100,top:84,right:280,bottom:112};
    let clipped=action_frame(element,bounds,Some(clip));
    for y in 0..1000 {for x in 0..640 {
        assert_eq!(clipped[y*640+x],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom {
            normal[y*640+x]
        }else{empty[y*640+x]});
    }}
    let offscreen=Rect{x:-20,y:-16,width:140,height:60};
    let offscreen_frame=action_frame(element,offscreen,None);
    let (visible,visible_count)=damage(&empty,&offscreen_frame,640);
    assert!(visible_count>10);
    assert_eq!(visible.intersection(offscreen.intersection(Rect{x:0,y:0,width:640,height:1000})),visible);
    let reference_bounds=Rect{x:32,y:32,..offscreen};
    let reference=action_frame(element,reference_bounds,None);
    for y in 0..offscreen.bottom()as usize {for x in 0..offscreen.right()as usize {
        let reference_x=(x as i32-offscreen.x+reference_bounds.x)as usize;
        let reference_y=(y as i32-offscreen.y+reference_bounds.y)as usize;
        assert_eq!(offscreen_frame[y*640+x],reference[reference_y*640+reference_x]);
    }}
}

// ------------------------=
// FUNC: action_scene_pixels
// DESC: Verifies live service-state caption changes reach the authored control in complete Settings frames and share its native hit geometry.
// ------------------=
fn action_scene_pixels() {
    use crate::ui::system_layout::SettingsTarget;
    use crate::ui::app_launcher::window_controls as controls;
    controls::publish(None);let _=controls::take_damage();
    for (width,height) in [(1280,800),(1920,1080)] {
        let layout=SystemLayout::new(width,height);
        let mut state=state(3);state.expanded_row=Some(1);
        let maximum=layout.settings_window_geometry_for_section(state,3).maximum_scroll;
        let mut witnessed=false;
        for offset in [0,maximum] {
            state.scroll_offset=offset;
            let geometry=layout.settings_action_geometry(state,3,1).unwrap();
            assert!(geometry.element.is_some());
            let action=geometry.bounds.intersection(geometry.clip);
            let row=layout.settings_row_geometry_for_section(state,1,3).summary;
            let (before,ok)=frame(width,height,3,state,snapshot(),None);assert!(ok);
            let mut changed=snapshot();changed.chat_enabled=false;
            let (after,ok)=frame(width,height,3,state,changed,None);assert!(ok);
            let mut action_damage=0usize;
            for (index,(before,after)) in before.iter().zip(&after).enumerate() {
                if before==after {continue;}
                let point=Point{x:(index%width)as i32,y:(index/width)as i32};
                assert!(row.contains(point)||action.contains(point));
                if action.contains(point) {
                    assert!(geometry.hit(point));
                    action_damage+=1;
                }
            }
            if action.height>=geometry.bounds.height&&action.width>=geometry.bounds.width {
                assert!(action_damage>10);
                witnessed=true;
                let point=Point{x:action.x+action.width as i32/2,y:action.y+action.height as i32/2};
                assert_eq!(layout.settings_target_for_section(point.x*1000/width as i32,
                    point.y*1000/height as i32,state,3),Some(SettingsTarget::ExpandedAction));
                controls::publish(Some(controls::Highlight{rect:geometry.bounds,pressed:false}));
                let (hover,ok)=frame(width,height,3,state,snapshot(),None);assert!(ok);
                let (hover_bounds,hover_count)=damage(&before,&hover,width);
                assert!(hover_count>10);
                assert_eq!(hover_bounds.intersection(action),hover_bounds);
                controls::publish(Some(controls::Highlight{rect:geometry.bounds,pressed:true}));
                let (pressed,ok)=frame(width,height,3,state,snapshot(),None);assert!(ok);
                let (pressed_bounds,pressed_count)=damage(&hover,&pressed,width);
                assert!(pressed_count>10);
                assert_eq!(pressed_bounds.intersection(action),pressed_bounds);
                controls::publish(None);let _=controls::take_damage();
                let (released,ok)=frame(width,height,3,state,snapshot(),None);assert!(ok);
                assert_eq!(released,before);
            }
        }
        assert!(witnessed);
    }
}
