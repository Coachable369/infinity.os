//! Deterministic receipts for direct personal-memory requests before model inference.
use super::hermes_profile::{conversation::{self, Action}, Document};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Saved,
    Forgotten,
    Inspected,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub length: usize,
    pub outcome: Outcome,
}

// ------------------------=
// FUNC: message
// DESC: Copies a bounded native receipt without allowing response text to become an action.
// ------------------=
fn message(out: &mut [u8; 8192], text: &[u8], outcome: Outcome) -> Receipt {
    let length = text.len().min(out.len());
    out[..length].copy_from_slice(&text[..length]);
    Receipt { length, outcome }
}

// ------------------------=
// FUNC: respond
// DESC: Handles only raw user memory requests and acknowledges edits after one successful durable commit.
// ------------------=
pub fn respond(
    input: &[u8],
    document: Option<&Document>,
    mut commit: impl FnMut(&[u8], &[u8]) -> bool,
    out: &mut [u8; 8192],
) -> Option<Receipt> {
    let action = conversation::action(
        input,
        document.is_some_and(|profile| profile.enabled(b"learning_enabled")),
    );
    let mut merged = [0u8; 1024];
    let (key, value, outcome) = match action {
        Action::None => return None,
        Action::Inspect => {
            return Some(match document {
                Some(profile) => match profile.inspect(out) {
                    Ok(length) => Receipt { length, outcome: Outcome::Inspected },
                    Err(_) => message(
                        out,
                        b"Your personal memory could not be read. Existing memory has been preserved.",
                        Outcome::Failed,
                    ),
                },
                None => message(
                    out,
                    b"No personal memory is loaded for this account.",
                    Outcome::Failed,
                ),
            });
        }
        Action::Set(key, value) => (key, value, Outcome::Saved),
        Action::Fact(label, value) => {
            let mut previous=[0u8;1024];
            let length=match document.map(|d|d.get(b"facts",&mut previous)) {
                Some(Ok(length))=>length,
                _=>return Some(message(out,b"Your memory could not be read. Nothing was changed.",Outcome::Failed)),
            };
            let mut used=0;
            for line in previous[..length].split(|b|*b==b'\n').filter(|l|!l.is_empty()) {
                if line.split(|b|*b==b':').next().is_some_and(|key|key.eq_ignore_ascii_case(label)) {continue;}
                if used+line.len()+1>merged.len() {return Some(message(out,b"Your memory is full. Nothing was changed.",Outcome::Failed));}
                merged[used..used+line.len()].copy_from_slice(line);used+=line.len();merged[used]=b'\n';used+=1;
            }
            if used+label.len()+2+value.len()>merged.len() {return Some(message(out,b"Your memory is full. Nothing was changed.",Outcome::Failed));}
            merged[used..used+label.len()].copy_from_slice(label);used+=label.len();
            merged[used..used+2].copy_from_slice(b": ");used+=2;
            merged[used..used+value.len()].copy_from_slice(value);used+=value.len();
            (b"facts".as_slice(),&merged[..used],Outcome::Saved)
        }
        Action::Forget(key) => {
            let value: &[u8] = match key {
                b"wake_word" => b"Infinity",
                b"assistant_name" => b"Hermes",
                b"learning_enabled" | b"inquiries_enabled" | b"inference_enabled" => b"false",
                _ => b"",
            };
            (key, value, Outcome::Forgotten)
        }
    };
    if !commit(key, value) {
        return Some(message(
            out,
            b"Memory was not saved. Check the field, value, active account, and storage. Existing memory has been preserved.",
            Outcome::Failed,
        ));
    }
    let text: &[u8] = match (outcome, key) {
        (Outcome::Forgotten, b"wake_word" | b"assistant_name" | b"learning_enabled"
            | b"inquiries_enabled" | b"inference_enabled") =>
            b"Reset that setting in your personal memory.",
        (Outcome::Forgotten, _) => b"Removed that fact from your personal memory.",
        (_, b"timezone") => b"Saved your personal time zone in your memory.",
        (_, b"favorite_color") => b"Saved your favorite color in your personal memory.",
        (_, b"learning_enabled" | b"inquiries_enabled" | b"inference_enabled") =>
            b"Updated your personal memory settings.",
        _ => b"Updated your personal memory in your home folder.",
    };
    Some(message(out, text, outcome))
}
