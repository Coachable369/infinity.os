//! Flow-matching Euler integration. The model-specific schedule is supplied explicitly.
use crate::Error;

// ------------------------=
// FUNC: validate_schedule
// DESC: Rejects unbounded, nonfinite, increasing or unterminated noise schedules before inference starts.
// ------------------=
pub fn validate_schedule(sigmas: &[f32]) -> Result<(), Error> {
    if sigmas.len()<2 || sigmas.len()>101 || sigmas.last()!=Some(&0.) { return Err(Error::Shape); }
    if sigmas.iter().any(|s|!s.is_finite() || *s<0. || *s>1.) { return Err(Error::NonFinite); }
    if sigmas.windows(2).any(|p|p[0]<=p[1]) { return Err(Error::Shape); }
    Ok(())
}

// ------------------------=
// FUNC: euler_step
// DESC: Advances a flow latent using observed model velocity, committing only after all arithmetic validates.
// ------------------=
pub fn euler_step(latent: &mut [f32], velocity: &[f32], sigma: f32, next: f32) -> Result<(), Error> {
    if latent.is_empty() || latent.len()!=velocity.len() { return Err(Error::Shape); }
    if !sigma.is_finite() || !next.is_finite() || sigma>1. || sigma<=next || next<0. { return Err(Error::NonFinite); }
    let dt=next-sigma;
    if latent.iter().zip(velocity).any(|(x,v)|!x.is_finite() || !v.is_finite() || !(x+dt*v).is_finite()) {return Err(Error::NonFinite);}
    for (x,v) in latent.iter_mut().zip(velocity) {*x+=dt*v;}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: integration_and_atomic_rejection
    // DESC: Verifies Euler output and guarantees rejected nonfinite predictions cannot partially mutate latents.
    // ------------------=
    fn integration_and_atomic_rejection() {
        let mut x=[1.,3.];euler_step(&mut x,&[2.,-4.],1.,0.5).unwrap();assert_eq!(x,[0.,5.]);
        assert_eq!(euler_step(&mut x,&[1.,f32::NAN],0.5,0.),Err(Error::NonFinite));assert_eq!(x,[0.,5.]);
        assert_eq!(validate_schedule(&[1.,0.5,0.]),Ok(()));
        assert_eq!(validate_schedule(&[1.,0.5,0.5,0.]),Err(Error::Shape));
        assert_eq!(validate_schedule(&[1.,0.5]),Err(Error::Shape));
    }
}
