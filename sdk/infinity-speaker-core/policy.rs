//! Bounded speaker enrollment and comparison policy. Scores are experimental,
//! not a published false-accept rate or replay/liveness guarantee.
use crate::{DIM,Error,normalize,similarity};
pub const COUNT:usize=3;
pub const BYTES:usize=COUNT*DIM*4;
pub struct Enrollment {samples:[[f32;DIM];COUNT],count:usize}
impl Enrollment {
    // ------------------------=
    // FUNC: new
    // DESC: Creates empty private enrollment state without accepting unverified vectors.
    // ------------------=
    pub const fn new()->Self{Self{samples:[[0.0;DIM];COUNT],count:0}}
    // ------------------------=
    // FUNC: count
    // DESC: Returns the number of accepted recordings for presentation.
    // ------------------=
    pub fn count(&self)->usize{self.count}
    // ------------------------=
    // FUNC: add
    // DESC: Requires consistent independent samples; the third recording verifies both earlier recordings before export.
    // ------------------=
    pub fn add(&mut self,mut vector:[f32;DIM])->Result<(),Error>{
        normalize(&mut vector)?;
        if self.count>=COUNT{return Err(Error::Shape);}
        let threshold=if self.count==2{0.60}else{0.45};
        for sample in &self.samples[..self.count]{if similarity(sample,&vector)?<threshold{return Err(Error::Mismatch);}}
        self.samples[self.count]=vector;self.count+=1;Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Exports only a completed enrollment as normalized little-endian vectors.
    // ------------------=
    pub fn encode(&self,out:&mut[u8;BYTES])->Result<(),Error>{
        if self.count!=COUNT{return Err(Error::Shape);}
        for (bytes,value) in out.chunks_exact_mut(4).zip(self.samples.iter().flatten()){bytes.copy_from_slice(&value.to_le_bytes());}
        Ok(())
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases biometric samples with volatile writes on cancellation or completion.
    // ------------------=
    pub fn clear(&mut self){for x in self.samples.iter_mut().flatten(){unsafe{core::ptr::write_volatile(x,0.0);}}self.count=0;}
}
impl Drop for Enrollment{
    // ------------------------=
    // FUNC: drop
    // DESC: Erases retained biometric vectors before releasing enrollment state.
    // ------------------=
    fn drop(&mut self){self.clear();}
}
// ------------------------=
// FUNC: matches
// DESC: Compares against the normalized enrollment centroid and requires agreement with two recordings; invalid templates fail closed.
// ------------------=
pub fn matches(template:&[u8;BYTES],vector:&[f32;DIM])->Result<bool,Error>{
    let mut accepted=0;let mut centroid=[0f32;DIM];
    for raw in template.chunks_exact(DIM*4){
        let mut sample=[0f32;DIM];
        for (v,b) in sample.iter_mut().zip(raw.chunks_exact(4)){*v=f32::from_le_bytes(b.try_into().map_err(|_|Error::Shape)?);}
        normalize(&mut sample)?;
        for (total,value) in centroid.iter_mut().zip(sample){*total+=value;}
        if similarity(&sample,vector)?>=0.45{accepted+=1;}
        for x in &mut sample{unsafe{core::ptr::write_volatile(x,0.0);}}
    }
    let score=similarity(&centroid,vector)?;
    for x in &mut centroid{unsafe{core::ptr::write_volatile(x,0.0);}}
    Ok(accepted>=2&&score>=0.60)
}

#[cfg(test)]
mod tests {
 use super::*;
 // ------------------------=
 // FUNC: enrollment_requires_consistent_complete_samples
 // DESC: Incomplete, inconsistent and malformed samples cannot produce a usable enrollment.
 // ------------------=
 #[test]
 fn enrollment_requires_consistent_complete_samples(){
  let mut e=Enrollment::new();let mut out=[0;BYTES];let mut a=[0.0;DIM];a[0]=1.0;let mut b=[0.0;DIM];b[1]=1.0;
  assert_eq!(e.encode(&mut out),Err(Error::Shape));
  e.add(a).unwrap();assert_eq!(e.add(b),Err(Error::Mismatch));assert_eq!(e.count(),1);
  assert_eq!(e.add([f32::NAN;DIM]),Err(Error::Numeric));
  e.add(a).unwrap();e.add(a).unwrap();e.encode(&mut out).unwrap();
  assert_eq!(matches(&out,&a),Ok(true));assert_eq!(matches(&out,&b),Ok(false));
  out[..4].copy_from_slice(&f32::NAN.to_le_bytes());assert!(matches(&out,&a).is_err());
  e.clear();assert_eq!(e.count(),0);assert_eq!(e.encode(&mut out),Err(Error::Shape));
 }
}
