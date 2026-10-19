//! Bounded source-name selection; generated prose never authorizes a file choice.

// ------------------------=
// FUNC: source_path
// DESC: Limits discoverable application sources to native C and Python documents.
// ------------------=
pub(crate) fn source_path(path: &[u8]) -> bool {
    path.ends_with(b".c") || path.ends_with(b".py")
}

// ------------------------=
// FUNC: snapshot_matches
// DESC: Requires a version-pinned read to retain its exact owned object and current version after reading.
// ------------------=
pub(crate) fn snapshot_matches(expected: ([u8; 16], u32), read_object: [u8; 16], current: Option<([u8; 16], u32)>) -> bool {
    expected.0 != [0; 16] && expected.1 > 0 && read_object == expected.0 && current == Some(expected)
}

// ------------------------=
// FUNC: saved_source_matches
// DESC: Refuses unsaved or stale editor bytes instead of binding them to a newer persisted source version.
// ------------------=
pub(crate) fn saved_source_matches(saved: bool, editor: &[u8], snapshot: &[u8]) -> bool {
    saved && !editor.is_empty() && editor == snapshot
}

// ------------------------=
// FUNC: current_target
// DESC: Recognizes only explicit references to the document already open in the editor.
// ------------------=
pub(crate) fn current_target(target: &[u8]) -> bool {
    [b"current".as_slice(), b"this", b"current file", b"current document", b"open file", b"this file", b"this document"]
        .iter().any(|value| target.eq_ignore_ascii_case(value))
}

// ------------------------=
// FUNC: matches
// DESC: Matches an exact path, basename or complete filename-slug words without inspecting arbitrary source prose.
// ------------------=
pub(crate) fn matches(target: &[u8], path: &[u8]) -> bool {
    if !source_path(path) || target.is_empty() { return false; }
    if target.contains(&b'/') { return target == path; }
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or(path);
    if target.contains(&b'.') { return name.eq_ignore_ascii_case(target); }
    let stem = &name[..name.iter().rposition(|byte| *byte == b'.').unwrap_or(name.len())];
    let mut count = 0;
    for word in target.split(|byte| !byte.is_ascii_alphanumeric()).filter(|word| !word.is_empty()) {
        count += 1;
        if !stem.split(|byte| !byte.is_ascii_alphanumeric()).any(|part| part.eq_ignore_ascii_case(word)) {
            return false;
        }
    }
    count > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: update_admission_requires_matching_saved_version
    // DESC: Rejects stale buffers, unsaved edits, replaced objects and version changes without mutating the inspected source.
    // ------------------=
    #[test]
    fn update_admission_requires_matching_saved_version() {
        let receipt = ([1; 16], 7);
        let editor = b"print(1)\n";
        let newer = b"print(2)\n";
        assert!(snapshot_matches(receipt, [1; 16], Some(receipt)));
        assert!(saved_source_matches(true, editor, editor));
        assert!(!saved_source_matches(false, editor, editor));
        assert!(!saved_source_matches(true, editor, newer));
        assert!(!saved_source_matches(true, b"", b""));
        assert!(!snapshot_matches(receipt, [2; 16], Some(receipt)));
        assert!(!snapshot_matches(receipt, [1; 16], Some(([1; 16], 8))));
        assert!(!snapshot_matches(receipt, [1; 16], Some(([2; 16], 7))));
        assert!(!snapshot_matches(receipt, [1; 16], None));
        assert!(!snapshot_matches(([0; 16], 7), [0; 16], Some(([0; 16], 7))));
        assert!(!snapshot_matches(([1; 16], 0), [1; 16], Some(([1; 16], 0))));
        assert_eq!(editor, b"print(1)\n");
        assert_eq!(newer, b"print(2)\n");
    }

    // ------------------------=
    // FUNC: source_selection_requires_specific_names
    // DESC: Exercises exact paths, semantic slugs and rejection of unrelated or legacy anonymous source names.
    // ------------------=
    #[test]
    fn source_selection_requires_specific_names() {
        assert!(matches(b"calculator", b"/home/default/documents/calculator-123.c"));
        assert!(matches(b"calendar planner", b"/home/default/documents/calendar-planner-123.py"));
        assert!(matches(b"Chess.py", b"/home/default/documents/chess.py"));
        assert!(!matches(b"calc", b"/home/default/documents/calculator.py"));
        assert!(!matches(b"calculator", b"/home/default/documents/calendar.py"));
        assert!(!matches(b"calculator", b"/home/default/documents/app-c-123.c"));
        assert!(!matches(b"calculator", b"/home/default/documents/calculator.elf"));
        assert!(!matches(b"", b"/home/default/documents/calculator.py"));
        assert!(!matches(b"../calculator.py", b"/home/default/documents/calculator.py"));
        assert!(matches(b"/home/default/documents/chess.c", b"/home/default/documents/chess.c"));
        assert!(current_target(b"current document"));
        assert!(!current_target(b"calculator"));
    }
}
