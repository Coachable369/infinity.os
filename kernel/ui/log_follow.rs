//! Bounded snapshot follower for native terminal logs. No disk I/O or terminal policy.
pub const CAPACITY: usize = 16 * 1024;
#[derive(Debug, PartialEq, Eq)]
pub struct Options<'a> { pub path: &'a [u8], pub lines: Option<usize>, pub follow: bool }
// ------------------------=
// FUNC: options
// DESC: Parses shared cat/tail options with explicit end-of-options and quoted paths, rejecting ambiguous or excessive arguments.
// ------------------=
pub fn options(command: &[u8]) -> Result<Options<'_>, ()> {
    let mut rest=command;
    let name=word(&mut rest)?.ok_or(())?;
    if name!=b"cat" && name!=b"tail" {return Err(());}
    let mut result=Options {path:b"",lines:if name==b"tail"{Some(10)}else{None},follow:false};
    let mut flags=true;
    while let Some(arg)=word(&mut rest)? {
        if flags && arg==b"--" {flags=false;continue;}
        if flags && arg==b"-f" {result.follow=true;continue;}
        if flags && arg==b"-n" {
            let value=word(&mut rest)?.ok_or(())?;
            if value.is_empty() {return Err(());}
            let mut number=0usize;
            for &digit in value {if !digit.is_ascii_digit(){return Err(());}number=number.checked_mul(10).and_then(|n|n.checked_add((digit-b'0')as usize)).ok_or(())?;}
            result.lines=Some(number);continue;
        }
        if flags && arg.starts_with(b"-") || !result.path.is_empty() {return Err(());}
        result.path=arg;
    }
    if result.path.is_empty(){Err(())}else{Ok(result)}
}
// ------------------------=
// FUNC: word
// DESC: Borrows one bounded argument without allocation or shell expansion.
// ------------------=
fn word<'a>(rest:&mut &'a [u8])->Result<Option<&'a [u8]>,()> {
    *rest=rest.trim_ascii_start();if rest.is_empty(){return Ok(None);}
    if matches!(rest[0],b'\''|b'"') {
        let quote=rest[0];let end=rest[1..].iter().position(|&b|b==quote).ok_or(())?+1;
        let value=&rest[1..end];*rest=&rest[end+1..];
        if !rest.is_empty()&&!rest[0].is_ascii_whitespace(){return Err(());}Ok(Some(value))
    }else{let end=rest.iter().position(u8::is_ascii_whitespace).unwrap_or(rest.len());let value=&rest[..end];*rest=&rest[end..];Ok(Some(value))}
}
pub struct Follow { bytes: [u8; CAPACITY], length: usize, initialized: bool }
impl Follow {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes an empty bounded comparison snapshot.
    // ------------------=
    pub const fn new() -> Self { Self { bytes: [0; CAPACITY], length: 0, initialized: false } }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases retained log content at cancellation or session revocation.
    // ------------------=
    pub fn clear(&mut self) { self.bytes.fill(0); self.length=0; self.initialized=false; }
    // ------------------------=
    // FUNC: update
    // DESC: Returns new bytes only, restarting on truncation or replacement and retaining partial lines without duplication.
    // ------------------=
    pub fn update(&mut self, input: &[u8], initial_lines: Option<usize>) -> Result<(usize,bool), ()> {
        if input.len()>CAPACITY { return Err(()); }
        let replaced=self.initialized && (input.len()<self.length || input[..self.length]!=self.bytes[..self.length]);
        let start=if !self.initialized { initial_lines.map_or(0,|n|tail_start(input,n)) }
            else if replaced {0} else {self.length};
        self.bytes.fill(0); self.bytes[..input.len()].copy_from_slice(input);
        self.length=input.len();self.initialized=true;
        Ok((start,replaced))
    }
}
// ------------------------=
// FUNC: tail_start
// DESC: Locates the last requested logical lines, counting an unterminated final line once.
// ------------------=
pub fn tail_start(input:&[u8],lines:usize)->usize {
    if lines==0{return input.len();}
    let end=input.len().saturating_sub(usize::from(input.last()==Some(&b'\n')));
    let mut remaining=lines;
    for index in (0..end).rev() {
        if input[index]==b'\n' {remaining-=1;if remaining==0{return index+1;}}
    }
    0
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: parses_shared_command_options
    // DESC: Verifies command defaults, quoted paths, zero lines, option termination and malformed input rejection.
    // ------------------=
    #[test]
    fn parses_shared_command_options(){
        assert_eq!(options(b"tail -f -n 2 '/Logs/a b.log'"),Ok(Options{path:b"/Logs/a b.log",lines:Some(2),follow:true}));
        assert_eq!(options(b"cat /System/motd"),Ok(Options{path:b"/System/motd",lines:None,follow:false}));
        assert_eq!(options(b"tail -- -filename"),Ok(Options{path:b"-filename",lines:Some(10),follow:false}));
        assert_eq!(options(b"cat -n 0 /Logs/a"),Ok(Options{path:b"/Logs/a",lines:Some(0),follow:false}));
        for invalid in [b"tail".as_slice(),b"cat -n -2 x",b"cat a b",b"tail 'unterminated",b"cat -x a",b"cat -n 999999999999999999999999999 x"]{assert!(options(invalid).is_err());}
    }
    // ------------------------=
    // FUNC: follows_append_partial_lines_and_rotation
    // DESC: Exercises incremental bytes, unchanged snapshots, truncation and same-size replacement.
    // ------------------=
    #[test]
    fn follows_append_partial_lines_and_rotation(){
        let mut f=Follow::new();
        assert_eq!(f.update(b"a\nb",Some(1)),Ok((2,false)));
        assert_eq!(f.update(b"a\nb",None),Ok((3,false)));
        assert_eq!(f.update(b"a\nbc\n",None),Ok((3,false)));
        assert_eq!(f.update(b"new!!",None),Ok((0,true)));
        assert_eq!(f.update(b"x",None),Ok((0,true)));
        f.clear();assert_eq!(f.update(b"a\nb\n",Some(1)),Ok((2,false)));
        assert_eq!(tail_start(b"a\nb\n",0),4);
        assert_eq!(tail_start(b"a\nb\n",5),0);
        assert_eq!(tail_start(b"",1),0);
    }
    // ------------------------=
    // FUNC: oversized_snapshot_preserves_cursor
    // DESC: Rejects overflow without advancing or erasing the previous stream position.
    // ------------------=
    #[test]
    fn oversized_snapshot_preserves_cursor(){
        let mut f=Follow::new();assert_eq!(f.update(b"a",None),Ok((0,false)));
        assert_eq!(f.update(&[0;CAPACITY+1],None),Err(()));
        assert_eq!(f.update(b"ab",None),Ok((1,false)));
    }
}
