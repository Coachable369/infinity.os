//! Deterministic AIKit memory commands; generated proposals use the existing authorized OS action dispatcher.
use super::{
    hermes_memory_request::{Outcome, Receipt},
    memory_api::{Backend, Engine, Operation},
    persistent_memory::{Category, Error, Owner},
};
use core::fmt::{self, Write};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Native {
    pub bytes: [u8; 512],
    pub length: usize,
}
impl Native {
    // ------------------------=
    // FUNC: decode
    // DESC: Accepts only bounded known memory operations, never an arbitrary command line.
    // ------------------=
    pub fn decode(input: &[u8]) -> Option<Self> {
        if input.len() > 512 || command(input).is_none() {
            return None;
        }
        let mut bytes = [0; 512];
        bytes[..input.len()].copy_from_slice(input);
        Some(Self {
            bytes,
            length: input.len(),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command<'a> {
    Status,
    Search(&'a [u8]),
    Inspect(u64),
    Remember(&'a [u8]),
    Update(u64, &'a [u8]),
    ForgetId(u64),
    ForgetQuery(&'a [u8]),
}
// ------------------------=
// FUNC: prefix
// DESC: Removes a case-insensitive literal prefix without altering the user's memory text.
// ------------------=
fn prefix<'a>(s: &'a [u8], p: &[u8]) -> Option<&'a [u8]> {
    s.get(..p.len())
        .filter(|s| s.eq_ignore_ascii_case(p))
        .map(|_| s[p.len()..].trim_ascii())
}
// ------------------------=
// FUNC: id
// DESC: Parses positive stable record identifiers with overflow checking.
// ------------------=
fn id(s: &[u8]) -> Option<u64> {
    let mut n = 0u64;
    if s.is_empty() {
        return None;
    }
    for &b in s {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    (n > 0).then_some(n)
}
// ------------------------=
// FUNC: command
// DESC: Decodes the CLI and model action protocol into typed operations with explicit correction targets.
// ------------------=
pub fn command(input: &[u8]) -> Option<Command<'_>> {
    let input = input.trim_ascii();
    if input.eq_ignore_ascii_case(b"status") || input.eq_ignore_ascii_case(b"stats") {
        return Some(Command::Status);
    }
    for (p, kind) in [
        (b"search ".as_slice(), 0),
        (b"remember ", 1),
        (b"inspect ", 2),
        (b"forget ", 3),
    ] {
        if let Some(value) = prefix(input, p) {
            if value.is_empty() {
                return None;
            }
            return match kind {
                0 => Some(Command::Search(value)),
                1 => Some(Command::Remember(value)),
                2 => Some(Command::Inspect(id(value)?)),
                _ => Some(
                    id(value)
                        .map(Command::ForgetId)
                        .unwrap_or(Command::ForgetQuery(value)),
                ),
            };
        }
    }
    if let Some(value) = prefix(input, b"update ") {
        let at = value.iter().position(|&b| b == b' ')?;
        let n = id(&value[..at])?;
        let text = prefix(&value[at + 1..], b"to ").unwrap_or(&value[at + 1..]);
        if !text.is_empty() {
            return Some(Command::Update(n, text));
        }
    }
    None
}
// ------------------------=
// FUNC: natural
// DESC: Recognizes explicit memory requests; other wording can use semantic OS_MEMORY tool inference.
// ------------------=
pub fn natural(input: &[u8]) -> Option<Command<'_>> {
    if super::control::veto_action(input) {
        return None;
    }
    let mut s = input.trim_ascii();
    for p in [b"Infinity, ".as_slice(), b"Infinity ", b"please "] {
        s = prefix(s, p).unwrap_or(s);
    }
    if let Some(value) = prefix(s, b"remember that ") {
        return (!value.is_empty()).then_some(Command::Remember(value));
    }
    if let Some(value) = prefix(s, b"what do you remember about ") {
        let query=value.trim_ascii_end().strip_suffix(b"?").unwrap_or(value);
        // Preserve the existing owner-profile inspection contract.
        if query.eq_ignore_ascii_case(b"me"){return None;}
        return Some(Command::Search(query));
    }
    if let Some(value) = prefix(s, b"forget memory ") {
        return Some(Command::ForgetId(id(value)?));
    }
    if let Some(value) = prefix(s, b"update memory ") {
        let at = value.iter().position(|&b| b == b' ')?;
        return Some(Command::Update(
            id(&value[..at])?,
            prefix(&value[at + 1..], b"to ").unwrap_or(&value[at + 1..]),
        ));
    }
    None
}
struct Output<'a> {
    bytes: &'a mut [u8],
    len: usize,
}
impl Write for Output<'_> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Rejects overflow rather than silently reporting an incomplete result.
    // ------------------=
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.bytes.len() - self.len {
            return Err(fmt::Error);
        }
        self.bytes[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
}
// ------------------------=
// FUNC: respond
// DESC: Executes only typed owner-authorized operations, refusing ambiguous deletions and acknowledging durable receipts.
// ------------------=
pub fn respond(
    engine: &mut Engine,
    backend: &mut impl Backend,
    owner: Owner,
    session: Owner,
    cmd: Command<'_>,
    now: u64,
    out: &mut [u8],
) -> Receipt {
    let mut output = Output { bytes: out, len: 0 };
    let result = (|| -> Result<Outcome, Error> {
        let operation = match cmd {
            Command::Status => {
                let stats = engine.execute(backend, owner, session, Operation::Status, now)?;
                write!(output,"Memories: {}\nEmbedded: {}\nStorage bytes: {}\nSemantic search: unavailable (no validated embedding model)\nRetrieval ns: {} (max {})\nStorage/index errors: {}\n",stats.count,stats.embedded,engine.storage_bytes,engine.retrieval_ns,engine.max_retrieval_ns,engine.errors).map_err(|_|Error::Capacity)?;
                return Ok(Outcome::Inspected);
            }
            Command::Inspect(id) => {
                let r = engine.inspect(backend, owner, session, id)?;
                write!(
                    output,
                    "{} {:?} created={} updated={} source={}\n{}\n",
                    r.id,
                    r.category,
                    r.created_at,
                    r.updated_at,
                    core::str::from_utf8(r.source()).map_err(|_| Error::Corrupt)?,
                    core::str::from_utf8(r.content()).map_err(|_| Error::Corrupt)?
                )
                .map_err(|_| Error::Capacity)?;
                return Ok(Outcome::Inspected);
            }
            Command::Search(query) => {
                let hits = engine.search(backend, owner, session, unquote(query), 8)?;
                if hits.len == 0 {
                    output
                        .write_str("No matching memories.")
                        .map_err(|_| Error::Capacity)?;
                }
                for hit in &hits.hits[..hits.len] {
                    let r = engine.inspect(backend, owner, session, hit.id)?;
                    writeln!(
                        output,
                        "{}: {}",
                        r.id,
                        core::str::from_utf8(r.content()).map_err(|_| Error::Corrupt)?
                    )
                    .map_err(|_| Error::Capacity)?;
                }
                return Ok(Outcome::Inspected);
            }
            Command::Remember(text) => {
                Operation::Store(Category::Semantic, unquote(text), b"explicit user request")
            }
            Command::Update(id, text) => Operation::Update(
                id,
                engine.inspect(backend,owner,session,id)?.category,
                unquote(text),
                b"explicit user correction",
            ),
            Command::ForgetId(id) => Operation::Delete(id),
            Command::ForgetQuery(query) => {
                let hits = engine.search(backend, owner, session, unquote(query), 16)?;
                if hits.len != 1 {
                    return Err(if hits.len == 0 {
                        Error::NotFound
                    } else {
                        Error::Invalid
                    });
                }
                Operation::Delete(hits.hits[0].id)
            }
        };
        let receipt = engine.execute(backend, owner, session, operation, now)?;
        if matches!(operation, Operation::Delete(_)) {
            write!(output, "Forgot memory {}.", receipt.id).map_err(|_| Error::Capacity)?;
            Ok(Outcome::Forgotten)
        } else {
            write!(output, "Saved memory {}.", receipt.id).map_err(|_| Error::Capacity)?;
            Ok(Outcome::Saved)
        }
    })();
    let outcome = match result {
        Ok(o) => o,
        Err(error) => {
            output.len = 0;
            let _=write!(output,"Memory operation failed: {:?}. No success is claimed. Use ai memory search/inspect to select one record for corrections or deletion.",error);
            Outcome::Failed
        }
    };
    Receipt {
        length: output.len,
        outcome,
    }
}
// ------------------------=
// FUNC: unquote
// DESC: Removes one optional enclosing pair of CLI quotes without evaluating escapes or commands.
// ------------------=
fn unquote(s: &[u8]) -> &[u8] {
    if s.len() >= 2 && s[0] == b'"' && s[s.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}
