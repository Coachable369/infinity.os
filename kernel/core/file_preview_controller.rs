//! One authenticated, read-only preview entry point for shell and desktop callers.
use super::*;
use crate::ui::file_preview::{self as preview, Kind, Preview};
use crate::ui::geometry::{Point,Rect};
// ------------------------=
// FUNC: inflate_preview
// DESC: Uses bounded no-allocator zlib decompression and requires exact stream consumption.
// ------------------=
fn inflate_preview(input:&[u8],output:&mut [u8])->bool {
    let mut inflater=miniz_oxide::inflate::core::DecompressorOxide::new();
    let (status,read,written)=miniz_oxide::inflate::core::decompress(&mut inflater,input,output,0,5);
    status==miniz_oxide::inflate::TINFLStatus::Done&&read==input.len()&&written==output.len()
}
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: refresh_desktop_preview_files
    // DESC: Projects authorized regular files in the existing Desktop namespace into bounded native tiles.
    // ------------------=
    pub(super) fn refresh_desktop_preview_files(&self) {
        let mut files=[preview::EMPTY_FILE;16];let mut count=0;
        if self.workplace_authorized(false){for i in 0..256{
            let Ok(Some(entry))=crate::storage::namespace_list_nth(b"/home/default/desktop",i) else{break;};
            let path=&entry.path[..entry.path_len as usize];let Ok((m,_))=crate::storage::object_inspect_path(path)else{continue;};
            if m.kind==crate::storage::object::ObjectType::NamespaceNode||!preview::can_read(true,m.space==crate::storage::object::Space::Personal,m.owner.0,self.current_user.0){continue;}
            if path.len()>96{continue;}files[count].path[..path.len()].copy_from_slice(path);files[count].length=path.len();count+=1;if count==files.len(){break;}
        }}preview::set_desktop_files(files);
    }
    // ------------------------=
    // FUNC: preview_file
    // DESC: Resolves relative paths and reads only authorized Personal Space content without invoking a file association.
    // ------------------=
    pub(super) fn preview_file(&mut self,path:&[u8]) {
        self.load_preview_file(path,true);
    }
    // ------------------------=
    // FUNC: load_preview_file
    // DESC: Shares authenticated decoding between read-only preview and wallpaper selection.
    // ------------------=
    fn load_preview_file(&mut self,path:&[u8],show:bool) {
        preview::close();
        if !self.workplace_authorized(false) {self.output.write_line(b"Preview requires an authorized session.");return;}
        let path=if path.starts_with(b"/"){crate::runtime::object_navigation::normalize_absolute_path(path)}
            else{crate::runtime::object_navigation::join_path(self.navigation_context.path(),path)};
        let Ok(path)=path else {self.output.write_line(b"Invalid preview path.");return;};
        let path=path.as_bytes();let mut state=Preview::new();state.open=true;state.owner=self.current_session.0;
        state.path[..path.len()].copy_from_slice(path);state.path_len=path.len();
        state.place(self.system.framebuffer_width,self.system.framebuffer_height,SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).scale());
        if let Ok((metadata,_))=crate::storage::object_inspect_path(path) {
            if preview::can_read(true,metadata.space==crate::storage::object::Space::Personal,metadata.owner.0,self.current_user.0) {
                if metadata.kind==crate::storage::object::ObjectType::NamespaceNode {
                    state.kind=Kind::Directory;
                    state.length=preview::load(|out|{let mut n=0;
                        for i in 0..256 {let Ok(Some(entry))=crate::storage::namespace_list_nth(path,i) else{break;};
                            let Ok((child,_))=crate::storage::object_inspect_path(&entry.path[..entry.path_len as usize]) else{continue;};
                            if !preview::can_read(true,child.space==crate::storage::object::Space::Personal,child.owner.0,self.current_user.0){continue;}
                            let name=crate::runtime::object_navigation::namespace_basename(&entry.path[..entry.path_len as usize]);
                            if n+name.len()+1>out.len(){break;}out[n..n+name.len()].copy_from_slice(name);n+=name.len();out[n]=b'\n';n+=1;
                        }Some(n)}).unwrap_or(0);
                }else if metadata.logical_size as usize>preview::CAPACITY {state.kind=Kind::TooLarge;}
                else if let Some(n)=preview::load(|out|crate::storage::object_read_path(path,None,out).ok().map(|(_,n)|n).filter(|n|*n==metadata.logical_size as usize)) {
                    state.length=n;preview::publish(state);state.kind=preview::content(preview::classify);
                    if state.kind==Kind::Unsupported && preview::decode_png(inflate_preview){state.kind=Kind::Png;}
                }
            }
        }
        state.open=show;
        if show&&!matches!(self.mode,ConsoleMode::Desktop|ConsoleMode::Settings) {self.open_command_window();}
        preview::publish(state);self.reflow_preview();if show{self.redraw();}
    }
    // ------------------------=
    // FUNC: set_desktop_wallpaper
    // DESC: Validates source access and decoding before saving a per-user reference and publishing cover artwork.
    // ------------------=
    pub(super) fn set_desktop_wallpaper(&mut self,path:&[u8],save:bool){
        if !self.workplace_authorized(save){return;}
        self.load_preview_file(path,false);let state=preview::current();
        if !matches!(state.kind,Kind::Bitmap|Kind::Png){if save{self.preview_file(path);}else{preview::close();}return;}
        if save&&crate::storage::wallpaper_preference(self.current_user.0,self.current_session.0,Some(&state.path[..state.path_len])).is_err(){
            self.output.write_line(b"Wallpaper could not be saved. The previous wallpaper is unchanged.");preview::close();return;
        }
        let owner=self.current_user.0;
        // Bound the retained cache to 1920x1080 while preserving source proportions.
        let install=|w:usize,h:usize,pixel:&mut dyn FnMut(usize,usize)->Option<[u8;4]>|{
            let divisor=((w+1919)/1920).max((h+1079)/1080).max(1);let dw=(w/divisor).max(1);let dh=(h/divisor).max(1);
            crate::ui::user_wallpaper::install(owner,dw,dh,|x,y|pixel(x*w/dw,y*h/dh))
        };
        if state.kind==Kind::Bitmap{preview::content(|b|{if let Some(b)=crate::ui::bitmap::RuntimeBitmap::parse(b){install(b.width(),b.height(),&mut|x,y|b.rgba(x,y));}});}
        else{preview::png(|p,raw|{install(p.width,p.height,&mut|x,y|p.rgba(raw,x,y));});}
        preview::close();if save{self.redraw();}
    }
    // ------------------------=
    // FUNC: restore_desktop_wallpaper
    // DESC: Restores only an accessible saved image, falling back to packaged artwork if missing or unsupported.
    // ------------------=
    pub(super) fn restore_desktop_wallpaper(&mut self){
        crate::ui::user_wallpaper::session([0;16],false);
        if let Ok((path,n))=crate::storage::wallpaper_preference(self.current_user.0,self.current_session.0,None){self.set_desktop_wallpaper(&path[..n],false);}
    }
    // ------------------------=
    // FUNC: reflow_preview
    // DESC: Recounts wrapped rows after resize and clamps the retained scroll offset.
    // ------------------=
    fn reflow_preview(&mut self) {let mut p=preview::current();p.lines=preview::content(|b|preview::text_rows(b,p.columns(),|_,_|{}));p.scroll_by(0);preview::publish(p);}
    // ------------------------=
    // FUNC: preview_key
    // DESC: Handles preview-only navigation without changing or saving the underlying file.
    // ------------------=
    pub(super) fn preview_key(&mut self,key:ConsoleKey)->bool {let mut p=preview::current();if !p.open{return false;}
        if !self.workplace_authorized(false)||p.owner!=self.current_session.0 {preview::close();return false;}
        match key {ConsoleKey::Escape=>preview::close(),ConsoleKey::Up=>{p.scroll_by(-1);preview::publish(p);},
            ConsoleKey::Down=>{p.scroll_by(1);preview::publish(p);},ConsoleKey::Home=>{p.scroll=0;preview::publish(p);},
            ConsoleKey::End=>{p.scroll=p.lines;p.scroll_by(0);preview::publish(p);},
            ConsoleKey::Enter if p.menu=>self.preview_file(&p.path[..p.path_len]),_=>{}}
        true
    }
    // ------------------------=
    // FUNC: preview_pointer
    // DESC: Captures sheet close, scrolling, title dragging and side/bottom resizing using the renderer's bounds.
    // ------------------=
    pub(super) fn preview_pointer(&mut self,clicked:bool,left:bool)->bool {
        let mut v=preview::current();if !v.open{return false;}
        if !self.workplace_authorized(false)||v.owner!=self.current_session.0 {preview::close();return false;}
        let p=Point{x:self.system.framebuffer_width as i32*self.pointer_x/1000,y:self.system.framebuffer_height as i32*self.pointer_y/1000};
        if v.menu {if clicked {if v.rect.contains(p){if p.y>=v.rect.y+36*v.scale as i32&&crate::ui::user_wallpaper::image_path(&v.path[..v.path_len]){self.set_desktop_wallpaper(&v.path[..v.path_len],true);}else{self.preview_file(&v.path[..v.path_len]);}}else{preview::close();}}return true;}
        if clicked && v.close_rect().contains(p) {preview::close();return true;}
        if let Some((origin,r,edge))=v.drag {
            if left {let dx=p.x-origin.x;let dy=p.y-origin.y;
                if edge==8 {v.scroll_to_pointer(p);}
                else if edge==0 {v.rect.x=(r.x+dx).clamp(0,(self.system.framebuffer_width as i32-r.width as i32).max(0));v.rect.y=(r.y+dy).clamp(0,(self.system.framebuffer_height as i32-r.height as i32).max(0));}
                else {let minw=(320*v.scale).min(self.system.framebuffer_width) as i32;let minh=(240*v.scale).min(self.system.framebuffer_height) as i32;
                    if edge&1!=0 {v.rect.x=(r.x+dx).clamp(0,(r.right()-minw).max(0));v.rect.width=(r.right()-v.rect.x) as u32;}
                    if edge&2!=0 {v.rect.width=(r.width as i32+dx).clamp(minw,(self.system.framebuffer_width as i32-r.x).max(minw)) as u32;}
                    if edge&4!=0 {v.rect.height=(r.height as i32+dy).clamp(minh,(self.system.framebuffer_height as i32-r.y).max(minh)) as u32;}
                }
            }else{v.drag=None;}preview::publish(v);self.reflow_preview();return true;
        }
        if clicked && v.rect.contains(p) {let s=v.scale as i32;let edge=if p.x<v.rect.x+8*s {1}else if p.x>=v.rect.right()-8*s{2}else{0} | if p.y>=v.rect.bottom()-8*s{4}else{0};
            if v.track().contains(p) {v.scroll_to_pointer(p);v.drag=Some((p,v.rect,8));}
            else if edge!=0 || p.y<v.rect.y+48*s {v.drag=Some((p,v.rect,edge));}
            preview::publish(v);
        }true
    }
    // ------------------------=
    // FUNC: desktop_preview_menu
    // DESC: Reuses the native compact context recipe for existing desktop namespace references.
    // ------------------=
    pub(super) fn desktop_preview_menu(&mut self)->bool {
        if self.mode!=ConsoleMode::Desktop || !self.workplace_authorized(false){return false;}
        let l=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let p=Point{x:self.system.framebuffer_width as i32*self.pointer_x/1000,y:self.system.framebuffer_height as i32*self.pointer_y/1000};
        let(e,c,t)=self.desktop_app_windows();for app in [e,c,t,self.browser_window_state()]{if app.visible&&l.desktop_app_window_geometry(app.x,app.y,app.width,app.height,app.maximized).window.contains(p){return false;}}
        if self.settings_open&&l.settings_window_geometry(self.settings_window).window.contains(p){return false;}
        if self.home_window_visible {let(x,y,w,h)=l.home_window_geometry_sized(self.home_window_x,self.home_window_y,self.home_window_width,self.home_window_height,self.home_window_maximized);if (Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32}).contains(p){return false;}}
        if self.inactive_file_navigator_at_pointer().is_some(){return false;}
        self.refresh_desktop_preview_files();let files=preview::desktop_files();
        let selected=files.iter().enumerate().find(|(i,f)|{let r=preview::desktop_rect(*i,self.system.framebuffer_width,self.system.framebuffer_height);f.length!=0&&(r.bottom() as usize)<self.system.framebuffer_height.saturating_sub(100*l.scale())&&r.contains(p)});
        let path=if let Some((_,f))=selected {&f.path[..f.length]}else{
            let Some(item)=self.desktop_item_at_pointer() else{return false;};
            let Some((_,path))=Self::desktop_reference_paths(item) else{return false;};path
        };
        let mut v=Preview::new();v.open=true;v.menu=true;v.owner=self.current_session.0;
        v.path[..path.len()].copy_from_slice(path);v.path_len=path.len();
        v.scale=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).scale();
        let image=crate::ui::user_wallpaper::image_path(path);
        let w=(300*v.scale).min(self.system.framebuffer_width);let h=if image{72*v.scale}else{40*v.scale};
        v.rect=Rect{x:(self.system.framebuffer_width as i32*self.pointer_x/1000).clamp(0,(self.system.framebuffer_width-w) as i32),y:(self.system.framebuffer_height as i32*self.pointer_y/1000).clamp(0,self.system.framebuffer_height.saturating_sub(h) as i32),width:w as u32,height:h as u32};
        preview::publish(v);true
    }
}
