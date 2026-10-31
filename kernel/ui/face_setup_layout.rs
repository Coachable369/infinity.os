//! Shared Eclipse Access setup geometry for painting and hit testing.
use super::geometry::{Rect,Point};
pub struct Layout {pub card:Rect,pub title:Rect,pub guidance:Rect,pub field:Rect,pub preview:Rect,pub mask:Rect,
    pub status:Rect,pub back:Rect,pub skip:Rect,pub primary:Rect,pub pixels:usize}
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Keeps 4:3 camera pixels and generous shared gutters within the existing configuration content frame.
    // ------------------=
    pub fn new(width:usize,height:usize)->Self {
        let mut layout=Self::fallback(width,height);
        if let Ok(template)=super::installer_template::InstallerTemplate::parse(super::installer_layout::CONFIGURATION_TEMPLATE_BYTES) {
            if template.element_count(9).is_some() {
                for (id,target) in [(b"facePanel" as &[u8],&mut layout.card),(b"faceTitle",&mut layout.title),
                    (b"faceGuidance",&mut layout.guidance),(b"faceName",&mut layout.field),
                    (b"facePreview",&mut layout.preview),(b"faceMesh",&mut layout.mask),
                    (b"faceStatus",&mut layout.status),(b"faceBack",&mut layout.back),
                    (b"faceSkip",&mut layout.skip),(b"facePrimary",&mut layout.primary)] {
                    *target=binding(&template,id,width,height).unwrap_or(Rect{x:0,y:0,width:0,height:0});
                }
            }
        }
        layout
    }
    // ------------------------=
    // FUNC: fallback
    // DESC: Retains layout compatibility for configuration documents predating the authored facial screen.
    // ------------------=
    fn fallback(width:usize,height:usize)->Self {
        let unit=|value:usize|(value*height/1000).max(1);
        let left=width*8/100;let top=height*16/100;
        let w=width*84/100;let h=height*70/100;let gutter=unit(32);
        let column=w.saturating_sub(gutter*3)/2;
        let mask_size=column.min(unit(420)).min(480)*170/100;
        let column=column.min(w.saturating_sub(gutter*3+mask_size));
        let preview_w=(column/3).min(unit(208));let preview_h=preview_w*3/4;
        let rect=|x:usize,y:usize,w:usize,h:usize|Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32};
        let bottom=top+h-gutter-unit(56);let button_w=(w-gutter*4)/3;
        Self {card:rect(left,top,w,h),title:rect(left+gutter,top+gutter,w-gutter*2,unit(64)),
            guidance:rect(left+gutter,top+unit(110),column,unit(140)),
            field:rect(left+gutter,top+unit(255),column,unit(56)),
            preview:rect(left+gutter,top+unit(345),preview_w,preview_h),
            mask:rect(left+w-gutter-mask_size,bottom.saturating_sub(gutter+mask_size),mask_size,mask_size),
            status:rect(left+gutter,top+h-unit(190),column,unit(80)),
            back:rect(left+gutter,bottom,button_w,unit(56)),
            skip:rect(left+gutter*2+button_w,bottom,button_w,unit(56)),
            primary:rect(left+gutter*3+button_w*2,bottom,button_w,unit(56)),pixels:unit(22).max(12)}
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Resolves pointer controls from the same rectangles used to render them.
    // ------------------=
    pub fn hit(&self,point:Point)->Option<usize> {
        [(self.back,0),(self.primary,1),(self.field,2),(self.skip,3)].into_iter()
            .find_map(|(rect,id)|rect.contains(point).then_some(id))
    }
}

// ------------------------=
// FUNC: binding
// DESC: Resolves visible authored face bounds for painting, camera damage, animation and pointer routing.
// ------------------=
pub fn binding(template:&super::installer_template::InstallerTemplate<'_>,id:&[u8],width:usize,height:usize)->Option<Rect> {
    for index in 0..template.element_count(9)? {
        let e=template.element_at(9,index)?;
        if e.code_identifier==id && !e.hidden && e.opacity>0 {
            let r=super::installer_layout::scale_template_rect(e.frame,width,height);
            return Some(Rect{x:r.left as i32,y:r.top as i32,width:r.width as u32,height:r.height as u32});
        }
    }
    None
}
