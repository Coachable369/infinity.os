//! Voice enrollment uses the authored configuration template and shared Access controls.
use super::DisplayDevice;
use crate::ui::voice_setup_layout::{Stage,Failure};
static mut WAVE_BASE:[u32;1024*256]=[0;1024*256];
static mut WAVE_RECT:Option<crate::ui::installer_layout::InstallerRect>=None;
static mut WAVE_TIME:u64=0;
impl DisplayDevice{
 // ------------------------=
 // FUNC: voice_setup_frame
 // DESC: Presents only the native identity workflow state; desktop assistant audio is not involved.
 // ------------------=
 #[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
 pub(super) fn voice_setup_frame(&mut self,focus:usize){
  let(stage,count,level)=crate::runtime::voice_identity_host::snapshot();
  self.voice_template_frame(crate::ui::installer_layout::CONFIGURATION_TEMPLATE_BYTES,focus,stage,count,level);
 }
 // ------------------------=
 // FUNC: voice_template_frame
 // DESC: Draws the shared Studio voice screen with real sample progress, explicit retry states and live amplitude feedback.
 // ------------------=
 pub(super) fn voice_template_frame(&mut self,bytes:&[u8],focus:usize,stage:Stage,count:usize,level:u16)->bool{
  use crate::ui::{installer_template::InstallerTemplate,installer_layout::scale_template_rect};
  let Ok(t)=InstallerTemplate::parse(bytes)else{return false;};let Some(n)=t.element_count(10)else{return false;};
  let message:&[u8]=match stage{
   Stage::Capturing=>b"Speak the phrase clearly. Recording starts when your voice is heard.",
   Stage::Comparing=>b"Comparing your voice locally. Please wait.",
   Stage::Saved=>b"Voice identity saved. Continue to finish setup.",
   Stage::Failed(Failure::Mismatch)=>b"This sample did not match. Speak naturally and record it again.",
   Stage::Failed(Failure::Quiet)=>b"The recording was too quiet. Move closer to the microphone and retry.",
   Stage::Failed(Failure::Timeout)=>b"Recording timed out. Speak after pressing Record sample.",
   Stage::Failed(Failure::Clipped)=>b"The recording is distorted. Lower microphone gain or move farther away.",
   Stage::Failed(Failure::AnalysisTimeout)=>b"Voice analysis timed out. Retry this sample or skip voice sign-in.",
   Stage::Failed(Failure::Worker)=>b"Voice analysis failed. Retry this sample or skip voice sign-in.",
   Stage::Failed(Failure::Storage)=>b"Your voice could not be saved. Retry or use password sign-in.",
   Stage::Failed(_)=>b"Voice capture unavailable. Check the microphone or skip for now.",
   _=>b"Speak clearly and naturally in a quiet place.",
  };
  for index in 0..n{
   let Some(mut e)=t.layer_at(10,index)else{continue;};if e.hidden||e.opacity==0{continue;}
   let r=scale_template_rect(e.frame,self.width,self.height);let id=e.code_identifier;
   match id{
    b"voiceGuidance"=>e.text=message,
    b"voiceStepTitle"=>e.text=match stage{Stage::Capturing=>b"Listening to you",Stage::Comparing=>b"Verifying voice",Stage::Saved=>b"Identity saved",Stage::Failed(_)=>b"Please try again",_=>b"Read the phrase"},
    b"voiceRecordStatus"=>e.text=match stage{Stage::Capturing=>b"Recording",Stage::Comparing=>b"Comparing",Stage::Saved=>b"Saved",Stage::Failed(_)=>b"Ready to retry",_=>b"Ready to record"},
    b"voicePrimary"=>e.text=match stage{Stage::Capturing=>b"Recording...",Stage::Comparing=>b"Comparing...",Stage::Saved=>b"Continue",Stage::Failed(_)=>b"Retry sample",_=>b"Record sample"},
    b"voiceSampleStatus0"|b"voiceSampleStatus1"|b"voiceSampleStatus2"=>{
     let slot=(id[id.len()-1]-b'0')as usize;e.text=if stage==Stage::Saved||slot<count{b"Accepted"}else if slot==count&&matches!(stage,Stage::Capturing|Stage::Comparing){b"Checking..."}else{b"Pending"};
    },
    b"voicePhotoCard0"|b"voicePhotoCard1"|b"voicePhotoCard2"=>{
     let slot=(id[id.len()-1]-b'0')as usize;
     if stage==Stage::Saved||slot<count{e.border=[90,205,180,255];}else if slot==count{e.border=[40,225,255,255];}
    },
    b"voiceWave"|b"voiceSampleWave0"|b"voiceSampleWave1"|b"voiceSampleWave2"=>{
     let active=id==b"voiceWave";
     if active && r.width<=1024 && r.height<=256 && r.left+r.width<=self.width && r.top+r.height<=self.height {unsafe {
      let base=&mut *core::ptr::addr_of_mut!(WAVE_BASE);
      for y in 0..r.height {core::ptr::copy_nonoverlapping(self.buffer.add((r.top+y)*self.stride+r.left),base.as_mut_ptr().add(y*1024),r.width);}
      WAVE_RECT=Some(r);
     }}let amplitude=if active&&stage==Stage::Capturing{(level as usize*100/4000).min(100)}else{28};
     let bars:usize=if active{39}else{17};
     for b in 0..bars{
      let distance=(b as i32-bars as i32/2).unsigned_abs()as usize;
      let envelope=(bars/2+1).saturating_sub(distance);
      let h=(r.height*amplitude*envelope/(100*(bars/2+1))).max(2);
      let x=r.left+(b*2+1)*r.width/(bars*2);let y=r.top+(r.height-h)/2;
      self.fill_rect(x,y,(r.width/(bars*4)).max(2),h,90,210,240);
     }
     continue;
    },_=>{},
   }
   if matches!(e.kind,1|4|5){let target=match id{b"voiceBack"=>Some(0),b"voicePrimary"=>Some(1),b"voiceSkip"=>Some(2),_=>None};
    self.access_template_control_with_focus(e,None,(0,0),Some(target==Some(focus)));
   }else{self.template_element_in_rect(e,t.asset(e.image_asset),None,r);}
  }
  true
 }
}

