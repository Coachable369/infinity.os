//! Native browser preferences and shared settings-page hit geometry.
use crate::Viewport;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Settings {pub search:u8,pub favorites:bool}
impl Settings {
    // ------------------------=
    // FUNC: new
    // DESC: Returns the browser's established defaults.
    // ------------------=
    pub const fn new()->Self {Self{search:0,favorites:true}}
    // ------------------------=
    // FUNC: bytes
    // DESC: Encodes a versioned bounded preference record.
    // ------------------=
    pub fn bytes(self)->[u8;6] {[b'I',b'B',b'S',1,self.search,self.favorites as u8]}
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt and future records rather than silently replacing them.
    // ------------------=
    pub fn decode(b:&[u8])->Option<Self> {
        if b.len()!=6 || b[..4]!=[b'I',b'B',b'S',1] || b[4]>2 || b[5]>1 {return None;}
        Some(Self{search:b[4],favorites:b[5]!=0})
    }
    // ------------------------=
    // FUNC: search_prefix
    // DESC: Supplies the selected provider to real address-bar navigation.
    // ------------------=
    pub fn search_prefix(self)->&'static str {match self.search {
        1=>"https://duckduckgo.com/?q=",2=>"https://www.bing.com/search?q=",_=>"https://www.google.com/search?q=",
    }}
}
// ------------------------=
// FUNC: control
// DESC: Shares settings hit targets with the native painter at every supported scale.
// ------------------=
pub fn control(content:Viewport,scale:u32,index:usize)->Viewport {
    let (x,y,w)=match index {0=>(24,88,144),1=>(176,88,144),2=>(328,88,144),
        3=>(328,144,144),4=>(328,200,144),5=>(24,264,168),_=>(328,264,144)};
    Viewport{x:content.x+(x*scale) as i32,y:content.y+(y*scale) as i32,width:w*scale,height:32*scale}
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: preferences_roundtrip_and_navigation
    // DESC: Exercises every supported preference and real omnibox routing, rejecting invalid records.
    // ------------------=
    #[test]
    fn preferences_roundtrip_and_navigation() {
        for search in 0..3 {for favorites in [false,true] {
            let s=Settings{search,favorites};assert_eq!(Settings::decode(&s.bytes()),Some(s));
            let mut out=[0;2048];let (_,n)=crate::omnibox::resolve("hello world",s.search_prefix(),&mut out).unwrap();
            assert!(n>s.search_prefix().len());assert_eq!(&out[..s.search_prefix().len()],s.search_prefix().as_bytes());
        }}
        assert_eq!(Settings::decode(&[b'I',b'B',b'S',1,3,1]),None);
        assert_eq!(Settings::decode(&[b'I',b'B',b'S',1,0,2]),None);
        assert_eq!(Settings::decode(&[]),None);
    }
    // ------------------------=
    // FUNC: visibility_reclaims_content_without_moving_navigation
    // DESC: Exercises actual layout and hit testing at every supported UI scale.
    // ------------------=
    #[test]
    fn visibility_reclaims_content_without_moving_navigation() {
        for scale in 1..=4 {
            let shown=crate::layout::Layout::new(900*scale,600*scale,scale).unwrap();
            let hidden=crate::layout::Layout::new(900*scale,600*scale,scale).unwrap().with_favorites(false);
            assert_eq!(shown.menu,hidden.menu);
            assert_eq!(hidden.content.height,shown.content.height+36*scale);
            assert_eq!(hidden.hit(30*scale as i32,105*scale as i32),Some(crate::layout::Control::Content));
            for index in 0..7 {
                let r=control(shown.content,scale,index);
                assert!(shown.content.local(r.x,r.y).is_some());
                assert!(shown.content.local(r.x+r.width as i32-1,r.y+r.height as i32-1).is_some());
                for other in 0..7 {if index!=other {assert!(control(shown.content,scale,other).local(r.x,r.y).is_none());}}
            }
        }
    }
}
