use super::*;
#[cfg(any(target_arch="x86_64",target_arch="aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: metric_ring
    // DESC: Draws a bounded annular metric against the UIKit track, with a fixed twelve-o'clock origin.
    // ------------------=
    pub(crate) fn metric_ring(&mut self,x:usize,y:usize,r:usize,value:u8,color:(u8,u8,u8)){
        self.paint_bitmap_alpha_fit_rect(include_bytes!("../../../assets/ui-design-kit/default/telemetry-ring.bmp"),x.saturating_sub(r+2),y.saturating_sub(r+2),2*r+4,2*r+4);
        const ARC:[(i16,i16);51]=ARC_POINTS;
        for (i,pair) in ARC.windows(2).enumerate(){
            let point=|(dx,dy):(i16,i16)|(x as i32+dx as i32*r as i32/1024,y as i32+dy as i32*r as i32/1024);
            let (a,b)=point(pair[0]);let(c,d)=point(pair[1]);
            self.icon_line(a,b,c,d,if i*2<(value as usize){color}else{(39,63,86)},r*2);
        }
    }
    // ------------------------=
    // FUNC: weather_widget
    // DESC: Composites real provider data on the reusable glass illustration; stale readings remain labeled.
    // ------------------=
    pub(crate) fn weather_widget(&mut self,scale:usize,report:Option<(i16,u8,u64)>,now:u64,city:&[u8],setup:bool,draft:&[u8],focused:bool,message:&[u8]){
        let r=crate::ui::desktop_widgets::current().rect(2,self.width,self.height,scale,false);
        let(x,y,w,h)=(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.paint_bitmap_alpha_fit_rect_opacity(include_bytes!("../../../assets/ui-design-kit/default/weather-glass.bmp"),x,y,w,h,crate::ui::widget_keyframes::value(2));
        let old=self.render_clip;self.intersect_render_clip(x,y,w,h);
        let left=x+14*scale; self.ui_text_strong(left,y+h/10,if setup{message}else{city},214,232,249,1);
        if setup{
            use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
            let mut control=|offset:usize,top:usize,width:usize,height:usize,kind:Surface,label:&[u8],focus:bool|{
                let rect=Rect{x:(x+offset) as i32,y:(y+top)as i32,width:width as u32,height:height as u32};
                self.precision_surface(rect,kind,Material{fill:(10,29,52),edge:(105,173,209),opacity:220,edge_opacity:220,focus:if focus{255}else{0},pressed:false,scale});
                self.intersect_render_clip(x+offset+4,y+top,width.saturating_sub(8),height);
                self.ui_text(x+offset+8,y+top+height.saturating_sub(15)/2,label,228,241,252,1);
                self.render_clip=old;self.intersect_render_clip(x,y,w,h);
            };
            control(w/20,h/3,w*9/10,h/4,Surface::Field,if draft.is_empty(){b"City name"}else{draft},focused);
            control(w/20,h*2/3,w*4/10,h/4,Surface::Secondary,b"Find city",false);
            control(w/2,h*2/3,w*45/100,h/4,Surface::Secondary,b"Detect",false);
            self.render_clip=old;return;
        }
        self.ui_text(x+w*72/100,y+h/10,b"Change",164,211,238,1);
        if let Some((t,c,last))=report{
            let mut bytes=[0u8;20];let mut text=[0u8;12];let mut n=0;
            if t<0{text[0]=b'-';n+=1;}
            let number=super::editor_view::decimal(t.unsigned_abs() as usize,&mut bytes);
            text[n..n+number.len()].copy_from_slice(number);n+=number.len();text[n..n+2].copy_from_slice(b" F");
            self.ui_text_strong(left,y+h/3,&text[..n+2],239,248,255,2);
            let condition=match c{0=>b"Clear".as_slice(),1..=3=>b"Clouds",45|48=>b"Fog",51..=57=>b"Drizzle",61..=67|80..=82=>b"Rain",71..=77|85|86=>b"Snow",95..=99=>b"Thunderstorms",_=>b"Weather"};
            self.ui_text(left,y+h*6/10,condition,192,218,238,1);
            let stale=now.saturating_sub(last)>1800;
            self.ui_text(left,y+h*8/10,if stale{b"Open-Meteo / stale"}else{b"Open-Meteo"},158,186,207,1);
        }else{self.ui_text(left,y+h/2,b"Awaiting forecast",191,212,232,1);}
        self.render_clip=old;
    }
}
const ARC_POINTS:[(i16,i16);51]=[(0,-1024),(128,-1016),(255,-992),(377,-952),(493,-897),(602,-828),(701,-746),(789,-653),(865,-549),(927,-436),(974,-316),(1006,-192),(1022,-64),(1022,64),(1006,192),(974,316),(927,436),(865,549),(789,653),(701,746),(602,828),(493,897),(377,952),(255,992),(128,1016),(0,1024),(-128,1016),(-255,992),(-377,952),(-493,897),(-602,828),(-701,746),(-789,653),(-865,549),(-927,436),(-974,316),(-1006,192),(-1022,64),(-1022,-64),(-1006,-192),(-974,-316),(-927,-436),(-865,-549),(-789,-653),(-701,-746),(-602,-828),(-493,-897),(-377,-952),(-255,-992),(-128,-1016),(0,-1024)];
