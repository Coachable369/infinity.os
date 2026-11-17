use super::*;
#[cfg(any(target_arch="x86_64",target_arch="aarch64"))]
impl DisplayDevice {
    // ------------------------=
    // FUNC: weather_widget
    // DESC: Composites real provider data on the reusable glass illustration; stale readings remain labeled.
    // ------------------=
    pub(crate) fn weather_widget(&mut self,scale:usize,report:Option<(i16,u8,u64,bool)>,outlook:Option<crate::ui::weather_forecast::Outlook>,now:u64,city:&[u8],setup:bool,draft:&[u8],focused:bool,message:&[u8]){
        let r=crate::ui::desktop_widgets::current().rect(2,self.width,self.height,scale,false);
        let(x,y,w,h)=(r.x as usize,r.y as usize,r.width as usize,r.height as usize);
        self.widget_shadow(r,scale);
        let alpha=crate::ui::desktop_widgets::current().material_alpha(2,crate::ui::widget_keyframes::value(2));
        let day=report.is_some_and(|r|r.3);
        self.weather_sky(x,y,w,h,alpha,day,crate::ui::widget_keyframes::cloud_phase());
        if let (true,Some(out),Some((_,_,received,_)))=(day,outlook,report) {
            if out.sunset>out.sunrise {
                let epoch=out.observed+now.saturating_sub(received).min(3600) as i64;
                let t=((epoch-out.sunrise)*1000/(out.sunset-out.sunrise)).clamp(0,1000) as usize;
                let cx=x+w*(12+76*t/1000)/100;
                let cy=y+h*(48-28*4*t*(1000-t)/1_000_000)/100;
                let radius=(w/45).clamp(3,16);
                let halo=(radius*4)as i32;
                let edge=halo*halo;
                for dy in -halo..=halo {for dx in -halo..=halo {
                    let d=dx*dx+dy*dy;if d>=edge{continue}
                    let falloff=(edge-d)*255/edge;
                    let core=(1+radius as i32).pow(2);
                    let strength=if d<core{175+72*(core-d)/core}else{falloff*falloff*falloff/260_100};
                    self.blend_color(cx as i32+dx,cy as i32+dy,255,232,186,(strength*i32::from(alpha)/255)as u8);
                }}
            }
        }
        self.widget_grip(x,y,scale);
        let old=self.render_clip;self.intersect_render_clip(x,y,w,h);
        let left=x+14*scale;
        let label=FontSize::new(UI_FONT_CELL_HEIGHT,(h/8).clamp(15,30));
        self.intersect_render_clip(left,y,w*63/100,h/3);
        self.ui_text_raster(left,y+h/14,if setup{message}else{city},214,232,249,label,true,None);
        self.render_clip=old;self.intersect_render_clip(x,y,w,h);
        if setup{
            use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface}};
            let mut control=|offset:usize,top:usize,width:usize,height:usize,kind:Surface,label:&[u8],focus:bool|{
                let rect=Rect{x:(x+offset) as i32,y:(y+top)as i32,width:width as u32,height:height as u32};
                self.precision_surface(rect,kind,Material{fill:(10,29,52),edge:(105,173,209),opacity:220,edge_opacity:220,focus:if focus{255}else{0},pressed:false,scale});
                self.intersect_render_clip(x+offset+4,y+top,width.saturating_sub(8),height);
                let text_y=y+top+height.saturating_sub(28)/2;
                if kind==Surface::Field {
                    self.ui_text(x+offset+8,text_y,label,228,241,252,1);
                } else {
                    self.ui_text_centered(x+offset,width,text_y,label,228,241,252,1);
                }
                self.render_clip=old;self.intersect_render_clip(x,y,w,h);
            };
            control(w/20,h/3,w*9/10,h/4,Surface::Field,if draft.is_empty(){b"City name"}else{draft},focused);
            control(w/20,h*2/3,w*4/10,h/4,Surface::Primary,b"Save",false);
            control(w/2,h*2/3,w*45/100,h/4,Surface::Secondary,b"Detect",false);
            self.render_clip=old;return;
        }
        self.ui_text_raster(x+w*73/100,y+h/14,b"Change",170,200,225,label,false,None);
        if let Some((t,c,last,_))=report{
            let mut bytes=[0u8;20];let mut text=[0u8;12];let mut n=0;
            if t<0{text[0]=b'-';n+=1;}
            let number=super::editor_view::decimal(t.unsigned_abs() as usize,&mut bytes);
            text[n..n+number.len()].copy_from_slice(number);n+=number.len();text[n..n+2].copy_from_slice(b" F");
            self.ui_text_raster(left,y+h/4,&text[..n+2],239,248,255,FontSize::new(UI_FONT_CELL_HEIGHT,(h/4).clamp(23,56)),true,None);
            let condition=match c{0=>b"Clear".as_slice(),1..=3=>b"Clouds",45|48=>b"Fog",51..=57=>b"Drizzle",61..=67|80..=82=>b"Rain",71..=77|85|86=>b"Snow",95..=99=>b"Thunderstorms",_=>b"Weather"};
            let stale=now.saturating_sub(last)>1800;
            let mut status=[0u8;32];
            status[..condition.len()].copy_from_slice(condition);
            let mut length=condition.len();
            if stale {
                let suffix=b" / stale";
                status[length..length+suffix.len()].copy_from_slice(suffix);
                length+=suffix.len();
            }
            self.ui_text_raster(left,y+h/2,&status[..length],192,218,238,label,false,None);
            if let Some(out)=outlook.filter(|v|v.count>0) {
                let small=FontSize::new(UI_FONT_CELL_HEIGHT,(h/9).clamp(13,25));
                self.fill_rounded_rect_alpha(x+6*scale,y+h*67/100,w-12*scale,h*29/100,8*scale,9,17,38,(u16::from(alpha)*2/3)as u8);
                for (i,hour) in out.hours[..out.count].iter().skip(out.page*3).take(3).enumerate(){
                    let col=x+w*(12+27*i)/100;
                    let display=hour.hour%12;let mut hour_text=[b' ';5];
                    let display=if display==0{12}else{display};
                    hour_text[0]=if display>=10{b'1'}else{b' '};hour_text[1]=b'0'+display%10;
                    hour_text[3..5].copy_from_slice(if hour.hour<12{b"AM"}else{b"PM"});
                    self.ui_text_raster(col,y+h*68/100,&hour_text,169,196,222,small,false,None);
                    let mut b=[0u8;20];let digits=super::editor_view::decimal(hour.temperature.unsigned_abs() as usize,&mut b);
                    let mut value=[0;10];let sign=usize::from(hour.temperature<0);if sign!=0{value[0]=b'-';}
                    value[sign..sign+digits.len()].copy_from_slice(digits);let end=sign+digits.len();value[end..end+2].copy_from_slice(b" F");
                    self.ui_text_raster(col,y+h*81/100,&value[..end+2],236,246,255,small,true,None);
                }
                if out.page>0 {self.ui_text_raster(x+w/28,y+h*74/100,b"<",160,209,248,label,true,None);}
                if (out.page+1)*3<out.count {self.ui_text_raster(x+w*91/100,y+h*74/100,b">",160,209,248,label,true,None);}
            } else {self.ui_text_raster(left,y+h*8/10,b"Open-Meteo",158,186,207,label,false,None);}
        }else{self.ui_text(left,y+h/2,b"Awaiting forecast",191,212,232,1);}
        self.render_clip=old;
    }

    // ------------------------=
    // FUNC: weather_sky
    // DESC: Interpolates four aligned cloud keyframes inside the rounded alpha mask, including subdued movement at night.
    // ------------------=
    fn weather_sky(&mut self,x:usize,y:usize,w:usize,h:usize,opacity:u8,day:bool,phase:u8){
        const FRAMES:[&[u8];4]=[include_bytes!("../../../assets/ui-design-kit/default/weather-day.bmp"),
            include_bytes!("../../../assets/ui-design-kit/default/weather-cloud-1.bmp"), include_bytes!("../../../assets/ui-design-kit/default/weather-cloud-2.bmp"),
            include_bytes!("../../../assets/ui-design-kit/default/weather-cloud-3.bmp")];
        const NIGHT:&[u8]=include_bytes!("../../../assets/ui-design-kit/default/weather-glass.bmp");
        let Some(clip)=self.clipped_render_region(x,y,w,h) else{return};
        let index=usize::from(phase/32);let mix=u16::from(phase%32);
        let (first,next)=(FRAMES[index],FRAMES[(index+1)%4]);
        for py in clip.top..clip.bottom {
            let row=(255-(py-y)*256/h)*512;
            for px in clip.left..clip.right {
                let at=54+4*(row+(px-x)*512/w);
                let channel=|c:usize|{let cloud=(u16::from(first[at+c])*(32-mix)+u16::from(next[at+c])*mix)/32;
                    if day{cloud as u8}else{((u16::from(NIGHT[at+c])*7+cloud)/8)as u8}};
                let alpha=(u16::from(first[at+3])*u16::from(opacity)/255)as u8;
                if alpha>0{self.blend_color_unchecked(px,py,channel(2),channel(1),channel(0),alpha);}
            }
        }
        self.mark_dirty_rect(clip.left,clip.top,clip.right-clip.left,clip.bottom-clip.top);
    }
}
