#![allow(dead_code)]
#[path="../kernel/ui/clipboard.rs"] mod clipboard;
use clipboard::*;
const OWNER:[u8;16]=[1;16];
// ------------------------=
// FUNC: session
// DESC: Creates an unlocked test session with real clipboard state.
// ------------------=
fn session()->SessionClipboard {let mut c=SessionClipboard::new();c.session(OWNER,true);c}
#[test]
// ------------------------=
// FUNC: bounded_deduplicated_search
// DESC: Exercises eviction, duplicate promotion, full-payload filtering and restoration.
// ------------------=
fn bounded_deduplicated_search() {
    let mut c=session();
    for n in 0..10 {c.write_public_text(OWNER,&[b'a'+n],n as u64).unwrap();}
    let entries=c.history_previews(OWNER,b"",10);
    assert_eq!(entries.iter().filter(|e|e.id!=0).count(),8);
    assert_eq!(entries[0].bytes[0],b'j');assert_eq!(entries[7].bytes[0],b'c');
    c.write_public_text(OWNER,b"d",11).unwrap();
    assert_eq!(c.history_previews(OWNER,b"",12)[0].bytes[0],b'd');
    let mut text=[b'a';MAX_CLIPBOARD_BYTES];text[MAX_CLIPBOARD_BYTES-6..].copy_from_slice(b"Needle");
    c.write_public_text(OWNER,&text,13).unwrap();
    let id=c.history_previews(OWNER,b"needle",14)[0].id;assert_ne!(id,0);
    let mut short=[42;4];assert_eq!(c.history_read(OWNER,id,&mut short,14),Err(ClipboardError::TooLarge));assert_eq!(short,[42;4]);
    c.history_select(OWNER,id,15).unwrap();let mut out=[0;MAX_CLIPBOARD_BYTES];
    assert_eq!(c.read(OWNER,ClipboardKind::Utf8Text,&mut out,15),Ok(text.len()));assert_eq!(out,text);
    assert!(c.history_remove(OWNER,id));assert_eq!(c.history_select(OWNER,id,16),Err(ClipboardError::Empty));
}
#[test]
// ------------------------=
// FUNC: history_authority_and_expiry
// DESC: Verifies session fences, locking, retention deadlines and explicit erasure.
// ------------------=
fn history_authority_and_expiry() {
    let mut c=session();c.write_public_text(OWNER,b"retained",100).unwrap();
    assert_eq!(c.history_previews([2;16],b"",100),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
    assert!(!c.history_remove([2;16],1));
    assert!(c.configure(OWNER,1,true,true,100));
    assert_ne!(c.history_previews(OWNER,b"",1099)[0].id,0);
    assert_eq!(c.history_previews(OWNER,b"",1100),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
    c.write_public_text(OWNER,b"next",2000).unwrap();c.session(OWNER,false);c.session(OWNER,true);
    assert_eq!(c.history_previews(OWNER,b"",2001),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
    c.write_public_text(OWNER,b"next",2002).unwrap();c.session([2;16],true);
    assert_eq!(c.history_previews([2;16],b"",2003),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
}
#[test]
// ------------------------=
// FUNC: unclassified_and_browser_copies_are_not_retained
// DESC: Ensures ordinary clipboard use does not silently create sensitive history.
// ------------------=
fn unclassified_and_browser_copies_are_not_retained() {
    let mut c=session();c.write(OWNER,ClipboardKind::Utf8Text,b"private",10).unwrap();
    let epoch=c.epoch(OWNER);c.grant(epoch,true,11);c.browser_write(b"web password",12).unwrap();
    assert_eq!(c.history_previews(OWNER,b"",13),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
    c.write_public_text(OWNER,b"safe",14).unwrap();let id=c.history_previews(OWNER,b"",14)[0].id;
    assert_eq!(c.write_public_text(OWNER,&[255],15),Err(ClipboardError::InvalidType));
    assert_eq!(c.write_public_text([2;16],b"no",15),Err(ClipboardError::AccessDenied));
    assert_eq!(c.history_previews(OWNER,b"",15)[0].id,id);
    c.grant(epoch,false,16);c.history_select(OWNER,id,17).unwrap();
    assert_eq!(c.browser_read(&mut [0;20],18),Err(ClipboardError::AccessDenied));
    c.clear();assert_eq!(c.history_previews(OWNER,b"",19),[HistoryPreview::EMPTY;HISTORY_CAPACITY]);
}
