#![no_std]
//! Native speaker embeddings for the pinned WeSpeaker ResNet34.
//! This module does not issue authentication credentials or claim anti-spoofing.
use infinity_face_core::inference::{Execution,Model};
pub mod policy;
pub const FRAMES:usize=200;
pub const SAMPLES:usize=400+(FRAMES-1)*160;
pub const DIM:usize=256;
pub const FEATURES:usize=80*FRAMES;
pub const TRUNK_OUTPUT:usize=256*10*25;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Error { Shape, Numeric, Model, Cancelled, Quiet, Clipped, Mismatch }

// ------------------------=
// FUNC: features
// DESC: Computes 16 kHz Kaldi-compatible log mel energies with Povey windows and utterance mean subtraction.
// ------------------=
pub fn features(pcm:&[i16],out:&mut [f32;FEATURES])->Result<(),Error> {
    if pcm.len()!=SAMPLES {return Err(Error::Shape);}
    let energy=pcm.iter().map(|&v|(v as f64)*(v as f64)).sum::<f64>()/pcm.len() as f64;
    if energy<100.0 {return Err(Error::Quiet);}
    if pcm.iter().filter(|&&v|v.unsigned_abs()>=32760).count()>pcm.len()/100 {return Err(Error::Clipped);}
    let mut window=[0f32;400];
    for (i,w) in window.iter_mut().enumerate(){*w=libm::powf(0.5-0.5*libm::cosf(2.0*core::f32::consts::PI*i as f32/399.0),0.85);}
    let low=1127.0*libm::logf(1.0+20.0/700.0);
    let high=1127.0*libm::logf(1.0+8000.0/700.0);
    let delta=(high-low)/81.0;
    let mut filters=[[0f32;257];80];
    for (b,filter) in filters.iter_mut().enumerate(){
        let left=low+b as f32*delta;
        for (k,w) in filter.iter_mut().enumerate(){
            let m=1127.0*libm::logf(1.0+(k as f32*16000.0/512.0)/700.0);
            *w=((m-left)/delta).min((left+2.0*delta-m)/delta).max(0.0);
        }
    }
    for frame in 0..FRAMES {
        let slice=&pcm[frame*160..frame*160+400];
        let mean=slice.iter().map(|&v|v as f32).sum::<f32>()/400.0;
        let mut real=[0f32;512];let mut imag=[0f32;512];
        for i in 0..400 {
            let x=slice[i] as f32-mean;
            let prior=slice[i.saturating_sub(1)] as f32-mean;
            real[i]=(x-0.97*prior)*window[i];
        }
        fft(&mut real,&mut imag);
        let mut power=[0f32;257];
        for k in 0..257 {power[k]=real[k]*real[k]+imag[k]*imag[k];}
        for b in 0..80 {
            let e=power.iter().zip(&filters[b]).map(|(a,w)|a*w).sum::<f32>();
            out[b*FRAMES+frame]=libm::logf(e.max(f32::EPSILON));
        }
    }
    for band in out.chunks_exact_mut(FRAMES) {
        let mean=band.iter().sum::<f32>()/FRAMES as f32;
        for x in band {*x-=mean;}
    }
    Ok(())
}
// ------------------------=
// FUNC: fft
// DESC: Performs a fixed-size in-place radix-two transform without allocation or host services.
// ------------------=
fn fft(real:&mut[f32;512],imag:&mut[f32;512]) {
    for i in 0usize..512 {
        let j=i.reverse_bits()>>(usize::BITS-9);
        if i<j {real.swap(i,j);imag.swap(i,j);}
    }
    let mut length=2;
    while length<=512 {
        let angle=-2.0*core::f32::consts::PI/length as f32;
        let wr=libm::cosf(angle);let wi=libm::sinf(angle);
        for start in (0..512).step_by(length) {
            let (mut r,mut q)=(1.0,0.0);
            for k in 0..length/2 {
                let a=start+k;let b=a+length/2;
                let tr=r*real[b]-q*imag[b];let ti=r*imag[b]+q*real[b];
                real[b]=real[a]-tr;imag[b]=imag[a]-ti;real[a]+=tr;imag[a]+=ti;
                let nr=r*wr-q*wi;q=r*wi+q*wr;r=nr;
            }
        }
        length*=2;
    }
}
// ------------------------=
// FUNC: embed
// DESC: Runs the authenticated fixed model with bounded inference visits and caller-owned private memory.
// ------------------=
pub fn embed(model:&[u8],head:&[u8],input:&[f32;FEATURES],arena:&mut[f32],trunk:&mut[f32;TRUNK_OUTPUT],out:&mut[f32;DIM],mut visit:impl FnMut()->bool)->Result<(),Error> {
    if head.len()!=40+(DIM*5120+DIM*2)*4 || &head[..8]!=b"ISPKHEAD" || model.get(8..40)!=Some(&head[8..40]) {return Err(Error::Model);}
    let m=Model::parse(model).map_err(|_|Error::Model)?;
    let mut run=Execution::new(m,arena).map_err(|_|Error::Model)?;
    let target=run.input().map_err(|_|Error::Model)?;
    if target.len()!=FEATURES{return Err(Error::Shape);}
    target.copy_from_slice(input);
    loop {
        if !visit(){run.cancel();return Err(Error::Cancelled);}
        // The native scheduler preempts every millisecond. Amortize voluntary
        // ABI round trips while retaining bounded cancellation between batches.
        let mut complete=false;
        for _ in 0..16 {
            if run.step(4096).map_err(|_|Error::Model)? {complete=true;break;}
        }
        if complete {break;}
    }
    run.output(0,trunk).map_err(|_|Error::Model)?;
    let mut pooled=[0f32;5120];
    for (i,values) in trunk.chunks_exact(25).enumerate() {
        let mean=values.iter().sum::<f32>()/25.0;
        let var=values.iter().map(|v|(v-mean)*(v-mean)).sum::<f32>()/24.0;
        pooled[i]=mean;pooled[2560+i]=libm::sqrtf(var+1e-5);
    }
    for (i,value) in out.iter_mut().enumerate() {
        let mut sum=read(head,40+(DIM*5120+i)*4)?;
        for (j,x) in pooled.iter().enumerate(){sum+=x*read(head,40+(i*5120+j)*4)?;}
        *value=sum-read(head,40+(DIM*5120+DIM+i)*4)?;
    }
    normalize(out)
}
// ------------------------=
// FUNC: read
// DESC: Reads finite projection weights without alignment assumptions.
// ------------------=
fn read(bytes:&[u8],at:usize)->Result<f32,Error>{
    let value=f32::from_le_bytes(bytes.get(at..at+4).ok_or(Error::Model)?.try_into().map_err(|_|Error::Model)?);
    if value.is_finite(){Ok(value)}else{Err(Error::Numeric)}
}
// ------------------------=
// FUNC: normalize
// DESC: Rejects degenerate embeddings and normalizes vectors for cosine comparison.
// ------------------=
pub fn normalize(out:&mut[f32;DIM])->Result<(),Error>{
    if out.iter().any(|x|!x.is_finite()){return Err(Error::Numeric);}
    let norm=libm::sqrtf(out.iter().map(|x|x*x).sum());
    if !norm.is_finite()||norm<1e-8{return Err(Error::Numeric);}
    for v in out {*v/=norm;}
    Ok(())
}
// ------------------------=
// FUNC: similarity
// DESC: Computes a checked cosine score; caller policy decides whether any score is acceptable.
// ------------------=
pub fn similarity(a:&[f32;DIM],b:&[f32;DIM])->Result<f32,Error>{
    let mut a=*a;let mut b=*b;normalize(&mut a)?;normalize(&mut b)?;
    Ok(a.iter().zip(b).map(|(x,y)|x*y).sum::<f32>().clamp(-1.0,1.0))
}
