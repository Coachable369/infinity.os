//! Versioned native application actions. Only the trusted session supplies authority.
//! Applications publish an agent-visible observation and an atomic set of legal choices.
pub const VERSION: u32 = 1;
pub const MAX_INSTANCES: usize = 8;
pub const MAX_CHOICES: usize = 128;
pub const STATE_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Missing, Denied, Capacity, Invalid, Stale, Busy, Expired, Approval, UnknownOutcome }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk { ReadOnly, Reversible, Sensitive }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Applied, Rejected, Cancelled, Unknown }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: u32,
    pub action: [u8; 48],
    pub label: [u8; 96],
    pub arguments: [i64; 4],
    pub risk: Risk,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub token: u64,
    pub instance: u64,
    pub revision: u64,
    pub choice: Choice,
    pub deadline: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub token: u64,
    pub instance: u64,
    pub revision: u64,
    pub outcome: Outcome,
}
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub revision: u64,
    pub agent_turn: bool,
    pub terminal: bool,
    pub state: [u8; STATE_BYTES],
    pub length: usize,
    pub choices: [Option<Choice>; MAX_CHOICES],
}
impl Snapshot {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an unpublished observation with no legal actions.
    // ------------------=
    pub const fn new() -> Self {
        Self { revision: 0, agent_turn: false, terminal: false, state: [0; STATE_BYTES], length: 0, choices: [None; MAX_CHOICES] }
    }
}
#[derive(Clone, Copy)]
struct Instance {
    id: u64,
    owner: [u8; 16],
    session: [u8; 16],
    snapshot: Snapshot,
    staging: Option<Snapshot>,
    request: Option<Request>,
    delivered: bool,
    receipt: Option<Receipt>,
    granted: bool,
}
pub struct Registry {
    instances: [Option<Instance>; MAX_INSTANCES],
    sequence: u64,
}
impl Registry {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes a bounded instance registry without ambient grants.
    // ------------------=
    pub const fn new() -> Self { Self { instances: [None; MAX_INSTANCES], sequence: 0 } }
    // ------------------------=
    // FUNC: next
    // DESC: Allocates a unique generation or request token without wrapping identities.
    // ------------------=
    fn next(&mut self) -> Result<u64, Error> {
        self.sequence = self.sequence.checked_add(1).ok_or(Error::Capacity)?;
        Ok(self.sequence)
    }
    // ------------------------=
    // FUNC: instance
    // DESC: Finds a live instance without granting another owner's access.
    // ------------------=
    fn instance(&mut self, id: u64, owner: [u8; 16], session: [u8; 16]) -> Result<&mut Instance, Error> {
        let entry = self.instances.iter_mut().flatten().find(|entry| entry.id == id).ok_or(Error::Missing)?;
        if entry.owner != owner || entry.session != session { return Err(Error::Denied); }
        Ok(entry)
    }
    // ------------------------=
    // FUNC: register
    // DESC: Binds capabilities to a new trusted launch identity; relaunch never reuses IDs.
    // ------------------=
    pub fn register(&mut self, owner: [u8; 16], session: [u8; 16]) -> Result<u64, Error> {
        if owner == [0; 16] || session == [0; 16] { return Err(Error::Denied); }
        let slot = self.instances.iter().position(Option::is_none).ok_or(Error::Capacity)?;
        let id = self.next()?;
        self.instances[slot] = Some(Instance { id, owner, session, snapshot: Snapshot::new(), staging: None,
            request: None, delivered: false, receipt: None, granted: false });
        Ok(id)
    }
    // ------------------------=
    // FUNC: grant
    // DESC: Sets explicit user permission and cancels undelivered actions when revoked.
    // ------------------=
    pub fn grant(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], allowed: bool) -> Result<(), Error> {
        let entry = self.instance(id, owner, session)?;
        entry.granted = allowed;
        if !allowed { Self::cancel_entry(entry); }
        Ok(())
    }
    // ------------------------=
    // FUNC: begin
    // DESC: Starts atomic publication of agent-visible state without exposing an incomplete choice set.
    // ------------------=
    pub fn begin(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], revision: u64,
        state: &[u8], agent_turn: bool, terminal: bool) -> Result<(), Error> {
        if state.is_empty() || state.len() > STATE_BYTES || core::str::from_utf8(state).is_err() { return Err(Error::Invalid); }
        let entry = self.instance(id, owner, session)?;
        if revision <= entry.snapshot.revision { return Err(Error::Stale); }
        let mut next = Snapshot::new();
        next.revision = revision; next.agent_turn = agent_turn; next.terminal = terminal;
        next.state[..state.len()].copy_from_slice(state); next.length = state.len();
        entry.staging = Some(next);
        Ok(())
    }
    // ------------------------=
    // FUNC: offer
    // DESC: Registers a legal typed action choice for the pending observation.
    // ------------------=
    pub fn offer(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], choice: Choice) -> Result<(), Error> {
        if choice.id == 0 || !identifier(&choice.action) || text(&choice.label).is_empty()
            || core::str::from_utf8(text(&choice.label)).is_err() { return Err(Error::Invalid); }
        let next = self.instance(id, owner, session)?.staging.as_mut().ok_or(Error::Invalid)?;
        if next.terminal || next.choices.iter().flatten().any(|item| item.id == choice.id) { return Err(Error::Invalid); }
        let slot = next.choices.iter_mut().find(|slot| slot.is_none()).ok_or(Error::Capacity)?;
        *slot = Some(choice);
        Ok(())
    }
    // ------------------------=
    // FUNC: commit
    // DESC: Atomically publishes the next observation and expires any undelivered stale proposal.
    // ------------------=
    pub fn commit(&mut self, id: u64, owner: [u8; 16], session: [u8; 16]) -> Result<(), Error> {
        let entry = self.instance(id, owner, session)?;
        entry.snapshot = entry.staging.take().ok_or(Error::Invalid)?;
        if entry.request.is_some() && !entry.delivered { Self::cancel_entry(entry); }
        Ok(())
    }
    // ------------------------=
    // FUNC: observe
    // DESC: Returns only the observation explicitly published for the authorized agent role.
    // ------------------=
    pub fn observe(&mut self, id: u64, owner: [u8; 16], session: [u8; 16]) -> Result<Snapshot, Error> {
        let entry = self.instance(id, owner, session)?;
        if !entry.granted { return Err(Error::Denied); }
        Ok(entry.snapshot)
    }
    // ------------------------=
    // FUNC: propose
    // DESC: Validates a model-selected legal choice against current state and explicit approval.
    // ------------------=
    pub fn propose(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], revision: u64,
        choice_id: u32, approved: bool, now: u64, deadline: u64) -> Result<Request, Error> {
        if deadline <= now || deadline - now > 30_000_000_000 { return Err(Error::Expired); }
        let token = self.next()?;
        let entry = self.instance(id, owner, session)?;
        if !entry.granted { return Err(Error::Denied); }
        if entry.request.is_some() { return Err(Error::Busy); }
        if entry.snapshot.revision != revision { return Err(Error::Stale); }
        if entry.snapshot.terminal || !entry.snapshot.agent_turn { return Err(Error::Denied); }
        let choice = *entry.snapshot.choices.iter().flatten().find(|item| item.id == choice_id).ok_or(Error::Invalid)?;
        if choice.risk == Risk::Sensitive && !approved { return Err(Error::Approval); }
        let request = Request { token, instance: id, revision, choice, deadline };
        entry.request = Some(request); entry.delivered = false; entry.receipt = None;
        Ok(request)
    }
    // ------------------------=
    // FUNC: poll
    // DESC: Delivers an action at most once and rechecks session, grant, state, and deadline.
    // ------------------=
    pub fn poll(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], now: u64) -> Result<Option<Request>, Error> {
        let entry = self.instance(id, owner, session)?;
        let Some(request) = entry.request else { return Ok(None); };
        if now >= request.deadline { Self::cancel_entry(entry); return Err(Error::Expired); }
        if entry.delivered { return Ok(None); }
        if !entry.granted || request.revision != entry.snapshot.revision { Self::cancel_entry(entry); return Err(Error::Stale); }
        entry.delivered = true;
        Ok(Some(request))
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Records an application result only for a delivered token and newly published state on success.
    // ------------------=
    pub fn finish(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], token: u64,
        outcome: Outcome, now: u64) -> Result<Receipt, Error> {
        let entry = self.instance(id, owner, session)?;
        let request = entry.request.ok_or(Error::Missing)?;
        if request.token != token || !entry.delivered { return Err(Error::Denied); }
        if now >= request.deadline { Self::cancel_entry(entry); return Err(Error::UnknownOutcome); }
        if outcome == Outcome::Applied && entry.snapshot.revision <= request.revision { return Err(Error::Stale); }
        let receipt = Receipt { token, instance: id, revision: entry.snapshot.revision, outcome };
        entry.request = None; entry.delivered = false; entry.receipt = Some(receipt);
        Ok(receipt)
    }
    // ------------------------=
    // FUNC: result
    // DESC: Observes the latest receipt, expiring unacknowledged operations without automatic retries.
    // ------------------=
    pub fn result(&mut self, id: u64, owner: [u8; 16], session: [u8; 16], now: u64) -> Result<Option<Receipt>, Error> {
        let entry = self.instance(id, owner, session)?;
        if entry.request.is_some_and(|request| now >= request.deadline) { Self::cancel_entry(entry); }
        Ok(entry.receipt)
    }
    // ------------------------=
    // FUNC: cancel_entry
    // DESC: Distinguishes cancellation before dispatch from an unknown outcome after delivery.
    // ------------------=
    fn cancel_entry(entry: &mut Instance) {
        if let Some(request) = entry.request.take() {
            entry.receipt = Some(Receipt { token: request.token, instance: entry.id, revision: entry.snapshot.revision,
                outcome: if entry.delivered { Outcome::Unknown } else { Outcome::Cancelled } });
        }
        entry.delivered = false;
    }
    // ------------------------=
    // FUNC: close
    // DESC: Removes one instance so its old actions cannot target a subsequent launch.
    // ------------------=
    pub fn close(&mut self, id: u64, owner: [u8; 16], session: [u8; 16]) -> Result<(), Error> {
        self.instance(id, owner, session)?;
        let slot = self.instances.iter_mut().find(|slot| slot.is_some_and(|entry| entry.id == id)).ok_or(Error::Missing)?;
        *slot = None;
        Ok(())
    }
}
// ------------------------=
// FUNC: text
// DESC: Reads the bounded UTF-8 payload of a zero-padded contract field.
// ------------------=
pub fn text(bytes: &[u8]) -> &[u8] { &bytes[..bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len())] }
// ------------------------=
// FUNC: identifier
// DESC: Validates a dotted application action identifier without executable syntax.
// ------------------=
fn identifier(bytes: &[u8]) -> bool {
    let value = text(bytes);
    !value.is_empty() && value[0].is_ascii_alphabetic()
        && value.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
}
