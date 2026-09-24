use super::DisplayDevice;
use crate::ui::{geometry::Rect, input_preferences, personalization::{Layout,CURSOR_NAMES,TINT_NAMES,TINTS}};
impl DisplayDevice {
    // ------------------------=
    // FUNC: personalization_label
    // DESC: Uses 16-pixel kit text with a clipped right gutter and the shared native font rasterizer.
    // ------------------=
    fn personalization_label(&mut self,x:usize,y:usize,width:usize,text:&[u8],r:u8,g:u8,b:u8) {
        let clip=self.render_clip;
        self.intersect_render_clip(x,y,width,22*self.ui_scale());
        self.ui_text_raster(x,y,text,r,g,b,super::primitives::FontSize::new(super::primitives::UI_FONT_NATIVE_SIZE_PX,16*self.ui_scale()),true,None);
        self.render_clip=clip;
    }
    // ------------------------=
    // FUNC: settings_personalization_panel
    // DESC: Renders native tint sliders and the exact runtime cursor art using shared clipped hit geometry.
    // ------------------=
    pub(super) fn settings_personalization_panel(&mut self, detail: Rect, scale: usize, pointer: bool) {
        let controls=Layout::new(detail,scale,pointer);
        let prefs=input_preferences::current();
        let tint=crate::ui::spatial::backdrop_tint();
        let labels: [&[u8];4]=if pointer {[b"Pointer speed",b"Cursor size",b"",b""]} else {[b"Veil red",b"Veil green",b"Veil blue",b"Veil strength"]};
        for i in 0..if pointer {2}else{4} {
            let r=controls.sliders[i];
            let y=r.y;let x=r.x.max(0) as usize;let w=r.width as usize;
            let (value,max)=if pointer {if i==0 {(prefs.speed-1,9)}else{(prefs.cursor_size-1,5)}}else{(tint[i],255)};
            if y>=24*scale as i32 {
                self.personalization_label(x,(y-24*scale as i32) as usize,w.saturating_sub(65*scale),labels[i],189,210,230);
                let n=if pointer {value+1}else{value};
                let digits=[b'0'+n/100,b'0'+n/10%10,b'0'+n%10];
                let start=if n>=100 {0}else if n>=10 {1}else{2};
                self.personalization_label(x+w.saturating_sub(42*scale),(y-24*scale as i32) as usize,42*scale,&digits[start..],80,222,255);
            }
            if y<0 {continue;}
            let y=y as usize;
            let fill=(w.saturating_sub(1)*value as usize/max as usize).max(1);
            self.fill_rounded_rect_alpha(x,y+7*scale,w,6*scale,3*scale,92,129,155,100);
            self.fill_rounded_rect_alpha(x,y+7*scale,fill,6*scale,3*scale,34,211,238,245);
            let thumb=x+fill.saturating_sub(7*scale).min(w.saturating_sub(14*scale));
            self.fill_rounded_rect_alpha(thumb.saturating_sub(3*scale),y,20*scale,20*scale,10*scale,34,211,238,40);
            self.fill_rounded_rect_alpha(thumb,y+3*scale,14*scale,14*scale,7*scale,230,247,255,255);
        }
        for i in 0..if pointer {10}else{5} {
            let r=controls.choices[i];
            if r.y<0 {continue;}
            let (x,y,w,h)=(r.x.max(0) as usize,r.y as usize,r.width as usize,r.height as usize);
            let selected=if pointer {prefs.cursor_style as usize==i}else{tint==TINTS[i]};
            self.fill_rounded_rect_alpha(x,y,w,h,10*scale,15,31,48,244);
            self.fill_rounded_rect_alpha(x+scale,y+scale,w.saturating_sub(2*scale),h/2,9*scale,44,77,103,70);
            let (red,green,blue)=if selected {(34,211,238)}else{(45,72,96)};
            self.outline_rounded_rect(x,y,w,h,10*scale,red,green,blue);
            if pointer {
                let edge=(50*scale).min(w.saturating_sub(16*scale));
                self.cursor_sprite(i,(x+(w-edge)/2) as i32,(y+10*scale) as i32,edge);
            }else{
                let c=TINTS[i];
                self.fill_rounded_rect_alpha(x+10*scale,y+10*scale,w.saturating_sub(20*scale),18*scale,5*scale,c[0].saturating_mul(3),c[1].saturating_mul(3),c[2].saturating_mul(3),255);
            }
            let color=if selected {(240,250,255)}else{(171,198,217)};
            if pointer && i<2 && w<115*scale {
                self.personalization_label(x+8*scale,y+h-36*scale,w.saturating_sub(16*scale),b"Classic",color.0,color.1,color.2);
                self.personalization_label(x+8*scale,y+h-18*scale,w.saturating_sub(16*scale),if i==0{b"White"}else{b"Black"},color.0,color.1,color.2);
            } else {
                self.personalization_label(x+8*scale,y+h-24*scale,w.saturating_sub(16*scale),if pointer {CURSOR_NAMES[i]}else{TINT_NAMES[i]},color.0,color.1,color.2);
            }
        }
        let last=controls.choices[if pointer {9}else{4}];
        let y=last.bottom()+12*scale as i32;
        if y>=0 {
            let status=crate::ui::personalization::save_status();
            let text:&[u8]=match status {1=>b"Release to save",2=>b"Saved for this user",3=>b"Save failed; changes restored",_=>b"Changes save automatically"};
            self.personalization_label(detail.x.max(0) as usize+16*scale,y as usize,detail.width.saturating_sub(32*scale as u32) as usize,text,if status==3 {240}else{135},if status==3 {140}else{172},if status==3 {125}else{194});
        }
    }
}
