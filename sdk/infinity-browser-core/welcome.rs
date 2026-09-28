//! Local start-page geometry shared by native painting and input.
use crate::Viewport;

pub const HERO: &[u8] = include_bytes!("../../assets/ui-design-kit/default/browser-welcome-hero-v1.bmp");
pub struct Layout { pub area: Viewport, pub scale: u32, pub narrow: bool }
impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Centers a bounded responsive start page in the content viewport.
    // ------------------=
    pub fn new(content:Viewport,scale:u32,scroll:u32)->Self {
        let scale=scale.max(1);let width=content.width.min(1200*scale);
        Self{area:Viewport{x:content.x+((content.width-width)/2) as i32,y:content.y-scroll as i32,width,..content},
            scale,narrow:width/scale<720}
    }
    // ------------------------=
    // FUNC: height
    // DESC: Gives the full scrollable page extent in physical pixels.
    // ------------------=
    pub fn height(&self)->u32 {if self.narrow {660*self.scale}else{380*self.scale}}
    // ------------------------=
    // FUNC: rect
    // DESC: Maps authored logical coordinates into the shared content plane.
    // ------------------=
    pub fn rect(&self,x:u32,y:u32,w:u32,h:u32)->Viewport {
        Viewport{x:self.area.x+(x*self.scale) as i32,y:self.area.y+(y*self.scale) as i32,width:w*self.scale,height:h*self.scale}
    }
    // ------------------------=
    // FUNC: button
    // DESC: Returns two distinct keyboard-accessible native action targets.
    // ------------------=
    pub fn button(&self,index:usize)->Viewport {self.rect(32+index as u32*180,166,164,36)}
    // ------------------------=
    // FUNC: hero
    // DESC: Keeps the complete transparent hero separate from text and controls.
    // ------------------=
    pub fn hero(&self)->Viewport {
        let w=self.area.width/self.scale;
        if self.narrow {self.rect(32,214,w-64,166)}else{let x=(w/2+12).max(400);self.rect(x,16,w-x-32,192)}
    }
    // ------------------------=
    // FUNC: card
    // DESC: Reflows three informational cards without overlapping their gutters.
    // ------------------=
    pub fn card(&self,index:usize)->Viewport {
        let w=self.area.width/self.scale;
        if self.narrow {self.rect(32,392+index as u32*70,w-64,60)}
        else {let width=(w-96)/3;self.rect(32+index as u32*(width+16),224,width,100)}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: responsive_actions_and_artwork_never_overlap
    // DESC: Exercises real hit geometry across supported window sizes, scales and scroll offsets.
    // ------------------=
    #[test]
    fn responsive_actions_and_artwork_never_overlap() {
        for scale in 1..=4 {for width in [520,719,720,818,1200,1900] {
            let content=Viewport{x:17,y:31,width:width*scale,height:379*scale};
            for scroll in [0,100*scale] {let l=Layout::new(content,scale,scroll);
                let mut all=[l.button(0),l.button(1),l.hero(),l.card(0),l.card(1),l.card(2)];
                for (i,a) in all.iter().enumerate() {assert!(a.x>=content.x);assert!(a.x+a.width as i32<=content.x+content.width as i32);
                    assert!(a.y+a.height as i32<=content.y+l.height() as i32-scroll as i32);
                    for b in &all[i+1..] {assert!(a.x+a.width as i32<=b.x || b.x+b.width as i32<=a.x || a.y+a.height as i32<=b.y || b.y+b.height as i32<=a.y);}}
                all[0].x+=1;assert!(l.button(0).local(all[0].x,all[0].y).is_some());
            }
        }}
        assert_eq!(&HERO[..2],b"BM");assert_eq!(u16::from_le_bytes([HERO[28],HERO[29]]),32);
    }
}
