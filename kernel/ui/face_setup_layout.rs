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
        let unit=|value:usize|(value*height/1000).max(1);
        let left=width*8/100;let top=height*16/100;
        let w=width*84/100;let h=height*70/100;let gutter=unit(32);
        let column=w.saturating_sub(gutter*3)/2;
        let preview_w=column.min(unit(208));let preview_h=preview_w*3/4;
        let mask_size=column.min(unit(420)).min(480);
        let rect=|x:usize,y:usize,w:usize,h:usize|Rect{x:x as i32,y:y as i32,width:w as u32,height:h as u32};
        let bottom=top+h-gutter-unit(56);let button_w=(w-gutter*4)/3;
        Self {card:rect(left,top,w,h),title:rect(left+gutter,top+gutter,w-gutter*2,unit(64)),
            guidance:rect(left+gutter,top+unit(110),column,unit(140)),
            field:rect(left+gutter,top+unit(255),column,unit(56)),
            preview:rect(left+gutter,top+unit(345),preview_w,preview_h),
            mask:rect(left+w-gutter-column+(column-mask_size)/2,top+unit(100),mask_size,mask_size),
            status:rect(left+gutter,top+h-unit(190),w-gutter*2,unit(80)),
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
