#[path = "../kernel/ui/generation_stream.rs"] mod generation_stream;
#[path = "../kernel/ui/text_editor.rs"] mod text_editor;
#[path = "../kernel/ui/text_input.rs"] pub mod text_input;
mod ui { pub use crate::text_input; }
use generation_stream::{Stream, MAX_FLUSH_BYTES, FLUSH_INTERVAL_NS};

// ------------------------=
// FUNC: main
// DESC: Exercises pacing, bounded final draining, exact document delivery, undo, and edit preservation through production APIs.
// ------------------=
fn main() {
    let mut doc = text_editor::TextDocument::new();
    assert!(doc.replace_generation_preview(b"original user document", false));
    let mut stream = Stream::new();
    let source = vec![b'x'; 4097];
    let first = stream.ready(&source, Some(1), false).unwrap();
    assert_eq!(first, 0..MAX_FLUSH_BYTES);
    assert!(doc.append_generation_preview(&source[first.clone()], true));
    assert!(stream.committed(first.end, Some(1)));
    assert!(stream.ready(&source, Some(2), false).is_none());
    assert!(stream.ready(&source, Some(FLUSH_INTERVAL_NS + 1), false).is_some());
    while let Some(range) = stream.ready(&source, Some(2), true) {
        assert!(range.len() <= MAX_FLUSH_BYTES);
        assert_eq!(range.start, doc.bytes().len());
        assert!(doc.append_generation_preview(&source[range.clone()], false));
        assert!(stream.committed(range.end, Some(2)));
    }
    assert_eq!(doc.bytes(), &source);
    assert_eq!(stream.delivered, source.len());
    assert_eq!(stream.flushes, 17);
    assert!(!doc.append_generation_preview(&[b'y'; 257], false));
    assert!(!stream.committed(stream.delivered + 257, Some(3)));
    assert!(doc.undo());
    assert_eq!(doc.bytes(), b"original user document");
    assert!(stream.ready(&source, None, true).is_none());
    let mut growing = Stream::new();
    for length in 1..=80 {
        if let Some(range) = growing.ready(&source[..length], Some(length as u64 * 1_000_000), false) {
            assert_eq!(range.start, growing.delivered);
            assert!(growing.committed(range.end, Some(length as u64 * 1_000_000)));
        }
    }
    assert_eq!(growing.flushes, 5);
}
