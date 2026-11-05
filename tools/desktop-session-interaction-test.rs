#[path="../kernel/runtime/ai/session_request.rs"] mod session;
#[path="../kernel/ui/scroll_capture.rs"] mod scroll;
// ------------------------=
// FUNC: main
// DESC: Exercises direct session intent, rejection boundaries, greeting bounds and continuous thumb geometry.
// ------------------=
fn main(){
    for phrase in [b"logout".as_slice(),b"please log me out",b"Infinity can you please sign me out?",b"LOG OUT!",b"sign me out please"]{assert!(session::logout(phrase));}
    for phrase in [b"do not logout".as_slice(),b"what does logout mean",b"logout Bob",b"please do not log me out",b"logout and delete files",b"my dog says logout"]{assert!(!session::logout(phrase));}
    let mut bytes=[0;160];let n=session::welcome(b"Jonathan",&mut bytes);assert_eq!(n,23);assert_eq!(bytes[14..22],*b"Jonathan");
    let n=session::welcome(&[b'x';500],&mut bytes);assert!(n<=160);assert!(bytes[..n].iter().all(|b|(32..=126).contains(b)));
    assert_eq!(scroll::offset(30,20,200,40,10,1600),0);
    assert_eq!(scroll::offset(110,20,200,40,10,1600),800);
    assert_eq!(scroll::offset(10000,20,200,40,10,1600),1600);
    assert_eq!(scroll::offset(-10000,20,200,40,10,1600),0);
    assert_eq!(scroll::offset(100,20,40,40,10,1600),0);
    let mut prior=0;for y in 30..=190{let at=scroll::offset(y,20,200,40,10,1600);assert!(at>=prior);prior=at;}
    let window=(100,40,640,400);
    assert!(scroll::file_rail(window,1,3,60,0).is_none());
    let grid=scroll::file_rail(window,1,12,60,usize::MAX).unwrap();
    assert_eq!(grid.maximum,470);assert_eq!(grid.thumb_y+grid.thumb_height as i32,grid.y+grid.height as i32);
    assert_eq!(scroll::offset(grid.thumb_y+3,grid.y,grid.height,grid.thumb_height,3,grid.maximum),grid.maximum);
    println!("Session intent, greeting isolation and scrollbar capture geometry passed");
}
