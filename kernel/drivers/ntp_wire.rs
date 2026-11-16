// ------------------------=
// FUNC: correction
// DESC: Unfolds the server timestamp around trusted RTC time, compensating for bounded transit and rounding to RTC precision.
// ------------------=
pub fn correction(b:&[u8],current:u64,rtt:u64)->Option<i64>{
    if b.len()<48 || rtt>5_000_000_000 || b[0]>>6==3 || b[0]&7!=4 ||
        !matches!((b[0]>>3)&7,3|4) || !(1..=15).contains(&b[1]) || b[40..48]==[0;8]{return None}
    let secs=u32::from_be_bytes(b[40..44].try_into().ok()?);
    let fraction=u32::from_be_bytes(b[44..48].try_into().ok()?);
    let expected=(current/1_000_000_000+2_208_988_800) as u32;
    let delta=i64::from(secs.wrapping_sub(expected) as i32);
    if delta.unsigned_abs()>31_536_000{return None}
    let adj=delta*1_000_000_000+((u64::from(fraction)*1_000_000_000)>>32) as i64
        +(rtt/2) as i64-(current%1_000_000_000) as i64;
    Some((adj+adj.signum()*500_000_000)/1_000_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: valid_era_and_rejections
    // DESC: Exercises real wire timestamps on either side of the 2036 rollover plus invalid stratum, latency and leaps.
    // ------------------=
    fn valid_era_and_rejections(){
        for now in [1_795_000_000u64,2_100_000_000]{
            let mut b=[0;48];b[0]=0x24;b[1]=2;
            b[40..44].copy_from_slice(&((now+2_208_988_800+5) as u32).to_be_bytes());
            assert_eq!(correction(&b,now*1_000_000_000,20_000_000),Some(5));
            assert_eq!(correction(&b,now*1_000_000_000,6_000_000_000),None);
            b[1]=0;assert_eq!(correction(&b,now*1_000_000_000,0),None);
            b[1]=2;b[0]|=192;assert_eq!(correction(&b,now*1_000_000_000,0),None);
        }
    }
}
