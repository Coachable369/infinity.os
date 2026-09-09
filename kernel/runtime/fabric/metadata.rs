//! Fixed-membership, single-writer metadata quorum primitives. No runtime or
//! namespace integration is implied. All durable and signing operations are
//! supplied explicitly; successful discovery confers no read or write authority.
use crate::runtime::{crypto::NodeCrypto, node::types::NodeId};
use sha2::{Digest, Sha256};
pub const RECORD_BYTES: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Denied,
    Signature,
    Stale,
    Conflict,
    Quorum,
    Persistence,
    ResourceLimit,
    Deleted,
}
#[derive(Clone, Copy)]
pub struct Group {
    pub epoch: u64,
    pub owner: NodeId,
    pub members: [NodeId; 3],
    pub keys: [[u8; 32]; 3],
}
impl Group {
    // ------------------------=
    // FUNC: validate
    // DESC: Requires three distinct explicitly supplied identities and the single writer among them.
    // ------------------=
    pub fn validate(&self) -> Result<(), Error> {
        if self.epoch == 0 || !self.members.contains(&self.owner) {
            return Err(Error::Invalid);
        }
        for i in 0..3 {
            let mut h = Sha256::new();
            h.update(b"InfinityOS NodeId v1");
            h.update(self.keys[i]);
            if self.members[i] != NodeId(h.finalize().into())
                || self.members[..i].contains(&self.members[i])
            {
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: digest
    // DESC: Binds membership and owner into every signed metadata generation.
    // ------------------=
    pub fn digest(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"InfinityOS Pool Metadata Group v1");
        h.update(self.epoch.to_le_bytes());
        h.update(self.owner.0);
        for id in self.members {
            h.update(id.0)
        }
        h.finalize().into()
    }
    // ------------------------=
    // FUNC: verify
    // DESC: Verifies a signature only against an explicitly configured member key.
    // ------------------=
    fn verify(&self, member: u8, message: &[u8], signature: &[u8; 64]) -> Result<(), Error> {
        let key = self.keys.get(member as usize).ok_or(Error::Denied)?;
        NodeCrypto::verify(key, message, signature).map_err(|_| Error::Signature)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub group: [u8; 32],
    pub object: [u8; 16],
    pub generation: u64,
    pub version: u64,
    pub previous: [u8; 32],
    pub manifest: [u8; 32],
    pub namespace: [u8; 32],
    pub policy: [u8; 32],
    pub revocation: u64,
    pub deleted: bool,
}
impl Record {
    // ------------------------=
    // FUNC: encode
    // DESC: Produces a canonical bounded record referencing immutable manifest, namespace and policy objects.
    // ------------------=
    pub fn encode(self) -> [u8; RECORD_BYTES] {
        let mut b = [0; RECORD_BYTES];
        b[..8].copy_from_slice(b"INFPMQ01");
        b[8..40].copy_from_slice(&self.group);
        b[40..56].copy_from_slice(&self.object);
        b[56..64].copy_from_slice(&self.generation.to_le_bytes());
        b[64..72].copy_from_slice(&self.version.to_le_bytes());
        for (at, v) in [
            (72, self.previous),
            (104, self.manifest),
            (136, self.namespace),
            (168, self.policy),
        ] {
            b[at..at + 32].copy_from_slice(&v)
        }
        b[200..208].copy_from_slice(&self.revocation.to_le_bytes());
        b[208] = u8::from(self.deleted);
        b
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects noncanonical flags, reserved bytes and unbounded or empty identities.
    // ------------------=
    pub fn decode(b: &[u8]) -> Result<Self, Error> {
        if b.len() != RECORD_BYTES
            || &b[..8] != b"INFPMQ01"
            || b[208] > 1
            || b[209..].iter().any(|v| *v != 0)
        {
            return Err(Error::Invalid);
        }
        let r = Self {
            group: b[8..40].try_into().unwrap(),
            object: b[40..56].try_into().unwrap(),
            generation: u64::from_le_bytes(b[56..64].try_into().unwrap()),
            version: u64::from_le_bytes(b[64..72].try_into().unwrap()),
            previous: b[72..104].try_into().unwrap(),
            manifest: b[104..136].try_into().unwrap(),
            namespace: b[136..168].try_into().unwrap(),
            policy: b[168..200].try_into().unwrap(),
            revocation: u64::from_le_bytes(b[200..208].try_into().unwrap()),
            deleted: b[208] != 0,
        };
        if r.object == [0; 16]
            || r.generation == 0
            || r.version == 0
            || r.revocation == 0
            || r.manifest == [0; 32]
            || r.namespace == [0; 32]
            || r.policy == [0; 32]
        {
            return Err(Error::Invalid);
        }
        Ok(r)
    }
    // ------------------------=
    // FUNC: digest
    // DESC: Hashes the complete canonical generation, including tombstone and revocation fence.
    // ------------------=
    pub fn digest(self) -> [u8; 32] {
        Sha256::digest(self.encode()).into()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedRecord {
    pub record: Record,
    pub signature: [u8; 64],
}
impl SignedRecord {
    // ------------------------=
    // FUNC: validate
    // DESC: Accepts metadata only from the fixed group's owner with canonical bytes and matching group epoch.
    // ------------------=
    pub fn validate(&self, g: &Group) -> Result<(), Error> {
        g.validate()?;
        Record::decode(&self.record.encode())?;
        if self.record.group != g.digest() {
            return Err(Error::Denied);
        }
        let owner = g
            .members
            .iter()
            .position(|n| *n == g.owner)
            .ok_or(Error::Denied)?;
        g.verify(owner as u8, &self.record.encode(), &self.signature)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub member: u8,
    pub digest: [u8; 32],
    pub published: bool,
    pub signature: [u8; 64],
}
impl Receipt {
    // ------------------------=
    // FUNC: transcript
    // DESC: Domain-separates durable staging acknowledgement from durable publication acknowledgement.
    // ------------------=
    pub fn transcript(&self) -> [u8; 48] {
        let mut b = [0; 48];
        b[..8].copy_from_slice(b"INFPMACK");
        b[8] = self.member;
        b[9] = u8::from(self.published);
        b[16..].copy_from_slice(&self.digest);
        b
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects wrong membership, phase, digest and forged acknowledgements.
    // ------------------=
    fn validate(&self, g: &Group, digest: [u8; 32], published: bool) -> Result<(), Error> {
        if self.digest != digest || self.published != published {
            return Err(Error::Conflict);
        }
        g.verify(self.member, &self.transcript(), &self.signature)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub value: SignedRecord,
    pub prepared: [Receipt; 2],
}
impl Certificate {
    // ------------------------=
    // FUNC: validate
    // DESC: Requires two distinct durable staging receipts for the same owner-signed record.
    // ------------------=
    pub fn validate(&self, g: &Group) -> Result<(), Error> {
        self.value.validate(g)?;
        if self.prepared[0].member == self.prepared[1].member {
            return Err(Error::Quorum);
        }
        for p in self.prepared {
            p.validate(g, self.value.record.digest(), false)?
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Default)]
pub struct Replica {
    pub staged: Option<SignedRecord>,
    pub committed: Option<Certificate>,
}
/// Writer completion gate: staging quorum alone must never acknowledge a write.
pub struct Publication {
    certificate: Certificate,
    published: u8,
    group: [u8;32],
}
impl Publication {
    // ------------------------=
    // FUNC: new
    // DESC: Starts a bounded W2 publication gate after validating the staging certificate.
    // ------------------=
    pub fn new(g: &Group, certificate: Certificate) -> Result<Self, Error> {
        certificate.validate(g)?;
        Ok(Self {
            certificate,
            published: 0,
            group: g.digest(),
        })
    }
    // ------------------------=
    // FUNC: acknowledge
    // DESC: Counts only distinct signed durable publication receipts for the exact certified generation.
    // ------------------=
    pub fn acknowledge(&mut self, g: &Group, receipt: Receipt) -> Result<(), Error> {
        if g.digest()!=self.group {return Err(Error::Denied)}
        receipt.validate(g, self.certificate.value.record.digest(), true)?;
        let bit = 1 << receipt.member;
        if self.published & bit != 0 {
            return Err(Error::Conflict);
        }
        self.published |= bit;
        Ok(())
    }
    // ------------------------=
    // FUNC: committed
    // DESC: Exposes writer success only after two replicas durably publish, preserving read-quorum intersection.
    // ------------------=
    pub fn committed(&self) -> Result<Certificate, Error> {
        if self.published.count_ones() < 2 {
            return Err(Error::Quorum);
        }
        Ok(self.certificate)
    }
}
impl Replica {
    // ------------------------=
    // FUNC: prepare
    // DESC: Stages one linked successor atomically before allowing a signed acknowledgement; persistence failure cannot advance state.
    // ------------------=
    pub fn prepare(
        &mut self,
        g: &Group,
        value: SignedRecord,
        persist: impl FnOnce(Self) -> Result<(), Error>,
    ) -> Result<(), Error> {
        value.validate(g)?;
        let previous = self.staged.or(self.committed.map(|c| c.value));
        if let Some(p) = previous {
            if p == value {
                return Ok(());
            }
            if value.record.object != p.record.object
                || value.record.generation <= p.record.generation
            {
                return Err(Error::Stale);
            }
            if value.record.generation
                != p.record.generation.checked_add(1).ok_or(Error::Invalid)?
                || value.record.previous != p.record.digest()
                || value.record.revocation < p.record.revocation
                || value.record.version < p.record.version
                || p.record.deleted
            {
                return Err(Error::Conflict);
            }
        } else if value.record.generation != 1 || value.record.previous != [0; 32] {
            return Err(Error::Stale);
        }
        let next = Self {
            staged: Some(value),
            ..*self
        };
        persist(next)?;
        *self = next;
        Ok(())
    }
    // ------------------------=
    // FUNC: publish
    // DESC: Publishes a quorum-certified snapshot or read write-back without granting writer authority; stale or conflicting rollback is rejected.
    // ------------------=
    pub fn publish(
        &mut self,
        g: &Group,
        cert: Certificate,
        persist: impl FnOnce(Self) -> Result<(), Error>,
    ) -> Result<(), Error> {
        cert.validate(g)?;
        if let Some(old) = self.committed {
            if old.value == cert.value {
                return Ok(());
            }
            if old.value.record.object != cert.value.record.object
                || cert.value.record.generation <= old.value.record.generation
                || cert.value.record.revocation < old.value.record.revocation
                || old.value.record.deleted
            {
                return Err(Error::Stale);
            }
        }
        if let Some(staged) = self.staged {
            if staged.record.generation == cert.value.record.generation && staged != cert.value {
                return Err(Error::Conflict);
            }
        }
        let next = Self {
            committed: Some(cert),
            staged: self
                .staged
                .filter(|s| s.record.generation > cert.value.record.generation),
        };
        persist(next)?;
        *self = next;
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct ReadRound {
    object: [u8; 16],
    group: Option<[u8; 32]>,
    seen: u8,
    best: Option<Certificate>,
    written: u8,
}
/// Explicit owner-signed read delegation. Local execution-context authentication
/// must supply `reader` and `principal`; discovery or metadata receipt is not a grant.
#[derive(Clone, Copy)]
pub struct ReaderGrant {
    pub group: [u8; 32],
    pub object: [u8; 16],
    pub reader: NodeId,
    pub principal: [u8; 16],
    pub policy: [u8; 32],
    pub revocation: u64,
    pub expires: u64,
    pub signature: [u8; 64],
}
impl ReaderGrant {
    // ------------------------=
    // FUNC: transcript
    // DESC: Binds read-only authority to exact owner epoch, object, principal, node, policy revision and expiry.
    // ------------------=
    pub fn transcript(&self) -> [u8; 160] {
        let mut b = [0; 160];
        b[..8].copy_from_slice(b"INFPMR01");
        b[8..40].copy_from_slice(&self.group);
        b[40..56].copy_from_slice(&self.object);
        b[56..88].copy_from_slice(&self.reader.0);
        b[88..104].copy_from_slice(&self.principal);
        b[104..136].copy_from_slice(&self.policy);
        b[136..144].copy_from_slice(&self.revocation.to_le_bytes());
        b[144..152].copy_from_slice(&self.expires.to_le_bytes());
        b
    }
    // ------------------------=
    // FUNC: authorize
    // DESC: Validates owner signature and exact caller against fresh quorum metadata without conferring write rights.
    // ------------------=
    fn authorize(
        &self,
        g: &Group,
        r: Record,
        reader: NodeId,
        principal: [u8; 16],
        now: u64,
    ) -> Result<(), Error> {
        if self.group != g.digest()
            || self.group != r.group
            || self.object != r.object
            || self.reader != reader
            || self.principal != principal
            || principal == [0; 16]
            || self.policy != r.policy
            || self.revocation != r.revocation
            || now >= self.expires
        {
            return Err(Error::Denied);
        }
        let owner = g
            .members
            .iter()
            .position(|n| *n == g.owner)
            .ok_or(Error::Denied)?;
        g.verify(owner as u8, &self.transcript(), &self.signature)
    }
}
impl ReadRound {
    // ------------------------=
    // FUNC: new
    // DESC: Starts one bounded round; caller must authenticate each observation through the existing native session and exact metadata-read grant.
    // ------------------=
    pub fn new(object: [u8; 16]) -> Self {
        Self {
            object,
            group: None,
            seen: 0,
            best: None,
            written: 0,
        }
    }
    // ------------------------=
    // FUNC: observe_authenticated
    // DESC: Consumes a distinct member response bound by the caller to THIS fresh correlated read round; cached or replayed authenticated replies must never be admitted by the future native IOP hook.
    // ------------------=
    pub fn observe_authenticated(
        &mut self,
        g: &Group,
        member: NodeId,
        value: Option<Certificate>,
    ) -> Result<(), Error> {
        g.validate()?;
        if self.group.is_some_and(|digest| digest != g.digest()) {
            return Err(Error::Denied);
        }
        let i = g
            .members
            .iter()
            .position(|n| *n == member)
            .ok_or(Error::Denied)?;
        if self.seen & (1 << i) != 0 {
            return Err(Error::Conflict);
        }
        if let Some(c) = value {
            c.validate(g)?;
            if c.value.record.object != self.object {
                return Err(Error::Denied);
            }
            if let Some(b) = self.best {
                if b.value.record.generation == c.value.record.generation && b.value != c.value {
                    return Err(Error::Conflict);
                }
            }
            if self
                .best
                .is_none_or(|b| c.value.record.generation > b.value.record.generation)
            {
                self.best = Some(c);
                self.written = 0;
            }
        }
        self.group = Some(g.digest());
        self.seen |= 1 << i;
        Ok(())
    }
    // ------------------------=
    // FUNC: selected
    // DESC: Requires an R2 quorum before exposing a candidate for write-back, never content access.
    // ------------------=
    pub fn selected(&self) -> Result<Certificate, Error> {
        if self.seen.count_ones() < 2 {
            return Err(Error::Quorum);
        }
        self.best.ok_or(Error::Quorum)
    }
    // ------------------------=
    // FUNC: acknowledge_writeback
    // DESC: Requires distinct signed publication receipts; a partial read cannot make an uncertified value available.
    // ------------------=
    pub fn acknowledge_writeback(&mut self, g: &Group, receipt: Receipt) -> Result<(), Error> {
        g.validate()?;
        if self.group != Some(g.digest()) {
            return Err(Error::Denied);
        }
        let c = self.selected()?;
        receipt.validate(g, c.value.record.digest(), true)?;
        let bit = 1 << receipt.member;
        if self.written & bit != 0 {
            return Err(Error::Conflict);
        }
        self.written |= bit;
        Ok(())
    }
    // ------------------------=
    // FUNC: readable
    // DESC: Releases the current certified descriptor only after W2 write-back; tombstones and stale read-policy revisions fail closed.
    // ------------------=
    pub fn readable(
        &self,
        g: &Group,
        grant: &ReaderGrant,
        reader: NodeId,
        principal: [u8; 16],
        now: u64,
    ) -> Result<Record, Error> {
        g.validate()?;
        if self.group != Some(g.digest()) {
            return Err(Error::Denied);
        }
        let r = self.selected()?.value.record;
        if self.written.count_ones() < 2 {
            return Err(Error::Quorum);
        }
        if r.deleted {
            return Err(Error::Deleted);
        }
        grant.authorize(g, r, reader, principal, now)?;
        Ok(r)
    }
    // ------------------------=
    // FUNC: confirmed
    // DESC: Returns only a freshly observed and durably written-back quorum certificate for transaction recovery; it conveys no content read authority.
    // ------------------=
    pub fn confirmed(&self,g:&Group)->Result<Certificate,Error>{g.validate()?;if self.group!=Some(g.digest()){return Err(Error::Denied)}if self.written.count_ones()<2{return Err(Error::Quorum)}self.selected()}
}
#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;
