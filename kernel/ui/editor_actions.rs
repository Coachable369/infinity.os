//! Atomic document transformations shared by menus, commands and behavioral tests.
use super::super::text_editor::{TextDocument, DOCUMENT_CAPACITY};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Word,
    Line,
    DeleteLine,
    MoveUp,
    MoveDown,
    Join,
    Sort,
    Trim,
    Upper,
    Lower,
    Comment,
    Indent,
    Outdent,
}
pub const ITEMS: [(&[u8], Action); 13] = [
    (b"Select word", Action::Word),
    (b"Select line", Action::Line),
    (b"Delete lines", Action::DeleteLine),
    (b"Move lines up", Action::MoveUp),
    (b"Move lines down", Action::MoveDown),
    (b"Join lines", Action::Join),
    (b"Sort lines", Action::Sort),
    (b"Trim trailing whitespace", Action::Trim),
    (b"Uppercase selection", Action::Upper),
    (b"Lowercase selection", Action::Lower),
    (b"Toggle line comments", Action::Comment),
    (b"Indent selection", Action::Indent),
    (b"Outdent selection", Action::Outdent),
];
// ------------------------=
// FUNC: apply
// DESC: Computes complete bounded edits before one undoable commit; failures preserve content and selection.
// ------------------=
pub fn apply(doc: &mut TextDocument, action: Action, prefix: &[u8]) -> bool {
    let bytes = doc.bytes();
    let len = bytes.len();
    let cursor = doc.cursor();
    let old = doc.selection();
    let (mut a, mut b) = old.unwrap_or((cursor, cursor));
    if action == Action::Word {
        a = cursor;
        b = cursor;
        while a > 0 && (bytes[a - 1].is_ascii_alphanumeric() || bytes[a - 1] == b'_') {
            a -= 1;
        }
        while b < len && (bytes[b].is_ascii_alphanumeric() || bytes[b] == b'_') {
            b += 1;
        }
        doc.select(a, b);
        return a != b;
    }
    if !matches!(action, Action::Upper | Action::Lower) {
        a = bytes[..a]
            .iter()
            .rposition(|c| *c == b'\n')
            .map_or(0, |i| i + 1);
        if b == a || b == 0 || bytes[b - 1] != b'\n' {
            b = bytes[b..]
                .iter()
                .position(|c| *c == b'\n')
                .map_or(len, |i| b + i + 1);
        }
    }
    if action == Action::Line {
        doc.select(a, b);
        return a != b;
    }
    if matches!(action, Action::Upper | Action::Lower) && a == b {
        return false;
    }
    let mut out = [0u8; DOCUMENT_CAPACITY];
    let mut n = 0;
    let mut moved = None;
    let mut append = |part: &[u8]| -> bool {
        if n + part.len() > out.len() {
            return false;
        }
        out[n..n + part.len()].copy_from_slice(part);
        n += part.len();
        true
    };
    let trailing = b > a && bytes[b - 1] == b'\n';
    let end = if trailing { b - 1 } else { b };
    match action {
        Action::DeleteLine => {}
        Action::Upper | Action::Lower => {
            for c in &bytes[a..b] {
                if !append(&[if action == Action::Upper {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                }]) {
                    return false;
                }
            }
        }
        Action::MoveUp => {
            if a == 0 {
                return false;
            }
            let prev = bytes[..a - 1]
                .iter()
                .rposition(|c| *c == b'\n')
                .map_or(0, |i| i + 1);
            moved = Some((prev, prev + end - a + 1));
            if !append(&bytes[a..end])
                || !append(b"\n")
                || !append(&bytes[prev..a - 1])
                || (trailing && !append(b"\n"))
            {
                return false;
            }
            a = prev;
        }
        Action::MoveDown => {
            if b == len {
                return false;
            }
            let next = bytes[b..]
                .iter()
                .position(|c| *c == b'\n')
                .map_or(len, |i| b + i);
            moved = Some((
                a + next - b + 1,
                a + next - b + 1 + end - a + usize::from(next < len),
            ));
            if !append(&bytes[b..next])
                || !append(b"\n")
                || !append(&bytes[a..end])
                || (next < len && !append(b"\n"))
            {
                return false;
            }
            b = if next < len { next + 1 } else { next };
        }
        Action::Join => {
            if old.is_none() && b < len {
                b = bytes[b..]
                    .iter()
                    .position(|c| *c == b'\n')
                    .map_or(len, |i| b + i + 1);
            }
            let trailing = b > a && bytes[b - 1] == b'\n';
            let end = if trailing { b - 1 } else { b };
            for (i, line) in bytes[a..end].split(|c| *c == b'\n').enumerate() {
                let line = if i == 0 {
                    line
                } else {
                    let start = line
                        .iter()
                        .position(|c| !matches!(c, b' ' | b'\t'))
                        .unwrap_or(line.len());
                    &line[start..]
                };
                if (i > 0 && !append(b" ")) || !append(line) {
                    return false;
                }
            }
            if trailing && !append(b"\n") {
                return false;
            }
        }
        Action::Sort => {
            let mut lines = [(0u16, 0u16); DOCUMENT_CAPACITY + 1];
            let mut count = 0;
            let mut start = a;
            for line in bytes[a..end].split(|c| *c == b'\n') {
                if count == lines.len() {
                    return false;
                }
                lines[count] = (start as u16, (start + line.len()) as u16);
                start += line.len() + 1;
                count += 1;
            }
            lines[..count].sort_unstable_by(|x, y| {
                bytes[x.0 as usize..x.1 as usize].cmp(&bytes[y.0 as usize..y.1 as usize])
            });
            for (i, (s, e)) in lines[..count].iter().enumerate() {
                if (i > 0 && !append(b"\n")) || !append(&bytes[*s as usize..*e as usize]) {
                    return false;
                }
            }
            if trailing && !append(b"\n") {
                return false;
            }
        }
        _ => {
            let uncomment = action == Action::Comment
                && !prefix.is_empty()
                && bytes[a..end]
                    .split(|c| *c == b'\n')
                    .all(|line| line.starts_with(prefix));
            if action == Action::Comment && prefix.is_empty() {
                return false;
            }
            for (i, line) in bytes[a..end].split(|c| *c == b'\n').enumerate() {
                if i > 0 && !append(b"\n") {
                    return false;
                }
                let line = match action {
                    Action::Trim => {
                        let e = line
                            .iter()
                            .rposition(|c| !matches!(c, b' ' | b'\t' | b'\r'))
                            .map_or(0, |j| j + 1);
                        &line[..e]
                    }
                    Action::Outdent => {
                        let n = if line.first() == Some(&b'\t') {
                            1
                        } else {
                            line.iter().take(4).take_while(|c| **c == b' ').count()
                        };
                        &line[n..]
                    }
                    Action::Indent => {
                        if !append(b"    ") {
                            return false;
                        }
                        line
                    }
                    Action::Comment if uncomment => &line[prefix.len()..],
                    Action::Comment => {
                        if !append(prefix) {
                            return false;
                        }
                        line
                    }
                    _ => line,
                };
                if !append(line) {
                    return false;
                }
            }
            if trailing && !append(b"\n") {
                return false;
            }
        }
    }
    if bytes[a..b] == out[..n] {
        return false;
    }
    doc.select(a, b);
    if !doc.replace_selection(&out[..n]) {
        if let Some((x, y)) = old {
            doc.select(if cursor == x { y } else { x }, cursor)
        } else {
            doc.set_cursor(cursor)
        }
        return false;
    }
    if let Some((start, end)) = moved {
        doc.select(start, end)
    } else {
        doc.select(a, a + n)
    }
    true
}
