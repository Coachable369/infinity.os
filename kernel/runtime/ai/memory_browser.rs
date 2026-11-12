//! Bounded, authenticated application projection over the AIKit catalog.
use super::memory_api::{Backend, Engine, Operation};
use super::memory_search::{self, EmbeddingProvider};
use super::persistent_memory::{Category, Error, Owner};
use crate::runtime::console_execution::Bytes;
use core::fmt::Write;

// ------------------------=
// FUNC: category
// DESC: Validates the category supplied by the editor.
// ------------------=
fn category(value: u8) -> Result<Category, Error> {
    match value { 1 => Ok(Category::Episodic), 2 => Ok(Category::Semantic), 3 => Ok(Category::Procedural), _ => Err(Error::Invalid) }
}

// ------------------------=
// FUNC: request
// DESC: Returns one page of summaries or one full detail, and commits edits through the shared engine.
// ------------------=
pub fn request(engine: &mut Engine, backend: &mut impl Backend, owner: Owner, session: Owner,
    action: u64, number: u64, filter: u8, text: &[u8], now: u64, out: &mut Bytes<4096>) -> Result<(),Error> {
    let content = core::str::from_utf8(text).map_err(|_| Error::Invalid)?;
    match action {
        0..=2 => {
            let catalog = engine.catalog(backend, owner, session)?;
            let records = catalog.records(owner)?;
            let mut ids = [0; super::persistent_memory::MAX_RECORDS];
            let count = if text.is_empty() {
                for (id, record) in ids.iter_mut().zip(records) { *id = record.id; }
                records.len()
            } else {
                let results = if action == 2 {
                    let mut vector = [0.0; super::memory_embedding::DIM];
                    match super::memory_embedding::Potion.embed(text, &mut vector) {
                        Ok(_) => memory_search::semantic(catalog,owner,super::memory_embedding::MODEL,&vector,ids.len(),0.40)?,
                        Err(Error::Unavailable) => memory_search::lexical(catalog,owner,text,ids.len())?,
                        Err(error) => return Err(error),
                    }
                } else { memory_search::lexical(catalog,owner,text,ids.len())? };
                for (id, hit) in ids.iter_mut().zip(&results.hits[..results.len]) { *id = hit.id; }
                results.len
            };
            let mut matched = 0;
            for &id in &ids[..count] {
                let r = catalog.inspect(owner,id)?;
                if filter != 0 && r.category != category(filter)? { continue; }
                if matched / 6 == number {
                    write!(out,"{}\t{}\t",r.id,r.category as u8).map_err(|_|Error::Capacity)?;
                    for c in core::str::from_utf8(r.content()).map_err(|_|Error::Corrupt)?.chars().take(56) {
                        out.write_char(if c.is_control(){' '}else{c}).map_err(|_|Error::Capacity)?;
                    }
                    out.write(b"\n");
                }
                matched += 1;
            }
            writeln!(out,"#\t{}\t{}\t{}\t{}",matched,records.len(),records.iter().filter(|r|!r.embedding().is_empty()).count(),engine.storage_bytes).map_err(|_|Error::Capacity)?;
        }
        3 => {
            let r = engine.inspect(backend,owner,session,number)?;
            write!(out,"{}\t{}\t{}\t{}\t{}",r.id,r.category as u8,r.created_at,r.updated_at,r.embedding().len()).map_err(|_|Error::Capacity)?;
            for field in [r.model(),r.source(),r.content()] { out.write(&[0]); out.write(field); }
        }
        4..=6 => {
            let op = match action {
                4 => Operation::Store(category(filter)?,content.as_bytes(),b"Infinity Memory"),
                5 => Operation::Update(number,category(filter)?,content.as_bytes(),b"Infinity Memory"),
                _ => Operation::Delete(number),
            };
            let receipt = engine.execute(backend,owner,session,op,now)?;
            write!(out,"{}",receipt.id).map_err(|_|Error::Capacity)?;
        }
        _ => return Err(Error::Invalid),
    }
    if out.truncated() { Err(Error::Capacity) } else { Ok(()) }
}
