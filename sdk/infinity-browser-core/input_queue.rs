//! BSP-owned input FIFO. A logical gesture is admitted completely or not at all.
use crate::worker::Command;
pub struct Queue<const N:usize>{entries:[Command;N],head:usize,len:usize}
impl<const N:usize> Queue<N>{
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a bounded allocation-free native input queue.
    // ------------------=
    pub const fn new()->Self {Self{entries:[Command::empty();N],head:0,len:0}}
    // ------------------------=
    // FUNC: push
    // DESC: Atomically admits a complete gesture, including its eventual key release.
    // ------------------=
    pub fn push(&mut self,commands:&[Command])->bool {
        if commands.len()>N-self.len {return false;}
        for command in commands {self.entries[(self.head+self.len)%N]=*command;self.len+=1;}
        true
    }
    // ------------------------=
    // FUNC: push_reserved
    // DESC: Leaves capacity for releases of already admitted pointer buttons.
    // ------------------=
    pub fn push_reserved(&mut self,commands:&[Command],reserve:usize)->bool {
        if reserve>N-self.len {return false;}
        if commands.len()==1 && commands[0].kind==crate::worker::POINTER && self.len>0 {
            let last=(self.head+self.len-1)%N;
            if self.entries[last].kind==crate::worker::POINTER {
                self.entries[last]=commands[0];return true;
            }
        }
        if commands.len()>N-self.len-reserve {return false;}
        self.push(commands)
    }
    // ------------------------=
    // FUNC: drain
    // DESC: Retains the oldest unsent command on backpressure and bounds each UI tick's work.
    // ------------------=
    pub fn drain(&mut self,limit:usize,mut send:impl FnMut(Command)->bool)->usize {
        let mut sent=0;
        while sent<limit && self.len>0 {
            if !send(self.entries[self.head]) {break;}
            self.head=(self.head+1)%N;self.len-=1;sent+=1;
        }
        sent
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Discards stale document input when navigation or window lifetime changes.
    // ------------------=
    pub fn clear(&mut self){self.head=0;self.len=0;}
    // ------------------------=
    // FUNC: is_empty
    // DESC: Allows navigation to wait until admitted input, including releases, has reached the worker.
    // ------------------=
    pub fn is_empty(&self)->bool {self.len==0}
}
#[cfg(test)]
mod tests{
    use super::*;
    // ------------------------=
    // FUNC: motion_coalesces_without_crossing_gesture_boundaries
    // DESC: Proves queued motion stays bounded while exact click coordinates and releases survive.
    // ------------------=
    #[test]
    fn motion_coalesces_without_crossing_gesture_boundaries(){
        let mut queue=Queue::<6>::new();
        let mut motion=Command::empty();motion.kind=crate::worker::POINTER;
        for x in 0..1000 {motion.x=x;assert!(queue.push_reserved(&[motion],3));}
        let mut click=motion;click.kind=crate::worker::BUTTON;click.flags=1;
        assert!(queue.push_reserved(&[click],3));
        motion.x=1001;assert!(queue.push_reserved(&[motion],3));
        motion.x=1002;assert!(queue.push_reserved(&[motion],3));
        click.flags=0;click.x=1002;assert!(queue.push(&[click]));
        let expected=[(crate::worker::POINTER,999,0),(crate::worker::BUTTON,999,1),
            (crate::worker::POINTER,1002,0),(crate::worker::BUTTON,1002,0)];
        let mut index=0;
        assert_eq!(queue.drain(6,|c|{assert_eq!((c.kind,c.x,c.flags),expected[index]);index+=1;true}),4);
    }
    // ------------------------=
    // FUNC: gestures_remain_ordered_across_pressure_and_wrap
    // DESC: Checks atomic pair admission, exact retry, ring wrap and lifetime clearing.
    // ------------------=
    #[test]
    fn gestures_remain_ordered_across_pressure_and_wrap(){
        let mut queue=Queue::<3>::new();
        let mut down=Command::empty();down.kind=7;down.flags=1;down.a=65;
        let mut up=down;up.flags=0;
        assert!(queue.push(&[down,up]));
        assert!(!queue.is_empty());
        assert!(!queue.push(&[down,up]));
        assert_eq!(queue.drain(3,|_|false),0);
        assert_eq!(queue.drain(1,|c|{assert_eq!((c.a,c.flags),(65,1));true}),1);
        assert!(queue.push(&[down,up]));
        let mut expected=[0,1,0].into_iter();
        assert_eq!(queue.drain(8,|c|{assert_eq!(c.flags,expected.next().unwrap());true}),3);
        assert!(queue.push(&[down,up]));queue.clear();
        assert_eq!(queue.drain(8,|_|panic!("stale input")),0);
        assert!(queue.is_empty());
        assert!(!Queue::<0>::new().push(&[down]));
    }
}
