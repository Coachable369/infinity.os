//! Shared authored voice screen hit geometry, used by the installed UI and Studio.
use super::geometry::{Rect,Point};
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Stage{Idle,Ready,Capturing,Comparing,Saved,Verified,Failed(Failure)}
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Failure{Unavailable,Unauthorized,NoEnrollment,Quiet,Clipped,Timeout,AnalysisTimeout,Mismatch,Storage,Worker}
#[derive(Clone,Copy)]
pub struct Layout{pub back:Rect,pub primary:Rect,pub skip:Rect}
// ------------------------=
// FUNC: binding
// DESC: Resolves only visible voice-screen elements using the same authored bounds as the painter.
// ------------------=
pub fn binding(id:&[u8],width:usize,height:usize)->Option<Rect>{
 let t=super::installer_template::InstallerTemplate::parse(super::installer_layout::CONFIGURATION_TEMPLATE_BYTES).ok()?;
 for index in 0..t.element_count(10)?{let e=t.element_at(10,index)?;if e.code_identifier==id&&!e.hidden&&e.opacity>0{
 let r=super::installer_layout::scale_template_rect(e.frame,width,height);return Some(Rect{x:r.left as i32,y:r.top as i32,width:r.width as u32,height:r.height as u32});}}
 None
}
// Accessed only by the serialized native console/compositor thread.
#[cfg(target_os="none")]
static mut LAYOUT_CACHE:Option<(usize,usize,Layout)>=None;
impl Layout{
 // ------------------------=
 // FUNC: new
 // DESC: Shares production-authored button rectangles without scaled hit-test guesses.
 // ------------------=
 pub fn new(w:usize,h:usize)->Self{
        #[cfg(target_os="none")]
        unsafe {if let Some((cached_w,cached_h,layout))=*core::ptr::addr_of!(LAYOUT_CACHE) {if (cached_w,cached_h)==(w,h) {return layout;}}}
        let layout=Self::resolve(w,h);
        #[cfg(target_os="none")]
        unsafe {*core::ptr::addr_of_mut!(LAYOUT_CACHE)=Some((w,h,layout));}
        layout
    }
    // ------------------------=
    // FUNC: resolve
    // DESC: Resolves immutable authored geometry once per viewport size.
    // ------------------=
    fn resolve(w:usize,h:usize)->Self{let get=|id:&[u8]|binding(id,w,h).unwrap_or(Rect{x:0,y:0,width:0,height:0});Self{back:get(b"voiceBack"),primary:get(b"voicePrimary"),skip:get(b"voiceSkip")}}
 // ------------------------=
 // FUNC: hit
 // DESC: Maps visible voice enrollment buttons to keyboard-equivalent actions.
 // ------------------=
 pub fn hit(&self,p:Point)->Option<usize>{[(self.back,0),(self.primary,1),(self.skip,2)].into_iter().find_map(|(r,id)|r.contains(p).then_some(id))}
}

// ------------------------=
// FUNC: wave_height
// DESC: Combines measured amplitude with a restrained live activity pulse so silent capture and comparison remain visibly active.
// ------------------=
pub fn wave_height(height:usize,bar:usize,level:u16,comparing:bool,millis:u64)->usize {
 if height==0{return 0;}
 let pulse=((millis/12)%100)as usize;
 let pulse=if pulse<50{pulse}else{100-pulse};
 let amplitude=if comparing{20+pulse}else{(level as usize*100/4000).min(100).max(12+pulse/3)};
 let envelope=20usize.saturating_sub((bar as i32-19).unsigned_abs()as usize);
 let phase=(bar*13+(millis/20)as usize)%40;
 let motion=20+if phase<20{phase}else{40-phase};
 (height*amplitude*envelope*motion/80000).max(2).min(height)
}
