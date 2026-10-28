use super::*;
use crate::ui::{geometry::Rect,installer_template::navigation};
impl DisplayDevice {
    // ------------------------=
    // FUNC: authored_infinity_menu
    // DESC: Paints the validated hierarchy with shared Precision menus, disabled/check states and bounded paging.
    // ------------------=
    pub(super) fn authored_infinity_menu(&mut self,scale:usize)->bool {
        let Some(menu)=crate::ui::desktop_template::menu()else{return false;};let view=navigation::view();
        let (x,y,w,h,count)=crate::ui::system_layout::SystemLayout::new(self.width,self.height).system_menu_geometry(0);
        self.glass_panel(x,y,w,h,true);
        let selection=self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        for row in 0..count {
            let logical=view.offset+row;let rect=Rect{x:(x+7*scale)as i32,y:(y+(11+row*34)*scale)as i32,width:w.saturating_sub(14*scale)as u32,height:30*scale as u32};
            let item=view.item(&menu,logical).map(|i|menu.items[i]);
            if item.is_some_and(|i|i.action==6){self.fill_rect(x+16*scale,rect.y.max(0)as usize+15*scale,w.saturating_sub(32*scale),scale,63,78,96);continue;}
            let enabled=view.selectable(&menu,logical);
            if enabled&&logical==view.selected{self.fill_rounded_rect_alpha(rect.x.max(0)as usize,rect.y.max(0)as usize,rect.width as usize,rect.height as usize,4*scale,selection.0,selection.1,selection.2,224);}
            let label=item.map(|i|i.label).unwrap_or(if view.parent.is_some(){b"Back"}else{b"No menu items"});
            let color=if enabled{(231,240,248)}else{(125,141,157)};
            self.app_label(Rect{x:rect.x+28*scale as i32,width:rect.width.saturating_sub(60*scale as u32),..rect},label,color,false,scale);
            if item.is_some_and(|i|i.checked){self.ui_text(rect.x.max(0)as usize+8*scale,rect.y.max(0)as usize+5*scale,b"*",color.0,color.1,color.2,1);}
            if item.is_some_and(|i|i.action==5){self.ui_text(rect.right().max(0)as usize-20*scale,rect.y.max(0)as usize+5*scale,b">",color.0,color.1,color.2,1);}
        }
        if view.offset>0{self.ui_text(x+w/2,y,b"^",134,199,255,1);}
        if view.offset+count<view.count(&menu){self.ui_text(x+w/2,y+h-12*scale,b"v",134,199,255,1);}
        true
    }
    // ------------------------=
    // FUNC: authored_desktop_content
    // DESC: Fits the same authored scene into a real compositor-owned window, preserving text, images and glass surfaces.
    // ------------------=
    pub(super) fn authored_desktop_content(&mut self,scene:u8,content:Rect)->bool {
        let Some(template)=crate::ui::desktop_template::template()else{return false;};let Some(count)=template.element_count(scene)else{return false;};
        let c=self.active_accent_surface(crate::ui::skin::AccentSurface::Widget);
        self.fill_rect(content.x.max(0)as usize,content.y.max(0)as usize,content.width as usize,content.height as usize,c.0,c.1,c.2);
        for i in 0..count {
            let Some(e)=template.layer_at(scene,i)else{continue;};if e.hidden||e.opacity==0{continue;}
            let mut r=crate::ui::installer_layout::scale_template_rect(e.frame,content.width as usize,content.height as usize);
            r.left+=content.x.max(0)as usize;r.top+=content.y.max(0)as usize;
            let old=self.render_clip;self.intersect_render_clip(content.x.max(0)as usize,content.y.max(0)as usize,content.width as usize,content.height as usize);
            self.template_element_in_rect(e,template.asset(e.image_asset),None,r);self.render_clip=old;
        }true
    }
}
