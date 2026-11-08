#![allow(dead_code)]
#[path="../kernel/ui/mod.rs"]mod ui;
#[path="../kernel/runtime/native_console_input.rs"]mod native_input;
use ui::{editor_chrome::terminal::{self,Preferences,State,Layout,Location},geometry::{Rect,Point}};
// ------------------------=
// FUNC: main
// DESC: Exercises persistent dock choices, responsive hit geometry, bounded stream delivery and typed diagnostic coordinates.
// ------------------=
fn main(){
    terminal_editor_sessions();
    for open in [false,true]{for height in [112,224,1024]{let p=Preferences{open,height};assert_eq!(Preferences::decode(&p.encode()),Some(p));}}
    assert_eq!(Preferences::decode(&[b'I',b'E',b'T',1,2,112,0,0]),None);
    for scale in [1,2,3]{for width in [440,720,1024]{for height in [160,280,640]{
        let area=Rect{x:12,y:33,width:width*scale,height:height*scale};
        let collapsed=Layout::new(area,scale as usize,Preferences::new(),0,0);
        assert_eq!(collapsed.panel.height,32*scale);assert_eq!(collapsed.panel.bottom(),area.bottom());
        let g=Layout::new(area,scale as usize,Preferences{open:true,height:1024},300,12);
        assert!(g.panel.y>=area.y+72*scale as i32);assert_eq!(g.panel.bottom(),area.bottom());
        for r in [g.run,g.stop,g.restart,g.input,g.output,g.thumb]{assert!(r.x>=area.x&&r.right()<=area.right());assert!(r.y>=area.y&&r.bottom()<=area.bottom());}
        let row=g.row_at(Point{x:g.output.x,y:g.output.y},scale as usize,100,3).unwrap();assert_eq!(row,100-g.rows(scale as usize)-3);
        assert_eq!(g.drag_scroll(g.track.bottom(),0,scale as usize,300),0);
        assert_eq!(g.drag_scroll(g.track.y,0,scale as usize,300),300-g.rows(scale as usize));
    }}}
    let mut state=State::new();state.append(b"abc");state.append(b"def\n");assert_eq!(&state.lines[0][..state.lengths[0]],b"abcdef");assert_eq!(state.count,1);
    for index in 0..400{state.append(&[(index%26)as u8+b'a',b'\n']);}assert_eq!(state.count,terminal::ROWS);assert!(state.lengths.iter().all(|n|*n<=terminal::COLS));
    assert_eq!(terminal::diagnostic(b"/home/default/main.c:42:7: error",b"/home/default/main.c"),Some(Location{line:42,column:7}));
    assert_eq!(terminal::diagnostic(b"File \"/home/default/main.py\", line 8",b"/home/default/main.py"),Some(Location{line:8,column:1}));
    assert_eq!(terminal::diagnostic(b"other.c:42:7: error",b"/home/default/main.c"),None);
    assert_eq!(terminal::diagnostic(b"notmain.c:42:7: error",b"/home/default/main.c"),None);
    let mut wrapped=State::new();wrapped.append(b"abcdefghijklmno\nsecond line\nthird line\nlast\n");
    wrapped.reflow(5,2);assert_eq!(wrapped.visual_count(5),9);assert_eq!(wrapped.visual_line(2,5),Some((0,10,15)));
    wrapped.scroll=5;wrapped.reflow(4,2);
    let anchor=wrapped.visual_count(4)-2-wrapped.scroll;assert_eq!(wrapped.visual_line(anchor,4),Some((0,8,12)));
    let before=wrapped.visual_line(anchor,4);wrapped.append(b"new output\n");wrapped.reflow(4,2);
    assert_eq!(wrapped.visual_line(wrapped.visual_count(4)-2-wrapped.scroll,4),before);
    let mut input=native_input::Input::new();assert_eq!(input.read(10,|_|true),-11);assert!(input.waiting);assert!(input.submit(b"hello"));
    assert_eq!(input.read(3,|bytes|bytes==b"hel"),3);assert!(!input.submit(b"again"));
    assert_eq!(input.read(10,|bytes|bytes==b"lo\n"),3);input.close();assert_eq!(input.read(10,|_|true),0);
    terminal::mutate(|state|*state=State::new());println!("editor terminal behavior passed");
}

// ------------------------=
// FUNC: terminal_editor_sessions
// DESC: Exercises nested remote surface selection, desktop preservation and complete private state erasure on slot reuse.
// ------------------=
fn terminal_editor_sessions(){
    use ui::terminal_edit as edit;
    edit::session([1;16]);edit::mutate(|s|{assert!(s.document.open(b"desktop"));s.document.set_cursor(7);assert!(s.document.insert(b'!'));s.active=true;});
    {
        let _first=edit::select_remote(0);edit::session([2;16]);edit::mutate(|s|{assert!(s.document.open(b"remote-a"));s.document.set_cursor(8);assert!(s.document.insert(b'!'));s.active=true;s.path[..7].copy_from_slice(b"private");s.path_len=7;s.cut[0]=65;s.cut_len=1;s.query[0]=66;s.query_len=1;s.revision=Some(([3;16],4));s.memory=Some(([5;16],6));s.top=9;s.prompt=2;s.closing_window=true;s.notice=b"Private state";});
        {
            let _second=edit::select_remote(1);edit::session([3;16]);edit::mutate(|s|{assert!(s.document.open(b"remote-b"));s.active=true;});
        }
        assert_eq!(edit::inspect(|s|s.owner),[2;16]);assert!(edit::inspect(|s|s.document.bytes()==b"remote-a!"));
        edit::session([4;16]);edit::mutate(|s|{assert!(s.document.bytes().is_empty());assert!(!s.document.undo());assert!(!s.document.redo());assert!(!s.active);assert!(!s.closing_window);assert_eq!(s.top,0);assert_eq!(s.revision,None);assert_eq!(s.memory,None);assert_eq!((s.path_len,s.cut_len,s.query_len,s.prompt),(0,0,0,0));assert!(s.path.iter().chain(s.cut.iter()).chain(s.query.iter()).all(|b|*b==0));assert!(s.notice.is_empty());});
    }
    assert_eq!(edit::inspect(|s|s.owner),[1;16]);assert!(edit::inspect(|s|s.document.bytes()==b"desktop!"));edit::mutate(|s|{assert!(s.document.undo());assert_eq!(s.document.bytes(),b"desktop");});
    {let _second=edit::select_remote(1);assert_eq!(edit::inspect(|s|s.owner),[3;16]);assert!(edit::inspect(|s|s.document.bytes()==b"remote-b"));edit::session([0;16]);assert!(edit::inspect(|s|s.document.bytes().is_empty()));}
    assert!(edit::inspect(|s|s.document.bytes()==b"desktop"));edit::session([0;16]);
}
