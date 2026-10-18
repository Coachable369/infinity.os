//! Streaming attention reference kernel: linear scratch, no image-token squared matrix.
use crate::Error;

// ------------------------=
// FUNC: attend
// DESC: Computes one joint-attention query using stable online softmax and bounded cancellation checkpoints.
// ------------------=
pub fn attend(query:&[f32],keys:&[f32],values:&[f32],valid:Option<&[bool]>,scale:f32,out:&mut[f32],exp:impl Fn(f32)->f32,mut keep_running:impl FnMut()->bool)->Result<(),Error>{
    let width=query.len();
    if width==0||out.len()!=width||keys.is_empty()||keys.len()%width!=0||values.len()!=keys.len(){return Err(Error::Shape);}
    let count=keys.len()/width;
    if valid.is_some_and(|m|m.len()!=count){return Err(Error::Shape);}
    if !scale.is_finite()||scale<=0.||query.iter().chain(keys).chain(values).any(|x|!x.is_finite()){return Err(Error::NonFinite);}
    out.fill(0.);let mut maximum=f32::NEG_INFINITY;let mut total=0.;
    for row in 0..count{
        if row%32==0&&!keep_running(){out.fill(0.);return Err(Error::Cancelled);}
        if valid.is_some_and(|m|!m[row]){continue;}
        let k=&keys[row*width..(row+1)*width];let v=&values[row*width..(row+1)*width];
        let score=query.iter().zip(k).map(|(q,k)|q*k).sum::<f32>()*scale;
        if !score.is_finite(){out.fill(0.);return Err(Error::NonFinite);}
        let next=maximum.max(score);let old_weight=if total==0.{0.}else{exp(maximum-next)};let weight=exp(score-next);
        if !old_weight.is_finite()||!weight.is_finite()||old_weight<0.||weight<0.||old_weight>1.||weight>1.{out.fill(0.);return Err(Error::NonFinite);}
        total=total*old_weight+weight;
        for (acc,value) in out.iter_mut().zip(v){*acc=*acc*old_weight+value*weight;}
        maximum=next;
    }
    if total==0.{return Err(Error::Shape);}
    for value in out.iter_mut(){*value/=total;}
    if out.iter().any(|x|!x.is_finite()){out.fill(0.);return Err(Error::NonFinite);}
    Ok(())
}

#[cfg(test)]
mod tests{
    extern crate std;
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: stable_attention_and_padding
    // DESC: Checks weighted values against an independent softmax calculation and excludes padded text tokens.
    // ------------------=
    fn stable_attention_and_padding(){
        let mut out=[0.;2];
        attend(&[1.,0.],&[1000.,0.,1001.,0.],&[2.,4.,6.,8.],None,1.,&mut out,|x|x.exp(),||true).unwrap();
        let p=1./(1.+(-1f32).exp());assert!((out[0]-(2.+4.*p)).abs()<0.00001);assert!((out[1]-(4.+4.*p)).abs()<0.00001);
        attend(&[1.,0.],&[1000.,0.,1001.,0.],&[2.,4.,6.,8.],Some(&[true,false]),1.,&mut out,|x|x.exp(),||true).unwrap();assert_eq!(out,[2.,4.]);
        assert_eq!(attend(&[1.,0.],&[1.,0.],&[2.,4.],None,1.,&mut out,|x|x.exp(),||false),Err(Error::Cancelled));assert_eq!(out,[0.,0.]);
    }
    #[test]
    // ------------------------=
    // FUNC: attention_rejects_invalid_shapes_and_math
    // DESC: Rejects mismatched tokens and nonfinite activations before output can be reported as valid.
    // ------------------=
    fn attention_rejects_invalid_shapes_and_math(){
        let mut out=[0.;2];
        assert_eq!(attend(&[1.,0.],&[1.],&[2.,4.],None,1.,&mut out,|x|x.exp(),||true),Err(Error::Shape));
        assert_eq!(attend(&[1.,0.],&[f32::NAN,0.],&[2.,4.],None,1.,&mut out,|x|x.exp(),||true),Err(Error::NonFinite));
    }
}
