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
        if reserve>N-self.len || commands.len()>N-self.len-reserve {return false;}
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
