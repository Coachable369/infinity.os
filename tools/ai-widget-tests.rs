//! Behavioral text/menu and production pixel checks; called by the existing active-painter harness.
use super::*;
use ui::{ai_widget as widget, geometry::{Point,Rect},system_layout::{SystemLayout,AiChatTarget}};

// ------------------------=
// FUNC: verify
// DESC: Exercises actual native prompt measurements, editing, menu actions and clipped painter output at both UI densities.
// ------------------=
pub fn verify() {
    verify_input();verify_menu();
    for (width,height) in [(1024,768),(1920,1080),(2560,1600)] { verify_pixels(width,height); }
    widget::publish_menu(Default::default());
}

// ------------------------=
// FUNC: verify_input
// DESC: Keeps a long edited prompt intact while every caret and pointer position stays in its measured viewport.
// ------------------=
fn verify_input() {
    let mut bytes=[0u8;4096];let mut length=0;let mut cursor=0;
    for &byte in b"Wi AV ii wide words - a long graphical calendar request. ".iter().cycle().take(3000) {
        assert!(ui::text_input::insert_ascii(&mut bytes,&mut length,&mut cursor,byte));
    }
    for scale in [1,2] {for width in [70,130,220] {
        for index in [0,1,2,10,500,2999,3000] {
            cursor=index;
            let view=widget::text_viewport(&bytes[..length],cursor,width*scale,scale);
            assert!(view.start<=cursor && view.end>=cursor && view.end<=length);
            assert!(view.caret_x+2*scale<=width*scale);
            assert_eq!(view.caret_x,widget::text_width(&bytes[view.start..cursor],scale));
            assert_eq!(widget::caret_at(&bytes[..length],view,view.caret_x as i32,scale),cursor);
            for boundary in view.start..=view.end {
                let x=widget::text_width(&bytes[view.start..boundary],scale);
                assert_eq!(widget::caret_at(&bytes[..length],view,x as i32,scale),boundary);
                let retained=widget::retained_viewport(&bytes[..length],boundary,width*scale,scale,view.start);
                assert_eq!(retained.start,view.start);
                assert_eq!(retained.caret_x,x);
            }
        }
    }}
    cursor=length;
    assert!(ui::text_input::move_caret(&mut cursor,length,-2));assert_eq!(cursor,0);
    assert!(ui::text_input::insert_ascii(&mut bytes,&mut length,&mut cursor,b'Z'));
    assert_eq!(bytes[0],b'Z');assert_eq!(cursor,1);
    assert!(ui::text_input::delete(&mut bytes,&mut length,&mut cursor));assert_eq!(bytes[1],b'i');
    assert!(ui::text_input::backspace(&mut bytes,&mut length,&mut cursor));assert_eq!(cursor,0);
    assert!(ui::text_input::move_caret(&mut cursor,length,2));assert_eq!(cursor,length);
    ui::text_input::set_presentation(true,true,cursor,3,true);
    assert_eq!(ui::text_input::caret(3),Some((true,cursor)));
    assert_eq!(ui::text_input::caret(5),None);
    ui::text_input::set_presentation(false,false,0,0,false);
    assert_eq!(ui::text_input::caret(3),None);
}

