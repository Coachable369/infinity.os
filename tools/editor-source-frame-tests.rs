use super::{ai, Action, TextDocument};

// ------------------------=
// FUNC: panel
// DESC: Creates an explicitly authorized source transaction with the production delivery cursor.
// ------------------=
fn panel() -> ai::Panel {
    let mut panel = ai::Panel::new();
    let request = b"Create a simple calculator app";
    panel.request[..request.len()].copy_from_slice(request);
    panel.request_len = request.len();
    panel.generation = ai::GenerationStatus::Running;
    panel
}

// ------------------------=
// FUNC: drain
// DESC: Applies only production-selected contiguous source bytes and commits them after editor acceptance.
// ------------------=
fn drain(panel: &mut ai::Panel, document: &mut TextDocument) {
    while let Some(range) = panel.stream.ready(&panel.argument[..panel.argument_len], None, true) {
        assert!(document.append_generation_preview(&panel.argument[range.clone()], range.start == 0));
        assert!(panel.stream.committed(range.end, None));
    }
}

// ------------------------=
// FUNC: run
// DESC: Verifies bytewise source boundaries, repeated frames, literal preservation, stop/retry and changed-prefix fencing against actual editor bytes.
// ------------------=
pub fn run() {
    let source = b"#include <stdio.h>\n/*\nEND_ACTION\nREPLACE_COMPILE\n*/\nint main(void) { puts(\"END_ACTION\"); return 0; }";
    for (prefix, suffix) in [
        (b"REPLACE_COMPILE\n".as_slice(), b"\nEND_ACTION\nREPLACE_COMPILE\nint main(void){return 2;}".as_slice()),
        (b"  REPLACE_COMPILE\r\n", b"\r\n   END_ACTION  \r\nREPLACE_COMPILE\nint main(void){return 2;}"),
        (b"REPLACE_COMPILE\n```c\n", b"\n```\nEND_ACTION\nREPLACE_COMPILE\nint main(void){return 2;}"),
        (b"", b"\nEND_ACTION\nREPLACE_COMPILE\nint main(void){return 2;}"),
    ] {
        let response = [prefix, source, suffix].concat();
        let mut panel = panel();
        let mut document = TextDocument::new();
        for end in 1..=response.len() {
            panel.stream_model_output(2, &response[..end]);
            drain(&mut panel, &mut document);
            assert_ne!(panel.generation, ai::GenerationStatus::Failed);
        }
        assert!(panel.accept_generated(&response, true, true));
        drain(&mut panel, &mut document);
        assert_eq!(document.bytes(), source);
        assert_eq!(&panel.argument[..panel.argument_len], source);
        assert_eq!(panel.pending, Action::ReplaceCompile);
    }
    let mut stopped = panel();
    let response = [b"REPLACE_COMPILE\n".as_slice(), source].concat();
    stopped.stream_model_output(2, &response);
    let mut document = TextDocument::new();
    assert!(document.append_generation_preview(&stopped.argument[..10], true));
    assert!(stopped.stream.committed(10, None));
    stopped.cancel_delivery();
    assert_eq!(stopped.argument_len, 10);
    assert!(stopped.argument[10..source.len()].iter().all(|byte| *byte == 0));
    assert_eq!(stopped.pending, Action::None);
    assert_eq!(document.bytes(), &source[..10]);
    let late = stopped.stream_model_output(2, &response);
    assert!(!late.progressed);
    assert_eq!(stopped.argument_len, 10);
    stopped.reset_delivery();
    assert_eq!(stopped.argument_len, 0);
    assert_eq!(stopped.stream.delivered, 0);
    stopped.generation = ai::GenerationStatus::Running;
    let replacement = b"int main(void) { return 42; }";
    stopped.stream_model_output(2, &[b"REPLACE_COMPILE\n".as_slice(), replacement].concat());
    drain(&mut stopped, &mut document);
    assert_eq!(document.bytes(), replacement);
    stopped.stream_model_output(2, b"REPLACE_COMPILE\nint main(void) { return 13; }");
    assert_eq!(stopped.generation, ai::GenerationStatus::Failed);
    assert_eq!(stopped.pending, Action::None);
    assert_eq!(document.bytes(), replacement);

    let mut malformed = panel();
    let repeated = b"REPLACE_COMPILE\nint main(void){return 0;}\nREPLACE_COMPILE\nint main(void){return 1;}";
    assert!(!malformed.accept_generated(repeated, true, true));
    assert_eq!(malformed.pending, Action::None);
    assert!(!malformed.accept_generated(b"REPLACE_COMPILE\nint main(void){return 0;}\nEND_ACT", true, true));
    assert!(!malformed.accept_generated(b"REPLACE_COMPILE\n```c\nint main(void){return 0;}\nEND_ACTION", true, true));
}
