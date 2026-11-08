//! Production Task Manager paint and interaction geometry acceptance fixtures.
use super::{DisplayDevice,Region};
use crate::ui::{geometry::{Rect,Point},task_management::{self as model,Entry,Feedback,Mode,Tab,Target,Text,View}};

// ------------------------=
// FUNC: fixture
// DESC: Supplies explicit catalog values to the real painter without claiming a native process is running.
// ------------------=
fn fixture(tab:Tab)->View {
    let mut v=View::new();v.available=true;v.can_manage=true;v.access_reason=Feedback::None;v.state.tab=tab;v.count=3;v.total=3;
    for i in 0..3 {v.entries[i]=Entry{id:i as u32+1,owner:[1;16],can_output:true,next_status:if i==0{model::NextRun::Running}else if i==2{model::NextRun::Disabled}else{model::NextRun::Scheduled},name:Text::from_bytes([b"archive-index".as_slice(),b"workspace-backup",b"cache-refresh"][i]),path:Text::from_bytes(b"/home/default/scripts/workspace.py"),enabled:i!=2,trigger:if tab==Tab::Services{1}else{2},timing:60,restart:if tab==Tab::Services{1}else{0},state:if i==0{2}else{0},next:1794189600,timing_text:model::Field::from_bytes(b"2026-11-09 02:00:00"),next_text:Text::from_bytes(b"2026-11-09 02:00:00"),exit:if i==1{Some(0)}else{None},runs:12};}
    v
}
// ------------------------=
// FUNC: frame
// DESC: Renders the unchanged production painter into a host memory framebuffer.
// ------------------=
fn frame(width:usize,height:usize,view:&View,clip:Option<Region>)->Vec<u32> {
    let mut pixels=vec![0x00090f26;width*height];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:1,render_clip:clip,fast_motion_frame:false,submissions:0,template_typography:[0;3],recording_surface:false};
    let toolbar=Rect{x:16,y:16,width:(width-32)as u32,height:44};let content=Rect{x:16,y:60,width:(width-32)as u32,height:(height-76)as u32};
    display.render_task_management_toolbar(content,toolbar,1,view,false);
    assert!(display.render_task_automation(content,toolbar,1,view));pixels
}
// ------------------------=
// FUNC: installed_frame
// DESC: Renders the shared production toolbar and workspace using the installed desktop's actual window geometry.
// ------------------=
fn installed_frame(view:&View,task_menu_open:bool)->Vec<u32> {
    let(width,height)=(1024,768);let mut pixels=vec![0x00090f26;width*height];
    let mut display=DisplayDevice{buffer:pixels.as_mut_ptr(),width,height,stride:width,format:1,render_clip:None,fast_motion_frame:false,submissions:0,template_typography:[0;3],recording_surface:false};
    let g=crate::ui::system_layout::SystemLayout::new(width,height).desktop_app_window_geometry(160,140,760,650,false);
    display.fill_rect_alpha(g.toolbar.x as usize,g.toolbar.y as usize,g.toolbar.width as usize,g.toolbar.height as usize,5,22,38,224);
    display.render_task_management_toolbar(g.content,g.toolbar,1,view,task_menu_open);
    display.render_task_automation(g.content,g.toolbar,1,view);pixels
}
// ------------------------=
// FUNC: compact_installed_geometry
// DESC: Verifies actual desktop dimensions preserve independent toolbar, action and feedback pixels across every automation mode.
// ------------------=
fn compact_installed_geometry() {
    let g=crate::ui::system_layout::SystemLayout::new(1024,768).desktop_app_window_geometry(160,140,760,650,false);
    assert_eq!((g.content.x,g.content.y,g.content.width,g.content.height),(179,209,746,381));
    for tab in [Tab::Services,Tab::Scheduled] {for mode in [Mode::Browse,Mode::Create,Mode::Edit,Mode::Delete] {
        let mut view=fixture(tab);view.select(1,4);
        if mode!=Mode::Browse{view.state.begin(mode,view.selected());}
        if matches!(mode,Mode::Create|Mode::Edit){view.state.fields[0]=model::Field::from_bytes(b"workspace-backup");view.state.fields[1]=model::Field::from_bytes(b"/home/ms9node01/documents/hello.py");view.state.fields[2]=model::Field::from_bytes(b"*/15 * * * *");if tab==Tab::Scheduled{view.state.trigger=4;}}
        let layout=model::layout(g.content,g.toolbar,&view,1);assert!(layout.compact);
        for c in &layout.controls[..layout.count] {if c.bounds.y>=g.content.y {
            assert!(g.content.contains(Point{x:c.bounds.x,y:c.bounds.y}));assert!(g.content.contains(Point{x:c.bounds.right()-1,y:c.bounds.bottom()-1}));
            assert!(c.bounds.bottom()<=layout.status.y);
            if matches!(c.target,Target::Save|Target::Cancel|Target::Output|Target::Delete){assert!(c.bounds.bottom()<=layout.detail.bottom()-4);}
            if c.enabled{assert_eq!(layout.hit(Point{x:c.bounds.x+1,y:c.bounds.y+1}),Some(c.target));}
        }}
        let pixels=installed_frame(&view,false);assert_eq!(pixels,installed_frame(&view,true));
        let mut feedback=view;feedback.state.feedback=Feedback::Storage;let changed=installed_frame(&feedback,false);assert_ne!(pixels,changed);
        for c in &layout.controls[..layout.count] {for y in c.bounds.y..c.bounds.bottom(){for x in c.bounds.x..c.bounds.right(){assert_eq!(pixels[y as usize*1024+x as usize],changed[y as usize*1024+x as usize]);}}}
        let tab_name=if tab==Tab::Services{"services"}else{"scheduled"};let mode_name=match mode{Mode::Browse=>"browse",Mode::Create=>"create",Mode::Edit=>"edit",Mode::Delete=>"delete"};
        let mut ppm=b"P6\n1024 768\n255\n".to_vec();for pixel in pixels{ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]);}std::fs::write(format!("build/task-manager-installed-{tab_name}-{mode_name}.ppm"),ppm).unwrap();
    }}
    let mut processes=View::new();processes.state.tab=Tab::Processes;assert_ne!(installed_frame(&processes,false),installed_frame(&processes,true));
}
// ------------------------=
// FUNC: verify
// DESC: Checks real pixels, exact hit geometry, focused fields, disabled actions and dirty draft transitions.
// ------------------=
pub(super) fn verify() {
    compact_installed_geometry();
    for (width,height) in [(620usize,500usize),(900,680)] {
        let toolbar=Rect{x:16,y:16,width:(width-32)as u32,height:44};let content=Rect{x:16,y:60,width:(width-32)as u32,height:(height-76)as u32};
        for(tab,name)in[(Tab::Services,"services"),(Tab::Scheduled,"scheduled")] {
            let mut v=fixture(tab);let layout=model::layout(content,toolbar,&v,1);
            for c in &layout.controls[..layout.count] {if c.enabled {assert_eq!(layout.hit(Point{x:c.bounds.x+c.bounds.width as i32/2,y:c.bounds.y+c.bounds.height as i32/2}),Some(c.target));}}
            assert!(!layout.controls[..layout.count].iter().find(|c|c.target==Target::Edit).unwrap().enabled);
            let before=frame(width,height,&v,None);
            v.select(1,layout.rows);let selected=frame(width,height,&v,None);assert_ne!(before,selected);
            let selected_entry=v.selected();v.state.begin(Mode::Edit,selected_entry);assert!(!v.state.switch(Tab::Processes));assert_eq!(v.state.feedback,Feedback::SaveOrCancel);
            v.state.feedback=Feedback::None;v.state.focus=Target::Field(1);
            let editor=frame(width,height,&v,None);assert_ne!(editor,selected);
            let mut readonly=v;readonly.can_manage=false;readonly.access_reason=Feedback::ReadOnly;let denied_layout=model::layout(content,toolbar,&readonly,1);assert!(denied_layout.controls[..denied_layout.count].iter().filter(|c|matches!(c.target,Target::Save|Target::Field(_)|Target::Enabled|Target::Trigger)).all(|c|!c.enabled));let denied_pixels=frame(width,height,&readonly,None);assert_ne!(denied_pixels,editor);
            let mut selection=v;selection.state.fields[1].selected=true;let selection_pixels=frame(width,height,&selection,None);assert_ne!(selection_pixels,editor);
            let mut manual=v;manual.state.trigger=0;assert!(!model::layout(content,toolbar,&manual,1).controls.iter().find(|c|c.target==Target::Field(2)).unwrap().enabled);
            let mut stale=v;stale.state.conflict=true;stale.state.feedback=Feedback::Conflict;assert!(!model::layout(content,toolbar,&stale,1).controls.iter().find(|c|c.target==Target::Save).unwrap().enabled);
            let mut choice_view=v;choice_view.state.choice=1;choice_view.state.focus=Target::TriggerOption(v.state.trigger);
            let choices=frame(width,height,&choice_view,None);assert_ne!(choices,editor);
            if tab==Tab::Scheduled{let mut once=v;once.state.trigger=3;once.state.fields[2]=model::Field::from_bytes(b"2026-11-09 02:00:00");let pixels=frame(width,height,&once,None);let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();for pixel in pixels{ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]);}std::fs::write(format!("build/task-manager-once-edit-{width}.ppm"),ppm).unwrap();}
            if tab==Tab::Scheduled{let mut cron=v;cron.state.trigger=4;cron.state.fields[2]=model::Field::from_bytes(b"*/15 * * * *");cron.state.focus=Target::Field(2);let pixels=frame(width,height,&cron,None);let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();for pixel in pixels{ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]);}std::fs::write(format!("build/task-manager-cron-edit-{width}.ppm"),ppm).unwrap();}
            let choice_layout=model::layout(content,toolbar,&choice_view,1);
            for c in &choice_layout.controls[..choice_layout.count]{if matches!(c.target,Target::TriggerOption(_)){assert_eq!(choice_layout.hit(Point{x:c.bounds.x+c.bounds.width as i32/2,y:c.bounds.y+c.bounds.height as i32/2}),Some(c.target));}}
            let l=model::layout(content,toolbar,&v,1);
            for c in &l.controls[..l.count] {if c.bounds.y>=content.y {assert!(content.contains(Point{x:c.bounds.x,y:c.bounds.y}));assert!(content.contains(Point{x:c.bounds.right()-1,y:c.bounds.bottom()-1}));}}
            let save=l.controls[..l.count].iter().find(|c|c.target==Target::Save).unwrap();assert!(save.enabled);assert!(save.bounds.bottom()<l.status.y);
            v.state.focus=Target::None;let mut visited=0;for _ in 0..l.count {l.tab(&mut v.state,false);if v.state.focus==Target::Save{visited+=1;}}assert!(visited>0);
            let clip=Region{left:30,top:80,right:width-30,bottom:height-30};let full=frame(width,height,&v,None);let partial=frame(width,height,&v,Some(clip));
            for y in 0..height{for x in 0..width{assert_eq!(partial[y*width+x],if x>=clip.left&&x<clip.right&&y>=clip.top&&y<clip.bottom{full[y*width+x]}else{0x00090f26});}}
            for (suffix,pixels) in [("browse",selected),("edit",editor),("choices",choices),("readonly",denied_pixels),("selection",selection_pixels)] {let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();for pixel in pixels{ppm.extend_from_slice(&[(pixel>>16)as u8,(pixel>>8)as u8,pixel as u8]);}std::fs::write(format!("build/task-manager-{name}-{suffix}-{width}.ppm"),ppm).unwrap();}
            v.state.cancel();assert!(v.state.switch(Tab::Processes));assert_eq!(v.state.mode,Mode::Browse);
        }
    }
    let mut t=Text::from_bytes(b"123");t.caret=1;t.edit(crate::ui::text_input::TextEditKey::Character(b'4'));assert_eq!(model::decimal(&t),Some(1423));t.edit(crate::ui::text_input::TextEditKey::Backspace);assert_eq!(model::decimal(&t),Some(123));
    assert_eq!(model::decimal(&Text::from_bytes(b"18446744073709551616")),None);
    let field=Text::from_bytes(b"/home/default/Wide and narrow iii.py");let first=model::field_first(&field,140,1);assert!(first>0);assert_eq!(model::field_caret(&field,140,0,1),first);assert_eq!(model::field_caret(&field,140,1000,1),field.len);let width=model::field_text_width(&field.bytes()[first..first+3],1);assert_eq!(model::field_caret(&field,140,width as i32,1),first+3);
    println!("Task Manager production pixels, clip parity, field state, keyboard reachability and draft preservation passed");
}
