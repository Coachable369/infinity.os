//! Pointer capture with reserved release capacity in the shared input FIFO.
use crate::{input_queue::Queue,worker as abi};
pub struct Pointer {buttons:u8,last:Option<(i32,i32)>}
impl Pointer {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an uncaptured native pointer stream.
    // ------------------=
    pub const fn new()->Self {Self{buttons:0,last:None}}
    // ------------------------=
    // FUNC: captured
    // DESC: Keeps releases routed to web content even outside its viewport.
    // ------------------=
    pub fn captured(&self)->bool {self.buttons!=0}
    // ------------------------=
    // FUNC: update
    // DESC: Delivers releases first and reserves three slots for all admitted buttons.
    // ------------------=
    pub fn update<const N:usize>(&mut self,queue:&mut Queue<N>,x:i32,y:i32,buttons:u8)->bool {
        let buttons=buttons&7;
        let mut command=abi::Command::empty();command.x=x;command.y=y;
        command.kind=abi::BUTTON;
        for index in 0..3 {
            let bit=1<<index;
            if self.buttons&bit!=0 && buttons&bit==0 {
                command.a=index;
                // All other producers reserve three slots; one release consumes
                // one reservation and clears the corresponding captured bit.
                if !queue.push(&[command]) {return false;}
                self.buttons&=!bit;
            }
        }
        if self.last!=Some((x,y)) {
            command.kind=abi::POINTER;
            if !queue.push_reserved(&[command],3) {return false;}
            self.last=Some((x,y));
        }
        command.kind=abi::BUTTON;command.flags=1;
        for index in 0..3 {
            let bit=1<<index;
            if self.buttons&bit==0 && buttons&bit!=0 {
                command.a=index;
                if !queue.push_reserved(&[command],3) {return false;}
                self.buttons|=bit;
            }
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: captured_release_survives_full_normal_queue
    // DESC: Exercises capture, queue pressure, outside release, and retained event ordering.
    // ------------------=
    #[test]
    fn captured_release_survives_full_normal_queue() {
        let mut pointer=Pointer::new();let mut queue=Queue::<8>::new();
        assert!(pointer.update(&mut queue,10,20,7));assert!(pointer.captured());
        assert!(queue.push_reserved(&[abi::Command::empty()],3));
        assert!(!queue.push_reserved(&[abi::Command::empty()],3));
        assert!(!pointer.update(&mut queue,-10,200,0));
        assert!(!pointer.captured());
        let mut events=[(0,0,0,0,0);8];let mut n=0;
        assert_eq!(queue.drain(8,|c|{events[n]=(c.kind,c.flags,c.a,c.x,c.y);n+=1;true}),8);
        for i in 0..3 {assert_eq!(events[5+i],(abi::BUTTON,0,i as u32,-10,200));}
        assert!(pointer.update(&mut queue,-10,200,0));
        assert_eq!(queue.drain(8,|c|{assert_eq!(c.kind,abi::POINTER);true}),1);
    }
}
