//! Display names label homes; immutable object ownership, never the label, grants access.
pub const CAPACITY: usize = 96;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Path { pub bytes:[u8;CAPACITY], pub len:usize }
impl Path {
    // ------------------------=
    // FUNC: new
    // DESC: Converts each Unicode whitespace character to a dot without permitting traversal or separators.
    // ------------------=
    pub fn new(name:&[u8])->Option<Self>{
        let name=core::str::from_utf8(name).ok()?.trim();
        if name.is_empty(){return None;}
        let mut path=Self{bytes:[0;CAPACITY],len:6};path.bytes[..6].copy_from_slice(b"/home/");
        for c in name.chars(){
            if c.is_control()&&!c.is_whitespace() || matches!(c,'/'|'\\'|':'){return None;}
            let mut utf8=[0;4];
            let bytes=if c.is_whitespace(){b".".as_slice()}else{c.encode_utf8(&mut utf8).as_bytes()};
            if path.len+bytes.len()>69{return None;}
            path.bytes[path.len..path.len+bytes.len()].copy_from_slice(bytes);path.len+=bytes.len();
        }
        if path.bytes[6..path.len].iter().all(|b|*b==b'.'){return None;}
        Some(path)
    }
    // ------------------------=
    // FUNC: as_bytes
    // DESC: Returns only initialized namespace bytes.
    // ------------------=
    pub fn as_bytes(&self)->&[u8]{&self.bytes[..self.len]}
    // ------------------------=
    // FUNC: child
    // DESC: Builds a bounded child path without truncation or traversal.
    // ------------------=
    pub fn child(mut self,name:&[u8])->Option<Self>{
        if name.is_empty()||name.contains(&b'/')||name==b".."||self.len+1+name.len()>CAPACITY{return None;}
        self.bytes[self.len]=b'/';self.len+=1;
        self.bytes[self.len..self.len+name.len()].copy_from_slice(name);self.len+=name.len();Some(self)
    }
}
