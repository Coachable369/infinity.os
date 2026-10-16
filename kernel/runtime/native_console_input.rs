//! Bounded, line-oriented input for isolated native applications.
pub struct Input { bytes:[u8;1024], length:usize, offset:usize, pub waiting:bool, eof:bool }
impl Input {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty input endpoint without authority or ambient keyboard access.
    // ------------------=
    pub const fn new()->Self {Self{bytes:[0;1024],length:0,offset:0,waiting:false,eof:false}}
    // ------------------------=
    // FUNC: submit
    // DESC: Admits one complete line without dropping unread bytes or accepting input after EOF.
    // ------------------=
    pub fn submit(&mut self,line:&[u8])->bool {
        if self.eof || self.offset<self.length || line.len()>=self.bytes.len(){return false;}
        self.bytes[..line.len()].copy_from_slice(line);self.bytes[line.len()]=b'\n';
        self.offset=0;self.length=line.len()+1;self.waiting=false;true
    }
    // ------------------------=
    // FUNC: close
    // DESC: Signals EOF after all admitted bytes have been delivered.
    // ------------------=
    pub fn close(&mut self){self.eof=true;self.waiting=false;}
    // ------------------------=
    // FUNC: read
    // DESC: Commits consumption only after the checked user-memory copy succeeds; empty live input parks the task.
    // ------------------=
    pub fn read(&mut self,capacity:usize,copy:impl FnOnce(&[u8])->bool)->i64 {
        if capacity==0{return 0;}
        if self.offset==self.length {
            self.waiting=!self.eof;return if self.eof{0}else{-11};
        }
        let length=capacity.min(self.length-self.offset);
        if !copy(&self.bytes[self.offset..self.offset+length]){return -14;}
        self.offset+=length;length as i64
    }
}
