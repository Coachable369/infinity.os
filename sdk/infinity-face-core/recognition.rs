//! Real model pipeline over decoded native RGB frames. Execute in the isolated
//! camera worker, never in a compositor callback or kernel service dispatch.
use crate::{inference::{Execution,Model}, vision::{self,Face,Head}, Binding, Observation, Pose, DIMENSIONS};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Model, Memory, Cancelled, Inference(crate::inference::Error), Image(vision::Error), Quality(crate::Error) }

/// Caller-owned private scratch, reused between frames. Do not place on a kernel
/// stack or expose its address through IOP. Approximately 132 KiB.
#[repr(C)]
pub struct Scratch {class:[f32;2100],object:[f32;2100],boxes:[f32;8400],landmarks:[f32;21000]}
pub struct Recognition {pub face:Face,pub pose:Pose,embedding:[f32;DIMENSIONS],grey:[u8;112*112]}
pub struct Recognizer<'a> {detector:Model<'a>,recognizer:Model<'a>}
impl<'a> Recognizer<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Parses fixed detector/recognizer packages once and rejects swapped or unsupported model roles.
    // ------------------=
    pub fn new(detector:&'a [u8],recognizer:&'a [u8])->Result<Self,Error> {
        let detector=Model::parse(detector).map_err(Error::Inference)?;
        let recognizer=Model::parse(recognizer).map_err(Error::Inference)?;
        if detector.kind()!=1 || recognizer.kind()!=2 {return Err(Error::Model);}
        Ok(Self {detector,recognizer})
    }
    // ------------------------=
    // FUNC: arena_len
    // DESC: Reports the exact reusable activation budget; the two graphs are never resident in scratch simultaneously.
    // ------------------=
    pub fn arena_len(&self)->usize {self.detector.arena_len.max(self.recognizer.arena_len)}
    // ------------------------=
    // FUNC: recognize
    // DESC: Runs YuNet detection, five-point alignment and SFace embedding, yielding cooperatively between bounded compute visits.
    // ------------------=
    pub fn recognize(&self,rgb:&[u8],arena:&mut [f32],scratch:&mut Scratch,mut visit:impl FnMut()->bool)->Result<Recognition,Error> {
        if arena.len()<self.arena_len() {return Err(Error::Memory);}
        let mut execution=Execution::new(self.detector,&mut arena[..self.detector.arena_len]).map_err(Error::Inference)?;
        vision::detector_input(rgb,execution.input().map_err(Error::Inference)?).map_err(Error::Image)?;
        while !execution.step(128).map_err(Error::Inference)? {if !visit() {return Err(Error::Cancelled);}}
        let ranges=[(0,1600),(1600,2000),(2000,2100)];
        for (level,&(a,b)) in ranges.iter().enumerate() {
            execution.output(level,&mut scratch.class[a..b]).map_err(Error::Inference)?;
            execution.output(level+3,&mut scratch.object[a..b]).map_err(Error::Inference)?;
            execution.output(level+6,&mut scratch.boxes[a*4..b*4]).map_err(Error::Inference)?;
            execution.output(level+9,&mut scratch.landmarks[a*10..b*10]).map_err(Error::Inference)?;
        }
        drop(execution);
        if !visit() {return Err(Error::Cancelled);}
        let head=|level:usize| {let (a,b)=ranges[level];Head {stride:8<<level,class:&scratch.class[a..b],
            object:&scratch.object[a..b],boxes:&scratch.boxes[a*4..b*4],landmarks:&scratch.landmarks[a*10..b*10]}};
        let face=vision::single_face([head(0),head(1),head(2)]).map_err(Error::Image)?;
        let pose=vision::pose(&face).map_err(Error::Image)?;
        let mut result=Recognition {face,pose,embedding:[0.0;DIMENSIONS],grey:[0;112*112]};
        let mut execution=Execution::new(self.recognizer,&mut arena[..self.recognizer.arena_len]).map_err(Error::Inference)?;
        vision::align(rgb,&face,execution.input().map_err(Error::Inference)?,&mut result.grey).map_err(Error::Image)?;
        crate::quality(&result.grey,100).map_err(Error::Quality)?;
        while !execution.step(128).map_err(Error::Inference)? {if !visit() {return Err(Error::Cancelled);}}
        if !visit() {return Err(Error::Cancelled);}
        execution.output(0,&mut result.embedding).map_err(Error::Inference)?;
        crate::norm(&result.embedding).map_err(Error::Quality)?;
        Ok(result)
    }
}
impl Recognition {
    // ------------------------=
    // FUNC: observation
    // DESC: Supplies actual pipeline outputs to the enrollment state machine with the transport's capture binding and timestamp.
    // ------------------=
    pub fn observation(&self,binding:Binding,sequence:u64,captured_ms:u64)->Observation<'_> {
        Observation {binding,sequence,captured_ms,face_count:1,face_width:self.face.width as u16,
            face_height:self.face.height as u16,pose:self.pose,crop:&self.grey,embedding:&self.embedding}
    }
}
impl Drop for Recognition {
    // ------------------------=
    // FUNC: drop
    // DESC: Wipes temporary aligned face pixels and embedding after the enrollment consumer has used them.
    // ------------------=
    fn drop(&mut self) {
        for v in &mut self.embedding {unsafe {core::ptr::write_volatile(v,0.0);}}
        for v in &mut self.grey {unsafe {core::ptr::write_volatile(v,0);}}
    }
}
impl Scratch {
    // ------------------------=
    // FUNC: clear
    // DESC: Erases face location/confidence scratch when a worker session ends.
    // ------------------=
    pub fn clear(&mut self) {
        for v in self.class.iter_mut().chain(&mut self.object).chain(&mut self.boxes).chain(&mut self.landmarks) {
            unsafe {core::ptr::write_volatile(v,0.0);}
        }
    }
}
