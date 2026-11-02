//! Native browser command resolution shared by input routing and behavioral tests.
use crate::worker as abi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Zoom(i32), Back, Forward, Reload, Location, Find, NewTab, CloseTab,
    Tab(bool), TabIndex(u8), Edit(u8), Page(u32), Favorite }

// ------------------------=
// FUNC: resolve
// DESC: Resolves normalized native shortcuts without giving field editing commands browser-wide effects.
// ------------------=
pub fn resolve(code: u8) -> Option<Action> {
    if code==b'Z' {return Some(Action::Edit(b'y'));}
    Some(match code.to_ascii_lowercase() {
        b'+'|b'=' => Action::Zoom(1), b'-'|b'_' => Action::Zoom(-1), b'0' => Action::Zoom(0),
        b'[' => Action::Back, b']' => Action::Forward, b'r'|0x86 => Action::Reload,
        b'l' => Action::Location, b'f' => Action::Find, b't' => Action::NewTab, b'w' => Action::CloseTab,
        0x82 => Action::Tab(false), 0x83 => Action::Tab(true),
        0x84 => Action::Page(12), 0x85 => Action::Page(13),
        b'1'..=b'9' => Action::TabIndex(code-b'1'), b'd' => Action::Favorite,
        value @ (b'a'|b'c'|b'x'|b'v'|b'z'|b'y') => Action::Edit(value),
        _ => return None,
    })
}

// ------------------------=
// FUNC: bypasses_field
// DESC: Keeps navigation, tabs, location and zoom reachable while a native field owns text editing.
// ------------------=
pub fn bypasses_field(action: Action) -> bool {
    !matches!(action, Action::Edit(_) | Action::Page(_))
}

// ------------------------=
// FUNC: wheel_command
// DESC: Converts physical wheel packets to either zoom or scrolling, never both.
// ------------------=
pub fn wheel_command(vertical: i8, modifiers: u8, x: i32, y: i32) -> Option<abi::Command> {
    if vertical == 0 { return None; }
    let mut command=abi::Command::empty();
    if modifiers & 6 != 0 {
        command.kind=abi::ZOOM; command.x=-i32::from(vertical.signum());
    } else {
        command.kind=abi::SCROLL; command.x=x; command.y=y;
        let delta=(i32::from(vertical)*48) as u32;
        if modifiers&1!=0 {command.a=delta;} else {command.b=delta;}
    }
    Some(command)
}

// ------------------------=
// FUNC: adjacent_tab
// DESC: Wraps tab traversal safely and rejects absent or stale active tabs.
// ------------------=
pub fn adjacent_tab(active: usize, count: usize, reverse: bool) -> Option<usize> {
    if count == 0 || active >= count { return None; }
    Some(if reverse { (active+count-1)%count } else { (active+1)%count })
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: commands_keep_fields_and_browser_actions_separate
    // DESC: Checks case normalization, field ownership, paging and tab wrapping with typed outcomes.
    // ------------------=
    #[test]
    fn commands_keep_fields_and_browser_actions_separate() {
        for c in b"acxvy" {for code in [*c,c.to_ascii_uppercase()] {
            assert_eq!(resolve(code),Some(Action::Edit(*c)));
            assert!(!bypasses_field(resolve(code).unwrap()));
        }}
        assert_eq!(resolve(b'z'),Some(Action::Edit(b'z')));
        assert_eq!(resolve(b'Z'),Some(Action::Edit(b'y')));
        for c in b"lrtwf+-=0[]" {assert!(bypasses_field(resolve(*c).unwrap()));}
        assert_eq!(resolve(0x84),Some(Action::Page(12)));
        assert_eq!(resolve(0x85),Some(Action::Page(13)));
        assert_eq!(adjacent_tab(0,3,true),Some(2));
        assert_eq!(adjacent_tab(2,3,false),Some(0));
        assert_eq!(adjacent_tab(0,0,false),None);
        assert_eq!(adjacent_tab(3,3,true),None);
    }
    // ------------------------=
    // FUNC: wheel_chords_never_also_scroll
    // DESC: Verifies both command modifiers, signed wheel extremes, ordinary scrolling and zero packets.
    // ------------------=
    #[test]
    fn wheel_chords_never_also_scroll() {
        for delta in [i8::MIN,-1,1,i8::MAX] {for modifiers in 0..8 {
            let c=wheel_command(delta,modifiers,10,20).unwrap();
            if modifiers&6!=0 {assert_eq!((c.kind,c.x,c.b),(abi::ZOOM,-i32::from(delta.signum()),0));}
            else {
                let scroll=i32::from(delta)*48;
                assert_eq!((c.kind,c.x,c.y),(abi::SCROLL,10,20));
                assert_eq!((c.a as i32,c.b as i32),if modifiers&1!=0 {(scroll,0)}else{(0,scroll)});
            }
        }}
        assert!(wheel_command(0,6,10,20).is_none());
    }
}
