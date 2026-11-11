//! Shared AIKit memory API, used by Console, IntelligenceKit and DevKit under the same session authority.
use super::{
    memory_search::{self, Results},
    persistent_memory::{Category, Error, Memory, Owner, Record, MAX_BYTES},
};
pub trait Backend {
    // ------------------------=
    // FUNC: authorize
    // DESC: Revalidates the live authenticated identity session on every operation, including cache hits.
    // ------------------=
    fn authorize(&mut self, owner: Owner, session: Owner) -> bool;
    // ------------------------=
    // FUNC: load
    // DESC: Reads a committed native checkpoint or an explicitly empty first-use catalog.
    // ------------------=
    fn load(&mut self, owner: Owner, session: Owner, out: &mut [u8]) -> Result<usize, Error>;
    // ------------------------=
    // FUNC: save
    // DESC: Acknowledges a new catalog only after native durable commit succeeds.
    // ------------------=
    fn save(&mut self, owner: Owner, session: Owner, memory: &Memory) -> Result<(), Error>;
    // ------------------------=
    // FUNC: now
    // DESC: Supplies monotonic latency samples without creating another scheduler.
    // ------------------=
    fn now(&self) -> u64;
}
pub struct Engine {
    memory: Memory,
    loaded: bool,
    owner: Owner,
    pub loads: u64,
    pub errors: u64,
    pub retrieval_ns: u64,
    pub max_retrieval_ns: u64,
    pub storage_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum Operation<'a> {
    Status,
    Search(&'a [u8]),
    Inspect(u64),
    Store(Category, &'a [u8], &'a [u8]),
    Update(u64, Category, &'a [u8], &'a [u8]),
    Delete(u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub id: u64,
    pub count: usize,
    pub embedded: usize,
    pub written: bool,
}
impl Engine {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates one bounded owner cache independent of the selected conversational model.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            memory: Memory::new([0; 16]),
            loaded: false,
            owner: [0; 16],
            loads: 0,
            errors: 0,
            retrieval_ns: 0,
            max_retrieval_ns: 0,
            storage_bytes: 0,
        }
    }
    // ------------------------=
    // FUNC: ensure
    // DESC: Revalidates identity before cache access, loading records only when the owner changes or a commit becomes uncertain.
    // ------------------=
    fn ensure(
        &mut self,
        backend: &mut impl Backend,
        owner: Owner,
        session: Owner,
    ) -> Result<(), Error> {
        if !backend.authorize(owner, session) {
            return Err(Error::Unauthorized);
        }
        if self.loaded && self.owner == owner {
            return Ok(());
        }
        self.loaded = false;
        self.memory = Memory::new([0; 16]);
        self.owner = owner;
        let mut bytes = [0; MAX_BYTES];
        let result = backend.load(owner, session, &mut bytes).and_then(|n| {
            if n > bytes.len() {
                return Err(Error::Corrupt);
            }
            self.storage_bytes = n;
            Memory::decode(&bytes[..n], owner)
        });
        match result {
            Ok(m) => {
                self.memory = m;
                self.loaded = true;
                self.loads += 1;
                Ok(())
            }
            Err(e) => {
                self.errors += 1;
                Err(e)
            }
        }
    }
    // ------------------------=
    // FUNC: search
    // DESC: Retrieves from the loaded index with live owner checks and measured latency; no disk work occurs on a cache hit.
    // ------------------=
    pub fn search(
        &mut self,
        backend: &mut impl Backend,
        owner: Owner,
        session: Owner,
        query: &[u8],
        limit: usize,
    ) -> Result<Results, Error> {
        let start = backend.now();
        self.ensure(backend, owner, session)?;
        let result = memory_search::lexical(&self.memory, owner, query, limit);
        self.retrieval_ns = backend.now().saturating_sub(start);
        self.max_retrieval_ns = self.max_retrieval_ns.max(self.retrieval_ns);
        result
    }
    // ------------------------=
    // FUNC: context
    // DESC: Supplies a single bounded untrusted-data block while allowing inference to proceed if native retrieval fails.
    // ------------------=
    pub fn context(
        &mut self,
        backend: &mut impl Backend,
        owner: Owner,
        session: Owner,
        query: &[u8],
        budget: usize,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let start = backend.now();
        self.ensure(backend, owner, session)?;
        let result = memory_search::context(&self.memory, owner, query, 3, budget, out);
        self.retrieval_ns = backend.now().saturating_sub(start);
        self.max_retrieval_ns = self.max_retrieval_ns.max(self.retrieval_ns);
        result
    }
    // ------------------------=
    // FUNC: inspect
    // DESC: Copies one authorized record for native clients without exposing another user's cached catalog.
    // ------------------=
    pub fn inspect(
        &mut self,
        backend: &mut impl Backend,
        owner: Owner,
        session: Owner,
        id: u64,
    ) -> Result<Record, Error> {
        self.ensure(backend, owner, session)?;
        self.memory.inspect(owner, id).copied()
    }
    // ------------------------=
    // FUNC: execute
    // DESC: Commits deterministic memory operations before reporting success; failed commits invalidate the cache for recovery.
    // ------------------=
    pub fn execute(
        &mut self,
        backend: &mut impl Backend,
        owner: Owner,
        session: Owner,
        operation: Operation<'_>,
        now: u64,
    ) -> Result<Receipt, Error> {
        self.ensure(backend, owner, session)?;
        let mut receipt = Receipt {
            id: 0,
            count: self.memory.records(owner)?.len(),
            embedded: self
                .memory
                .records(owner)?
                .iter()
                .filter(|r| !r.embedding().is_empty())
                .count(),
            written: false,
        };
        match operation {
            Operation::Status => return Ok(receipt),
            Operation::Search(query) => {
                let hits = self.search(backend, owner, session, query, 16)?;
                receipt.count = hits.len;
                receipt.id = hits.hits[0].id;
                return Ok(receipt);
            }
            Operation::Inspect(id) => {
                self.memory.inspect(owner, id)?;
                receipt.id = id;
                return Ok(receipt);
            }
            _ => {}
        }
        let mut next = self.memory;
        match operation {
            Operation::Store(category, text, source) => {
                if sensitive(text) {
                    return Err(Error::Unauthorized);
                }
                receipt.id = next.store(owner, Record::new(owner, category, text, source, now)?)?;
            }
            Operation::Update(id, category, text, source) => {
                if sensitive(text) {
                    return Err(Error::Unauthorized);
                }
                next.update(owner, id, Record::new(owner, category, text, source, now)?)?;
                receipt.id = id;
            }
            Operation::Delete(id) => {
                next.delete(owner, id)?;
                receipt.id = id;
            }
            _ => return Err(Error::Invalid),
        }
        let mut bytes = [0; MAX_BYTES];
        let n = next.encode(&mut bytes)?;
        let mut previous = [0; MAX_BYTES];
        let old = self.memory.encode(&mut previous)?;
        if n == old && bytes[..n] == previous[..old] {
            return Ok(receipt);
        }
        if let Err(e) = backend.save(owner, session, &next) {
            self.loaded = false;
            self.errors += 1;
            return Err(e);
        }
        self.memory = next;
        self.storage_bytes = n;
        receipt.count = self.memory.records(owner)?.len();
        receipt.written = true;
        Ok(receipt)
    }
}
// ------------------------=
// FUNC: sensitive
// DESC: Refuses common credential-bearing memory requests; automatic candidates require separate explicit acceptance.
// ------------------=
pub fn sensitive(text: &[u8]) -> bool {
    [
        b"password".as_slice(),
        b"private key",
        b"api key",
        b"api_key",
        b"access token",
        b"secret key",
        b"seed phrase",
        b"credit card",
        b"social security",
        b"-----begin",
    ]
    .iter()
    .any(|needle| {
        text.windows(needle.len())
            .any(|s| s.eq_ignore_ascii_case(needle))
    })
}