// ------------------------=
// FUNC: animate
// DESC: Updates only the live microphone waveform at 30 Hz, preserving cursor pixels and the static authored scene.
// ------------------=
#[cfg(all(feature="native-face",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
pub fn animate(visible:bool)->bool{unsafe{
 if !visible{WAVE_RECT=None;return false;}
 let clock=crate::ui::performance::monotonic_ns().unwrap_or(0)/1_000_000;
 if clock.saturating_sub(WAVE_TIME)<33{return false;}
 let Some(r)=WAVE_RECT else{return false;};
 let Some(c)=(*core::ptr::addr_of_mut!(super::CONSOLE)).as_mut()else{return false;};
 let(stage,_,level)=crate::runtime::voice_identity_host::snapshot();
 if !matches!(stage,Stage::Capturing|Stage::Comparing){return false;}
 if r.left+r.width>c.display.width||r.top+r.height>c.display.height{return false;}
 c.restore_cursor();let d=&mut c.display;let base=&*core::ptr::addr_of!(WAVE_BASE);
 for y in 0..r.height{core::ptr::copy_nonoverlapping(base.as_ptr().add(y*1024),d.buffer.add((r.top+y)*d.stride+r.left),r.width);}
 let motion=if crate::console::reduced_motion(){0}else{clock};
 for b in 0usize..39{
 let h=crate::ui::voice_setup_layout::wave_height(r.height,b,level,stage==Stage::Comparing,motion);
 d.fill_rect(r.left+(b*2+1)*r.width/78,r.top+(r.height-h)/2,(r.width/156).max(2),h,90,210,240);}
 d.mark_dirty_rect(r.left,r.top,r.width,r.height);WAVE_TIME=clock;
 c.save_and_draw_cursor(c.cursor_x,c.cursor_y);c.display.present_damage();true
}}
