//! Bounded native field editing with selection and one-step undo/redo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Snapshot<const N: usize> { bytes: [u8; N], length: usize, caret: usize, anchor: usize }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field<const N: usize> { state: Snapshot<N>, undo: Option<Snapshot<N>>, redo: Option<Snapshot<N>>, owner: Option<[u8;16]> }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key { Character(u8), Backspace, Delete, Move(i8,bool), SelectAll, Undo, Redo }

impl<const N: usize> Field<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty field with no prior editable state.
    // ------------------=
    pub const fn new() -> Self { Self {state:Snapshot {bytes:[0;N],length:0,caret:0,anchor:0},undo:None,redo:None,owner:None} }
    // ------------------------=
    // FUNC: clear
    // DESC: Clears text and reversible snapshots while retaining the current session binding.
    // ------------------=
    pub fn clear(&mut self) {let owner=self.owner;*self=Self::new();self.owner=owner;}
    // ------------------------=
    // FUNC: bind
    // DESC: Invalidates native drafts and undo history on lock, logout or a different authenticated session.
    // ------------------=
    pub fn bind(&mut self,session:[u8;16],active:bool)->bool {
        let owner=(active && session!=[0;16]).then_some(session);
        if self.owner==owner {return false;}
        self.clear();self.owner=owner;true
    }
    // ------------------------=
    // FUNC: text
    // DESC: Returns the initialized text bytes only.
    // ------------------=
    pub fn text(&self) -> &[u8] { &self.state.bytes[..self.state.length] }
    // ------------------------=
    // FUNC: caret
    // DESC: Returns the bounded insertion position.
    // ------------------=
    pub fn caret(&self) -> usize { self.state.caret }
    // ------------------------=
    // FUNC: position
    // DESC: Synchronizes an external native caret without discarding the field's edit history.
    // ------------------=
    pub fn position(&mut self,caret:usize,anchor:usize) {
        self.state.caret=caret.min(self.state.length);self.state.anchor=anchor.min(self.state.length);
    }
    // ------------------------=
    // FUNC: selection
    // DESC: Returns the ordered selection, including an empty caret range.
    // ------------------=
    pub fn selection(&self) -> (usize,usize) { (self.state.caret.min(self.state.anchor),self.state.caret.max(self.state.anchor)) }
    // ------------------------=
    // FUNC: set
    // DESC: Replaces externally supplied text and clears history without resetting an unchanged field.
    // ------------------=
    pub fn set(&mut self, bytes: &[u8]) {
        let bytes=&bytes[..bytes.len().min(N)];
        if self.text()==bytes {return;}
        self.clear();self.state.bytes[..bytes.len()].copy_from_slice(bytes);
        self.state.length=bytes.len();self.state.caret=bytes.len();self.state.anchor=bytes.len();
    }
    // ------------------------=
    // FUNC: replace
    // DESC: Replaces the selected range atomically, rejecting invalid or oversized clipboard input.
    // ------------------=
    pub fn replace(&mut self, bytes: &[u8]) -> bool {
        if bytes.iter().any(|c| !(32..=126).contains(c)) {return false;}
        let (a,b)=self.selection();
        let length=self.state.length-(b-a);
        if bytes.len()>N-length {return false;}
        let old=self.state;
        self.state.bytes.copy_within(b..self.state.length,a+bytes.len());
        self.state.bytes[a..a+bytes.len()].copy_from_slice(bytes);
        self.state.length=length+bytes.len();self.state.bytes[self.state.length..].fill(0);
        self.state.caret=a+bytes.len();self.state.anchor=self.state.caret;
        if self.state==old {return false;}
        self.undo=Some(old);self.redo=None;true
    }
    // ------------------------=
    // FUNC: key
    // DESC: Applies selection-aware editing, navigation and reversible changes without forwarding to a web page.
    // ------------------=
    pub fn key(&mut self,key:Key) -> bool {
        let old=self.state;
        match key {
            Key::Character(c)=>{return self.replace(&[c]);},
            Key::SelectAll=>{self.state.anchor=0;self.state.caret=self.state.length;},
            Key::Undo=>{if let Some(state)=self.undo.take(){self.redo=Some(self.state);self.state=state;}},
            Key::Redo=>{if let Some(state)=self.redo.take(){self.undo=Some(self.state);self.state=state;}},
            Key::Move(direction,extend)=>{
                let (a,b)=self.selection();
                self.state.caret=match direction {
                    -2=>0,2=>self.state.length,
                    -1 if !extend && a!=b=>a,1 if !extend && a!=b=>b,
                    -1=>self.state.caret.saturating_sub(1),1=>(self.state.caret+1).min(self.state.length),_=>self.state.caret,
                };
                if !extend {self.state.anchor=self.state.caret;}
            },
            Key::Backspace|Key::Delete=>{
                let (a,b)=self.selection();
                if a==b {
                    if key==Key::Backspace {self.state.anchor=a.saturating_sub(1);}
                    else {self.state.anchor=(a+1).min(self.state.length);}
                }
                let changed=self.replace(&[]);
                if changed {self.undo=Some(old);}
                return changed;
            },
        }
        self.state!=old
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: selection_clipboard_caret_and_history_are_transactional
    // DESC: Exercises replacement, bounded paste, directional selection, deletion and undo/redo state.
    // ------------------=
    #[test]
    fn selection_clipboard_caret_and_history_are_transactional() {
        let mut field=Field::<8>::new();field.set(b"abcd");
        field.key(Key::SelectAll);assert_eq!(field.selection(),(0,4));
        let selected=field;
        assert!(!field.replace(b"bad\n"));assert_eq!(field,selected);
        assert!(!field.replace(b"too large"));assert_eq!(field,selected);
        assert!(field.key(Key::Character(b'x')));assert_eq!(field.text(),b"x");
        let edited=field;field.set(b"x");assert_eq!(field,edited);
        field.key(Key::Undo);assert_eq!(field.state,selected.state);
        field.key(Key::Redo);assert_eq!(field.text(),b"x");
        field.set(b"abcd");field.key(Key::Move(-1,true));field.key(Key::Move(-1,true));
        assert_eq!(field.selection(),(2,4));assert_eq!(field.caret(),2);
        field.key(Key::Delete);assert_eq!(field.text(),b"ab");
        field.key(Key::Undo);assert_eq!(field.text(),b"abcd");assert_eq!(field.selection(),(2,4));
        field.key(Key::Move(-1,false));assert_eq!(field.caret(),2);
        field.key(Key::Backspace);assert_eq!(field.text(),b"acd");
        field.key(Key::Move(2,false));assert!(!field.key(Key::Delete));
    }
    // ------------------------=
    // FUNC: omnibox_focus_sync_preserves_atomic_paste_and_history
    // DESC: Replays native address synchronization, select-all replacement, undo/redo and rejected clipboard state.
    // ------------------=
    #[test]
    fn omnibox_focus_sync_preserves_atomic_paste_and_history() {
        let mut field=Field::<2048>::new();field.set(b"https://example.test/");
        let length=field.text().len();field.position(length,0);
        assert!(field.replace(b"https://other.test/"));
        field.set(b"https://other.test/");field.position(field.text().len(),field.text().len());
        field.key(Key::Undo);assert_eq!(field.text(),b"https://example.test/");
        assert_eq!(field.selection(),(0,length));
        field.key(Key::Redo);assert_eq!(field.text(),b"https://other.test/");
        field.key(Key::SelectAll);let previous=field;
        assert!(!field.replace(b"javascript:\nmalformed"));assert_eq!(field,previous);
        field.key(Key::Move(-1,false));assert_eq!(field.caret(),0);
        field.key(Key::Character(b'x'));assert_eq!(field.text(),b"xhttps://other.test/");
        field.key(Key::Move(2,false));field.key(Key::Backspace);
        assert_eq!(field.text(),b"xhttps://other.test");
        field=Field::new();assert!(!field.key(Key::Undo));assert!(field.text().is_empty());
    }
    // ------------------------=
    // FUNC: session_changes_clear_drafts_and_both_history_directions
    // DESC: Verifies same-session editing survives synchronization but lock, logout and another user cannot restore prior text.
    // ------------------=
    #[test]
    fn session_changes_clear_drafts_and_both_history_directions() {
        let mut field=Field::<32>::new();assert!(field.bind([1;16],true));
        field.set(b"private query");field.key(Key::SelectAll);field.replace(b"replacement");
        assert!(!field.bind([1;16],true));assert!(field.key(Key::Undo));
        assert_eq!(field.text(),b"private query");
        assert!(field.bind([2;16],true));assert!(field.text().is_empty());
        assert!(!field.key(Key::Undo));assert!(!field.key(Key::Redo));
        field.set(b"second user");field.key(Key::Backspace);
        assert!(field.bind([2;16],false));assert!(field.text().is_empty());
        assert!(field.bind([2;16],true));assert!(!field.key(Key::Undo));
        field.set(b"last draft");field.clear();assert!(!field.bind([2;16],true));
        assert!(!field.key(Key::Undo));assert!(!field.key(Key::Redo));
        assert!(field.bind([0;16],true));assert!(field.text().is_empty());
    }
}