pub struct Native;
impl Backend for Native {
    // ------------------------=
    // FUNC: authorize
    // DESC: Uses IdentitySystem rather than supplied prompt content to authorize native memory access.
    // ------------------=
    fn authorize(&mut self, owner: Owner, session: Owner) -> bool {
        #[cfg(target_os = "none")]
        {
            crate::storage::aikit_memory_authorized(owner, session)
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (owner, session);
            false
        }
    }
    // ------------------------=
    // FUNC: load
    // DESC: Loads the native checkpoint through the existing storage adapter.
    // ------------------=
    fn load(&mut self, owner: Owner, session: Owner, out: &mut [u8]) -> Result<usize, Error> {
        #[cfg(target_os = "none")]
        {
            crate::storage::aikit_memory_checkpoint(owner, session, None, out)
                .map_err(|_| Error::Storage)
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (owner, session, out);
            Err(Error::Unavailable)
        }
    }
    // ------------------------=
    // FUNC: save
    // DESC: Performs an owner-checked durable transaction and reports all commit failures.
    // ------------------=
    fn save(&mut self, owner: Owner, session: Owner, memory: &Memory) -> Result<(), Error> {
        #[cfg(target_os = "none")]
        {
            let mut bytes = [0; MAX_BYTES];
            crate::storage::aikit_memory_checkpoint(owner, session, Some(memory), &mut bytes)
                .map(|_| ())
                .map_err(|_| Error::Storage)
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = (owner, session, memory);
            Err(Error::Unavailable)
        }
    }
    // ------------------------=
    // FUNC: now
    // DESC: Reads the existing runtime performance clock for measured retrieval latency.
    // ------------------=
    fn now(&self) -> u64 {
        crate::ui::performance::monotonic_ns().unwrap_or(0)
    }
}

/// Only trusted native callers may attest provenance. Model proposals never attest themselves.
pub enum Evidence<'a> {
    UserPreference,
    Development(&'a super::intelligence::software::Execution),
    Generated,
}
// ------------------------=
// FUNC: candidate
// DESC: Accepts opted-in explicit preferences or narrowly verified native development milestones; rejects generated and secret-bearing candidates.
// ------------------=
pub fn candidate(owner:Owner, text:&[u8], evidence:Evidence<'_>, consent:bool, now:u64)->Result<Record,Error> {
    if !consent || owner==[0;16] || sensitive(text) {return Err(Error::Unauthorized);}
    let (category,source,prefix):(_, &[u8], &[u8])=match evidence {
        Evidence::UserPreference if text.starts_with(b"I prefer ") || text.starts_with(b"I always prefer ") =>
            (Category::Semantic,b"user preference",b""),
        Evidence::Development(e) if e.owner==owner && e.source!=[0;16] && e.artifact!=[0;16]
            && e.context!=0 && e.interaction_active && e.runtime_failure().is_none() =>
            (Category::Episodic,b"native DevKit receipts",b"Compiled, launched and published controls for requested task: "),
        _=>return Err(Error::Unauthorized),
    };
    let mut content=[0;super::persistent_memory::MAX_CONTENT];
    if prefix.len()+text.len()>content.len(){return Err(Error::Capacity);}
    content[..prefix.len()].copy_from_slice(prefix);content[prefix.len()..prefix.len()+text.len()].copy_from_slice(text);
    Record::new(owner,category,&content[..prefix.len()+text.len()],source,now)
}
