//! Bounded worker-to-compositor RGBA handoff. No framebuffer, allocation or wait.
//! Two slots allow a reader to retain a frame while the worker replaces pending
//! frames. Window generation tags prevent late delivery into a reopened window.
use core::{cell::UnsafeCell, sync::atomic::{AtomicU8, AtomicU64, Ordering}};

const FREE:u8=0;
const WRITING:u8=1;
const READY:u8=2;
const READING:u8=3;

#[derive(Debug,PartialEq,Eq)]
pub enum Error { Invalid, Busy, Exhausted }
struct Pixels<const N:usize> { generation:u64,width:u32,height:u32,length:usize,bytes:[u8;N] }
struct Slot<const N:usize> { state:AtomicU8,sequence:AtomicU64,pixels:UnsafeCell<Pixels<N>> }
pub struct Frames<const N:usize> { next:AtomicU64,slots:[Slot<N>;2] }
// A successful CAS exclusively owns each slot until release publication. Readers
// keep READING for the entire borrow; writers cannot modify a borrowed frame.
unsafe impl<const N:usize> Sync for Frames<N> {}
pub struct Frame<'a,const N:usize> { slot:&'a Slot<N> }

impl<const N:usize> Slot<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves fixed storage without runtime allocation.
    // ------------------=
    const fn new()->Self {
        Self {state:AtomicU8::new(FREE),sequence:AtomicU64::new(0),pixels:UnsafeCell::new(Pixels {
            generation:0,width:0,height:0,length:0,bytes:[0;N],
        })}
    }
}
impl<const N:usize> Frames<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a statically allocatable double-buffered frame transport.
    // ------------------=
    pub const fn new()->Self {Self {next:AtomicU64::new(1),slots:[const {Slot::new()};2]}}
    // ------------------------=
    // FUNC: publish
    // DESC: Coalesces pending pixels while never waiting for the desktop consumer.
    // ------------------=
    pub fn publish(&self,generation:u64,width:u32,height:u32,bytes:&[u8])->Result<(),Error> {
        let length=(width as usize).checked_mul(height as usize).and_then(|n|n.checked_mul(4))
            .ok_or(Error::Invalid)?;
        if generation==0 || width==0 || height==0 || length>N || bytes.len()!=length {return Err(Error::Invalid);}
        // Replace a pending frame before occupying the spare slot.
        for expected in [READY,FREE] {
            for slot in &self.slots {
                if slot.state.compare_exchange(expected,WRITING,Ordering::Acquire,Ordering::Relaxed).is_err() {continue;}
                let sequence=match self.next.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|n|n.checked_add(1)) {
                    Ok(sequence)=>sequence,
                    Err(_)=>{slot.state.store(expected,Ordering::Release);return Err(Error::Exhausted);}
                };
                let pixels=unsafe {&mut *slot.pixels.get()};
                pixels.bytes[..length].copy_from_slice(bytes);
                pixels.generation=generation;pixels.width=width;pixels.height=height;pixels.length=length;
                slot.sequence.store(sequence,Ordering::Relaxed);
                slot.state.store(READY,Ordering::Release);
                return Ok(());
            }
        }
        Err(Error::Busy)
    }
    // ------------------------=
    // FUNC: acquire
    // DESC: Borrows the newest complete frame for this window generation with bounded work.
    // ------------------=
    pub fn acquire(&self,generation:u64)->Option<Frame<'_,N>> {
        let first=if self.slots[0].sequence.load(Ordering::Relaxed)>=self.slots[1].sequence.load(Ordering::Relaxed) {0}else{1};
        for index in [first,1-first] {
            let slot=&self.slots[index];
            if slot.state.compare_exchange(READY,READING,Ordering::Acquire,Ordering::Relaxed).is_err() {continue;}
            let frame=Frame {slot};
            if frame.generation()==generation {return Some(frame);}
            drop(frame);
        }
        None
    }
}
impl<const N:usize> Frame<'_,N> {
    // ------------------------=
    // FUNC: generation
    // DESC: Identifies the originating window lifetime, not a reused window slot.
    // ------------------=
    pub fn generation(&self)->u64 {unsafe {(*self.slot.pixels.get()).generation}}
    // ------------------------=
    // FUNC: size
    // DESC: Reports the exact dimensions attached to this completed pixel buffer.
    // ------------------=
    pub fn size(&self)->(u32,u32) {let p=unsafe {&*self.slot.pixels.get()};(p.width,p.height)}
    // ------------------------=
    // FUNC: bytes
    // DESC: Lends immutable RGBA pixels while the producer is excluded from this slot.
    // ------------------=
    pub fn bytes(&self)->&[u8] {let p=unsafe {&*self.slot.pixels.get()};&p.bytes[..p.length]}
}
impl<const N:usize> Drop for Frame<'_,N> {
    // ------------------------=
    // FUNC: drop
    // DESC: Returns the consumed slot only after all borrowed pixels are released.
    // ------------------=
    fn drop(&mut self) {self.slot.state.store(FREE,Ordering::Release);}
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    // ------------------------=
    // FUNC: bounded_resize_coalescing_and_generation
    // DESC: Verifies exact pixels, stable reader ownership, bounds and stale-window rejection.
    // ------------------=
    #[test]
    fn bounded_resize_coalescing_and_generation() {
        let frames=Frames::<64>::new();
        assert_eq!(frames.publish(1,2,2,&[1;16]),Ok(()));
        let old=frames.acquire(1).unwrap();
        for value in 2..20 {assert_eq!(frames.publish(1,4,2,&[value;32]),Ok(()));}
        assert_eq!(old.bytes(),&[1;16]);
        let current=frames.acquire(1).unwrap();
        assert_eq!(current.bytes(),&[19;32]);assert_eq!(current.size(),(4,2));
        assert_eq!(frames.publish(1,1,1,&[0;4]),Err(Error::Busy));
        drop(old);drop(current);
        assert_eq!(frames.publish(1,4,4,&[3;64]),Ok(()));
        assert!(frames.acquire(2).is_none());
        assert_eq!(frames.publish(2,1,1,&[9;4]),Ok(()));
        assert_eq!(frames.acquire(2).unwrap().bytes(),&[9;4]);
        for (g,w,h,len) in [(0,1,1,4),(1,0,1,0),(1,5,5,64),(1,2,2,15)] {
            assert_eq!(frames.publish(g,w,h,&[0;64][..len]),Err(Error::Invalid));
        }
    }
    // ------------------------=
    // FUNC: concurrent_frames_never_tear
    // DESC: Exercises real producer/consumer overlap and final delivery with patterned pixel buffers.
    // ------------------=
    #[test]
    fn concurrent_frames_never_tear() {
        let frames=Frames::<256>::new();
        let done=core::sync::atomic::AtomicBool::new(false);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for sequence in 1..=200u8 {
                    while frames.publish(1,8,8,&[sequence;256]).is_err() {std::thread::yield_now();}
                }
                done.store(true,Ordering::Release);
            });
            let mut last=0;
            loop {
                if let Some(frame)=frames.acquire(1) {
                    let value=frame.bytes()[0];
                    assert!(frame.bytes().iter().all(|byte|*byte==value));last=value;
                }
                if done.load(Ordering::Acquire) && last==200 {break;}
                std::thread::yield_now();
            }
        });
    }
}
