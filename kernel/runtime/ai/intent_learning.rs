//! Owner-scoped, explicitly taught intent references. Memory never grants capabilities.
use super::{control::Command, memory_api::{Backend, Engine}, persistent_memory::{Category, Owner}};
pub const SOURCE: &[u8] = b"user.intent.v1";

// ------------------------=
// FUNC: unquote
// DESC: Removes paired quotation marks from a taught phrase without changing its contents.
// ------------------=
fn unquote(s: &[u8]) -> &[u8] {
    let s=s.trim_ascii();
    if s.len()>1 && matches!(s[0],b'"'|b'\'') && s.last()==Some(&s[0]) {&s[1..s.len()-1]}else{s}
}

// ------------------------=
// FUNC: teaching
// DESC: Validates an explicit user-defined phrase against an existing executable native action.
// ------------------=
pub fn teaching(text: &[u8]) -> Option<(&[u8], Command)> {
    let text=text.trim_ascii();
    let prefix=b"when I say ";
    if !text.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {return None;}
    let rest=&text[prefix.len()..];
    for separator in [b", I mean ".as_slice(),b" I mean ",b", do "] {
        for i in 1..rest.len().saturating_sub(separator.len()) {
            if !rest[i..].get(..separator.len()).is_some_and(|s|s.eq_ignore_ascii_case(separator)){continue;}
            let phrase=unquote(&rest[..i]);
            let action=rest[i+separator.len()..].trim_ascii();
            if phrase.is_empty() || phrase.len()>128 || super::control::veto_action(phrase)
                || super::control::veto_action(action) {return None;}
            let command=Command::explicit(action)?;
            return Some((phrase,command));
        }
    }
    None
}

// ------------------------=
// FUNC: resolve
// DESC: Revalidates session and native action for an exact learned phrase; semantic variations use ordinary memory-grounded inference.
// ------------------=
pub fn resolve(engine:&mut Engine, backend:&mut impl Backend, owner:Owner, session:Owner, input:&[u8])->Option<Command> {
    if super::control::veto_action(input) {return None;}
    let catalog=engine.catalog(backend,owner,session).ok()?;
    let mut resolved=None;
    for record in catalog.records(owner).ok()? {
        if record.category!=Category::Procedural || record.source()!=SOURCE {continue;}
        let Some((phrase,command))=teaching(record.content())else{continue;};
        if !phrase.eq_ignore_ascii_case(input.trim_ascii()) {continue;}
        if resolved.is_some_and(|previous|previous!=command) {return None;}
        resolved=Some(command);
    }
    resolved
}

// ------------------------=
// FUNC: relevant
// DESC: Uses authorized vector retrieval to select tool-aware inference, never to authorize execution.
// ------------------=
pub fn relevant(engine:&mut Engine, backend:&mut impl Backend, owner:Owner, session:Owner, input:&[u8])->bool {
    if super::control::veto_action(input) {return false;}
    let Ok(hits)=engine.search(backend,owner,session,input,4) else{return false;};
    hits.hits[..hits.len].iter().any(|hit| {
        engine.inspect(backend,owner,session,hit.id).is_ok_and(|record|
            record.category==Category::Procedural && record.source()==SOURCE && teaching(record.content()).is_some())
    })
}