// ------------------------=
// FUNC: verify_menu
// DESC: Asserts keyboard selection, disabled Stop and exact edge-contained pointer actions without text oracles.
// ------------------=
fn verify_menu() {
    let mut state=widget::MenuState {open:true,selected:0};
    assert_eq!(state.action(false),Some(widget::MenuAction::Voice));
    state.move_selection(false);assert_eq!(state.action(false),None);
    assert_eq!(state.action(true),Some(widget::MenuAction::Stop));
    state.move_selection(false);assert_eq!(state.action(false),Some(widget::MenuAction::Settings));
    state.move_selection(false);assert_eq!(state.selected,0);
    state.move_selection(true);assert_eq!(state.selected,2);
    state.open=false;assert_eq!(state.action(true),None);
    for (width,height) in [(1024,768),(1920,1080),(2560,1600)] {
        let layout=SystemLayout::new(width,height);let s=layout.scale();
        for minimized in [false,true] {
            let g=layout.ai_chat_geometry(minimized);
            for (rect,target) in [(g.status,AiChatTarget::Status),(g.close,AiChatTarget::Close),(g.minimize,AiChatTarget::Minimize)] {
                let p=Point{x:rect.x+rect.width as i32/2,y:rect.y+rect.height as i32/2};
                assert_eq!(layout.ai_chat_target(p.x*1000/width as i32,p.y*1000/height as i32,minimized),Some(target));
            }
            assert!(g.status.right()<=g.panel.right() && g.status.bottom()<=g.header.bottom());
            assert!(!g.status.intersects(g.close));
            let anchor=Rect{x:width as i32-30,y:height as i32-30,width:25,height:25};
            let menu=widget::MenuGeometry::new(anchor,width,height,s);
            assert!(menu.panel.x>=0 && menu.panel.y>=0 && menu.panel.right()<=width as i32 && menu.panel.bottom()<=height as i32);
            for (index,row) in menu.rows.iter().enumerate() {
                assert_eq!(menu.hit(Point{x:row.x+1,y:row.y+1}),Some(index));
            }
            assert_eq!(menu.hit(Point{x:menu.panel.x,y:menu.panel.y}),None);
        }
    }
}

// ------------------------=
// FUNC: verify_pixels
// DESC: Renders production controls and asserts long input and caret pixels never escape the input field.
// ------------------=
fn verify_pixels(width:usize,height:usize) {
    use std::io::Write;
    let layout=SystemLayout::new(width,height);let scale=layout.scale();let g=layout.ai_chat_geometry(false);
    let mut pixels=vec![0x201207u32;width*height];
    let mut display=DisplayDevice {buffer:pixels.as_mut_ptr(),width,height,stride:width,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,recording_surface:false};
    let text=b"Create a graphical calendar with a seven-column month grid and previous and next month buttons and editable event notes app.";
    let accent=(70,195,240);
    display.app_card(g.panel,(6,21,35),(47,91,118),scale);
    let before=pixels.clone();
    display.ai_widget_composer(g.composer,text,text.len(),Some((true,text.len())),scale,accent);
    let view=widget::text_viewport(text,text.len(),g.composer.width as usize-24*scale,scale);
    assert!(view.start>0);
    assert_eq!(widget::text_width(&text[view.start..view.end],scale),display.app_text_width(&text[view.start..view.end],false,scale));
    let caret_x=g.composer.x as usize+12*scale+view.caret_x;
    let caret_y=g.composer.y as usize+(g.composer.height as usize-20*scale)/2;
    let caret_pixel=accent.0 as u32 | ((accent.1 as u32)<<8) | ((accent.2 as u32)<<16);
    assert_eq!(pixels[caret_y*width+caret_x],caret_pixel);
    assert!(g.composer.contains(Point{x:caret_x as i32,y:caret_y as i32}));
    let mut changed=0;
    for y in 0..height {for x in 0..width {
        if pixels[y*width+x]!=before[y*width+x] {
            changed+=1;assert!(g.composer.contains(Point{x:x as i32,y:y as i32}));
        }
    }}
    assert!(changed>100);
    let menu=widget::MenuState{open:true,selected:1};widget::publish_menu(menu);
    display.ai_widget_header(g,b"Generating",false,true,scale,accent);
    display.app_card(g.send,accent,(130,223,247),scale);
    display.app_symbol(g.send,b'^',(3,25,41),scale);
    display.ai_widget_status_menu(g.status,menu,true,true,scale);
    std::fs::create_dir_all("builds/validation/ai-widget").unwrap();
    let mut file=std::fs::File::create(format!("builds/validation/ai-widget/controls-{width}x{height}.ppm")).unwrap();
    write!(file,"P6\n{width} {height}\n255\n").unwrap();
    for pixel in pixels {file.write_all(&[pixel as u8,(pixel>>8)as u8,(pixel>>16)as u8]).unwrap();}
}
