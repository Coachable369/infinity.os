//! Desktop-thread UIKit transitions; quantized frames invalidate only the owning control.
use super::app_launcher::motion::Motion;
use core::sync::atomic::{AtomicU8,Ordering};
static mut CURVES:[Motion;4]=[Motion::settled(0);4];
static mut TARGET:[i32;4]=[0;4];
static mut LAST:[u8;4]=[0;4];
static DIRTY:AtomicU8=AtomicU8::new(0);

// ------------------------=
// FUNC: advance
// DESC: Eases metric, artwork reveal and hover values from their current frame; reduced motion commits immediately.
// ------------------=
pub fn advance(id:usize,value:u8,now:u64,reduced:bool)->bool{
    if id>=4{return false}
    unsafe{
        if TARGET[id]!=i32::from(value) || (reduced && CURVES[id].active(now)){
            TARGET[id]=i32::from(value);
            CURVES[id].retarget(i32::from(value),now,if id==3{150}else{320},reduced);
        }
        let next=CURVES[id].value(now) as u8;
        if LAST[id]==next{return false}
        LAST[id]=next; invalidate(id);true
    }
}
// ------------------------=
// FUNC: value
// DESC: Reads a prepared keyframe without advancing timing during a clipped repaint.
// ------------------=
pub fn value(id:usize)->u8{unsafe{LAST[id]}}
// ------------------------=
// FUNC: invalidate
// DESC: Marks metrics, weather or the shelf for regional composition.
// ------------------=
pub fn invalidate(id:usize){DIRTY.fetch_or(1<<id,Ordering::Relaxed);}
// ------------------------=
// FUNC: take
// DESC: Consumes the requested control's pending paint only.
// ------------------=
pub fn take(mask:u8)->bool{DIRTY.fetch_and(!mask,Ordering::Relaxed)&mask!=0}

static CLOUD_PHASE:AtomicU8=AtomicU8::new(0);
// ------------------------=
// FUNC: weather_phase
// DESC: Advances the repeating sky transition and invalidates only weather, respecting reduced motion.
// ------------------=
pub fn weather_phase(now:u64,reduced:bool)->bool {
    let phase=if reduced{0}else{((now/250)%128) as u8};
    if CLOUD_PHASE.swap(phase,Ordering::Relaxed)==phase{return false}
    invalidate(2);true
}
// ------------------------=
// FUNC: cloud_phase
// DESC: Samples the prepared cloud transition without changing it during composition.
// ------------------=
pub fn cloud_phase()->u8{CLOUD_PHASE.load(Ordering::Relaxed)}
