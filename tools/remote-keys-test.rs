#![allow(dead_code)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ConsoleKey{Shortcut(u8),Character(u8),Backspace,Enter,Escape,Tab(bool),Up,Down,Left,Right,Home,End,Delete}
#[path="../kernel/core/console_remote_keys.rs"]mod keys;
#[path="../kernel/core/console_remote_screen.rs"]mod screen;
// ------------------------=
// FUNC: decode
// DESC: Feeds exact wire bytes one at a time through the production decoder and returns semantic key events.
// ------------------=
fn decode(decoder:&mut keys::Decoder,bytes:&[u8],now:u64)->Vec<ConsoleKey>{bytes.iter().flat_map(|b|decoder.feed(*b,now).0.into_iter().flatten()).collect()}
// ------------------------=
// FUNC: main
// DESC: Verifies fragmented ANSI, SS3, standalone Escape timeout, CRLF folding, control keys and independent terminal streams.
// ------------------=
fn main(){
    editor_cells();
    use ConsoleKey::*;
    for(sequence,expected)in[(b"\x1b[A".as_slice(),Up),(b"\x1bOB",Down),(b"\x1b[C",Right),(b"\x1bOD",Left),(b"\x1bOH",Home),(b"\x1bOF",End),(b"\x1b[7~",Home),(b"\x1b[8~",End),(b"\x1b[3~",Delete),(b"\x1b[Z",Tab(true))]{
        for split in 0..=sequence.len(){let mut decoder=keys::Decoder::new();let mut out=decode(&mut decoder,&sequence[..split],1);out.extend(decode(&mut decoder,&sequence[split..],2));assert_eq!(out,vec![expected]);assert!(decoder.expire(100).is_none());}
    }
    let mut decoder=keys::Decoder::new();assert!(decode(&mut decoder,b"\x1b",100).is_empty());assert!(decoder.expire(149).is_none());assert_eq!(decoder.expire(150),Some(Escape));assert!(decoder.expire(151).is_none());
    assert_eq!(decode(&mut decoder,b"\x1bx",200),vec![Escape,Character(b'x')]);assert!(decode(&mut decoder,b"\x1b",300).is_empty());assert_eq!(decode(&mut decoder,b"z",351),vec![Escape,Character(b'z')]);
    assert_eq!(decode(&mut decoder,b"\r",400),vec![Enter]);assert!(decode(&mut decoder,b"\n",401).is_empty());assert_eq!(decode(&mut decoder,b"\n\r\rx",402),vec![Enter,Enter,Enter,Character(b'x')]);
    assert_eq!(decode(&mut decoder,&[1,3,7,9,19,23,24,127],500),vec![Shortcut(b'a'),Shortcut(b'c'),Shortcut(b'g'),Tab(false),Shortcut(b's'),Shortcut(b'w'),Shortcut(b'x'),Backspace]);
    let mut other=keys::Decoder::new();assert!(decode(&mut decoder,b"\x1b[",600).is_empty());assert_eq!(decode(&mut other,b"A",601),vec![Character(b'A')]);assert_eq!(decode(&mut decoder,b"A",602),vec![Up]);
    assert!(decode(&mut decoder,b"\x1b[999~",700).is_empty());assert_eq!(decode(&mut decoder,b"k",701),vec![Character(b'k')]);
    for byte in 0..=255{let mut decoder=keys::Decoder::new();for _ in 0..64{let _=decoder.feed(byte,1);}let _=decoder.expire(100);}
    println!("PASS remote terminal wire-key behavior");
}

// ------------------------=
// FUNC: editor_cells
// DESC: Verifies production tab-expanded cursor geometry and clipped viewport bytes without using terminal prose as an oracle.
// ------------------=
fn editor_cells(){
    let source=b"\t\t\t\t\tA";assert_eq!(screen::position(source,5,20),(1,0));assert_eq!(screen::position(source,6,20),(1,1));
    let mut cells=[0xa5;60];assert!(screen::cells(source,0,20,2,&mut cells));assert!(cells[..20].iter().all(|b|*b==b' '));assert_eq!(cells[20],b'A');assert!(cells[21..40].iter().all(|b|*b==b' '));assert!(cells[40..].iter().all(|b|*b==0xa5));
    cells.fill(0xa5);assert!(screen::cells(source,1,20,1,&mut cells));assert_eq!(cells[0],b'A');assert!(cells[1..20].iter().all(|b|*b==b' '));assert!(cells[20..].iter().all(|b|*b==0xa5));
    assert_eq!(screen::position(b"ab\ncd",5,20),(1,2));assert_eq!(screen::position(b"abcd\nX",6,4),(1,1));assert_eq!(screen::position(b"abc\tZ",5,4),(1,1));assert_eq!(screen::position(b"a\r\nb",99,20),(1,1));
    cells.fill(0xa5);assert!(!screen::cells(source,0,0,2,&mut cells));assert!(!screen::cells(source,0,100,2,&mut cells));assert!(cells.iter().all(|b|*b==0xa5));
}
