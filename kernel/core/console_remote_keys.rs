//! Bounded fragmented terminal key decoding, independent of SSH transport and editor state.
use super::ConsoleKey;
pub const ESCAPE_TIMEOUT_MS:u64=50;
pub struct Events(pub [Option<ConsoleKey>;2]);
impl Events {
    // ------------------------=
    // FUNC: push
    // DESC: Keeps Escape and its following ordinary key ordered without dropping the latter.
    // ------------------=
    fn push(&mut self,key:Option<ConsoleKey>){if self.0[0].is_none(){self.0[0]=key;}else{self.0[1]=key;}}
}
pub struct Decoder {escape:[u8;16],length:usize,started:u64,after_cr:bool}
impl Decoder {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an independent key decoder for one authenticated terminal stream.
    // ------------------=
    pub const fn new()->Self{Self{escape:[0;16],length:0,started:0,after_cr:false}}
    // ------------------------=
    // FUNC: expire
    // DESC: Delivers a standalone Escape after the bounded sequence window without requiring another input byte.
    // ------------------=
    pub fn expire(&mut self,now:u64)->Option<ConsoleKey>{if self.length!=0&&now.saturating_sub(self.started)>=ESCAPE_TIMEOUT_MS{self.length=0;Some(ConsoleKey::Escape)}else{None}}
    // ------------------------=
    // FUNC: plain
    // DESC: Decodes ASCII controls and folds a fragmented CRLF pair into one Enter event.
    // ------------------=
    fn plain(&mut self,byte:u8,now:u64)->Option<ConsoleKey>{
        let after_cr=self.after_cr;self.after_cr=byte==b'\r';
        match byte{27=>{self.escape[0]=27;self.length=1;self.started=now;None},b'\n' if after_cr=>None,
            13|10=>Some(ConsoleKey::Enter),8|127=>Some(ConsoleKey::Backspace),9=>Some(ConsoleKey::Tab(false)),
            1..=26=>Some(ConsoleKey::Shortcut(b'a'+byte-1)),32..=126=>Some(ConsoleKey::Character(byte)),_=>None}
    }
    // ------------------------=
    // FUNC: feed
    // DESC: Recognizes CSI and SS3 keys across arbitrary packet boundaries while discarding unsupported sequences safely.
    // ------------------=
    pub fn feed(&mut self,byte:u8,now:u64)->Events{
        let mut events=Events([self.expire(now),None]);
        if self.length==0{events.push(self.plain(byte,now));return events;}
        if self.length==1&&!matches!(byte,b'['|b'O'){self.length=0;events.push(Some(ConsoleKey::Escape));events.push(self.plain(byte,now));return events;}
        if self.length==self.escape.len(){self.length=0;return events;}
        self.escape[self.length]=byte;self.length+=1;
        if self.length==2{return events;}
        if !(0x40..=0x7e).contains(&byte){return events;}
        let key=match &self.escape[..self.length]{
            b"\x1b[A"|b"\x1bOA"=>Some(ConsoleKey::Up),b"\x1b[B"|b"\x1bOB"=>Some(ConsoleKey::Down),
            b"\x1b[C"|b"\x1bOC"=>Some(ConsoleKey::Right),b"\x1b[D"|b"\x1bOD"=>Some(ConsoleKey::Left),
            b"\x1b[H"|b"\x1bOH"|b"\x1b[1~"|b"\x1b[7~"=>Some(ConsoleKey::Home),
            b"\x1b[F"|b"\x1bOF"|b"\x1b[4~"|b"\x1b[8~"=>Some(ConsoleKey::End),
            b"\x1b[3~"=>Some(ConsoleKey::Delete),b"\x1b[Z"=>Some(ConsoleKey::Tab(true)),_=>None,
        };self.length=0;events.push(key);events
    }
}
