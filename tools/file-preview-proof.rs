//! Exercises the production preview, frame, typography and control painters on the shared host framebuffer.
// ------------------------=
// FUNC: render
// DESC: Captures the production text-preview composition for review without claiming a VM screenshot.
// ------------------=
pub fn render(){use crate::ui::file_preview as p;use crate::ui::geometry::Rect;let mut state=p::Preview::new();state.open=true;state.place(1024,768,1);state.kind=p::Kind::Text;
    let path=b"/home/default/documents/readme.md";state.path[..path.len()].copy_from_slice(path);state.path_len=path.len();
    let text=b"# InfinityOS project notes\n\nPreview documents without opening an editor.\nYour original file stays unchanged.\n\nFeatures\n- Right-click a file and choose Preview\n- Scroll long documents with the wheel or rail\n- Move the title bar and resize the side edges\n- Press Escape to return to your workspace\n\nCommand line\npreview /home/default/documents/readme.md\n";
    state.length=p::load(|b|{b[..text.len()].copy_from_slice(text);Some(text.len())}).unwrap();state.lines=p::text_rows(text,state.columns(),|_,_|{});p::publish(state);
    let mut pixels=vec![0x000c0c0cu32;1024*768];
    let mut d=super::DisplayDevice{buffer:pixels.as_mut_ptr(),width:1024,height:768,stride:1024,format:0,
        render_clip:None,fast_motion_frame:false,submissions:0,template_typography:[0;3],recording_surface:false};d.file_preview_overlay();
    use std::io::Write;let mut out=std::fs::File::create("builds/file-preview-proof.ppm").unwrap();write!(out,"P6\n1024 768\n255\n").unwrap();for pixel in &pixels {out.write_all(&[*pixel as u8,(*pixel>>8) as u8,(*pixel>>16) as u8]).unwrap();}p::close();
    state=p::Preview::new();state.open=true;state.menu=true;state.rect=Rect{x:40,y:40,width:300,height:72};
    let path=b"/home/default/pictures/sky.png";state.path[..path.len()].copy_from_slice(path);state.path_len=path.len();p::publish(state);
    pixels.fill(0x000c0c0c);d.file_preview_overlay();let mut out=std::fs::File::create("builds/wallpaper-menu-proof.ppm").unwrap();write!(out,"P6\n1024 768\n255\n").unwrap();for pixel in &pixels {out.write_all(&[*pixel as u8,(*pixel>>8) as u8,(*pixel>>16) as u8]).unwrap();}p::close();
}
