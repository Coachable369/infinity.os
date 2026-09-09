//! Placement-only authority: a repair never changes the owner's immutable
//! namespace, content, policy, or version. Callers must obtain the anchor and
//! overlay head from a fresh quorum before proposing or exposing a repair.
use super::{
    manifest::{Manifest, MANIFEST_BYTES},
    metadata::{Certificate, Error, Group, Receipt},
};
use crate::runtime::{crypto::NodeCrypto, node::types::NodeId};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairGrant {
    pub group: [u8; 32],
    pub anchor: [u8; 32],
    pub writer: NodeId,
    pub destinations: [NodeId; 4],
    pub expires: u64,
    pub signature: [u8; 64],
}
impl RepairGrant {
    // ------------------------=
    // FUNC: transcript
    // DESC: Binds one explicitly approved repair writer and destination set to an immutable owner generation and deadline.
    // ------------------=
    pub fn transcript(&self) -> [u8; 256] {
        let mut b = [0; 256];
        b[..8].copy_from_slice(b"INFPRG01");
        b[8..40].copy_from_slice(&self.group);
        b[40..72].copy_from_slice(&self.anchor);
        b[72..104].copy_from_slice(&self.writer.0);
        b[104..112].copy_from_slice(&self.expires.to_le_bytes());
        for (i, n) in self.destinations.iter().enumerate() {
            b[112 + i * 32..144 + i * 32].copy_from_slice(&n.0);
        }
        b
    }
    // ------------------------=
    // FUNC: digest
    // DESC: Gives the exact delegated authority a domain-separated identity for repair signatures.
    // ------------------=
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.transcript()).into()
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects expired, substituted, unsigned, nonmember or revoked-generation repair authority.
    // ------------------=
    pub fn validate(&self, g: &Group, anchor: &Certificate, now: u64) -> Result<(), Error> {
        anchor.validate(g)?;
        self.validate_anchor(g, &anchor.value, now)
    }
    // ------------------------=
    // FUNC: validate_anchor
    // DESC: Validates owner-issued delegation while its signed anchor is staged, without fabricating a publication certificate.
    // ------------------=
    pub fn validate_anchor(
        &self,
        g: &Group,
        anchor: &super::metadata::SignedRecord,
        now: u64,
    ) -> Result<(), Error> {
        anchor.validate(g)?;
        if anchor.record.deleted {
            return Err(Error::Deleted);
        }
        if self.group != g.digest()
            || self.anchor != anchor.record.digest()
            || now >= self.expires
            || !g.members.contains(&self.writer)
            || self.writer == g.owner
        {
            return Err(Error::Denied);
        }
        let mut count = 0;
        for (i, n) in self.destinations.iter().enumerate() {
            if n.0 != [0; 32] {
                if self.destinations[..i].contains(n) {
                    return Err(Error::Invalid);
                }
                count += 1;
            }
        }
        if count == 0 {
            return Err(Error::Invalid);
        }
        let owner = g
            .members
            .iter()
            .position(|n| *n == g.owner)
            .ok_or(Error::Denied)?;
        NodeCrypto::verify(&g.keys[owner], &self.transcript(), &self.signature)
            .map_err(|_| Error::Signature)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects noncanonical standalone grant bytes before callers bind them to an owner-signed anchor.
    // ------------------=
    pub fn decode(b: &[u8]) -> Result<Self, Error> {
        if b.len() != 320 {
            return Err(Error::Invalid);
        }
        let grant = Self {
            group: b[8..40].try_into().unwrap(),
            anchor: b[40..72].try_into().unwrap(),
            writer: NodeId(b[72..104].try_into().unwrap()),
            expires: u64::from_le_bytes(b[104..112].try_into().unwrap()),
            destinations: core::array::from_fn(|i| {
                NodeId(b[112 + i * 32..144 + i * 32].try_into().unwrap())
            }),
            signature: b[256..320].try_into().unwrap(),
        };
        if grant.transcript() != b[..256] {
            return Err(Error::Invalid);
        }
        Ok(grant)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairRecord {
    pub anchor: [u8; 32],
    pub grant: [u8; 32],
    pub sequence: u64,
    pub previous: [u8; 32],
    pub manifest: [u8; 32],
    pub signature: [u8; 64],
}
impl RepairRecord {
    // ------------------------=
    // FUNC: transcript
    // DESC: Canonically binds the repair chain and effective placement manifest without representing a content write.
    // ------------------=
    pub fn transcript(&self) -> [u8; 160] {
        let mut b = [0; 160];
        b[..8].copy_from_slice(b"INFPRH01");
        b[8..40].copy_from_slice(&self.anchor);
        b[40..72].copy_from_slice(&self.grant);
        b[72..80].copy_from_slice(&self.sequence.to_le_bytes());
        b[80..112].copy_from_slice(&self.previous);
        b[112..144].copy_from_slice(&self.manifest);
        b
    }
    // ------------------------=
    // FUNC: digest
    // DESC: Produces the exact repair head named by durable quorum receipts and successor fences.
    // ------------------=
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.transcript()).into()
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Checks delegated signature and exact immutable owner data; only bounded placement records can differ.
    // ------------------=
    pub fn validate(
        &self,
        g: &Group,
        anchor: &Certificate,
        grant: &RepairGrant,
        base: &Manifest,
        next: &Manifest,
        now: u64,
    ) -> Result<(), Error> {
        grant.validate(g, anchor, now)?;
        base.validate().map_err(|_| Error::Invalid)?;
        next.validate().map_err(|_| Error::Invalid)?;
        let mut bytes = [0; MANIFEST_BYTES];
        base.encode(&mut bytes).map_err(|_| Error::Invalid)?;
        let base_hash: [u8; 32] = Sha256::digest(bytes).into();
        if base_hash != anchor.value.record.manifest
            || base.object != anchor.value.record.object
            || base.version != anchor.value.record.version
            || base.authority != g.owner
            || self.anchor != grant.anchor
            || self.grant != grant.digest()
            || self.sequence == 0
        {
            return Err(Error::Denied);
        }
        if next.object != base.object
            || next.version != base.version
            || next.length != base.length
            || next.hash != base.hash
            || next.policy != base.policy
            || next.minimum_available != base.minimum_available
            || next.authority != base.authority
            || next.authority_generation != base.authority_generation
            || next.chunks != base.chunks
            || next.healing.is_some()
            || Some(next.generation) != base.generation.checked_add(self.sequence)
        {
            return Err(Error::Denied);
        }
        for p in next.placements.iter().flatten() {
            if !base.placements.contains(&Some(*p))
                && (!grant.destinations.contains(&p.node)
                    || p.version != base.version
                    || p.hash != base.hash)
            {
                return Err(Error::Denied);
            }
        }
        next.encode(&mut bytes).map_err(|_| Error::Invalid)?;
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        if hash != self.manifest {
            return Err(Error::Conflict);
        }
        let writer = g
            .members
            .iter()
            .position(|n| *n == grant.writer)
            .ok_or(Error::Denied)?;
        NodeCrypto::verify(&g.keys[writer], &self.transcript(), &self.signature)
            .map_err(|_| Error::Signature)
    }
    // ------------------------=
    // FUNC: successor
    // DESC: Rejects rollback, skipped heads, changed authority and concurrent repair forks before persistence.
    // ------------------=
    pub fn successor(&self, previous: Option<Self>) -> Result<(), Error> {
        match previous {
            Some(old) if old == *self => Ok(()),
            Some(old)
                if self.anchor == old.anchor
                    && self.grant == old.grant
                    && Some(self.sequence) == old.sequence.checked_add(1)
                    && self.previous == old.digest() =>
            {
                Ok(())
            }
            None if self.sequence == 1 && self.previous == [0; 32] => Ok(()),
            _ => Err(Error::Stale),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairCertificate {
    pub value: RepairRecord,
    pub prepared: [Receipt; 2],
}
pub const REPAIR_BUNDLE_BYTES: usize = 800 + MANIFEST_BYTES + 224;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairAvailability {
    pub repair: [u8; 32],
    pub node: NodeId,
    pub resource: [u8; 16],
    pub device: [u8; 16],
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
}
impl RepairAvailability {
    // ------------------------=
    // FUNC: transcript
    // DESC: Names the exact persisted physical copy and proposed repair head acknowledged by its actual recipient.
    // ------------------=
    pub fn transcript(&self) -> [u8; 128] {
        let mut b = [0; 128];
        b[..8].copy_from_slice(b"INFPRAV1");
        b[8..40].copy_from_slice(&self.repair);
        b[40..72].copy_from_slice(&self.node.0);
        b[72..88].copy_from_slice(&self.resource);
        b[88..104].copy_from_slice(&self.device);
        b
    }
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects a writer-fabricated availability claim or a receipt from another destination, device, or repair generation.
    // ------------------=
    pub fn validate(&self, r: &RepairRecord, p: &super::manifest::Placement) -> Result<(), Error> {
        let mut h = Sha256::new();
        h.update(b"InfinityOS NodeId v1");
        h.update(self.public_key);
        let id: [u8; 32] = h.finalize().into();
        if self.node.0 != id
            || self.node != p.node
            || self.resource != p.resource.0
            || self.device != p.device
            || self.repair != r.digest()
        {
            return Err(Error::Denied);
        }
        NodeCrypto::verify(&self.public_key, &self.transcript(), &self.signature)
            .map_err(|_| Error::Signature)
    }
}
pub const REPAIR_AUTHORIZATION_BYTES: usize =
    super::metadata_bundle::BUNDLE_BYTES + REPAIR_BUNDLE_BYTES;
#[derive(Clone, Copy)]
pub struct RepairAuthorization {
    pub anchor: super::metadata_bundle::Bundle,
    pub repair: RepairBundle,
}
impl RepairAuthorization {
    // ------------------------=
    // FUNC: decode_committed
    // DESC: Restores a quorum-certified durable repair after its admission lease expires without renewing authority for any new repair.
    // ------------------=
    pub fn decode_committed(bytes: &[u8]) -> Result<Self, Error> {
        let a = Self::decode(0, bytes)?;
        if a.repair.certificate.is_none() {
            return Err(Error::Quorum);
        }
        Ok(a)
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Carries the independently owner-certified payload to a replacement node without assuming cached owner state or sending credentials.
    // ------------------=
    pub fn encode(&self, now: u64) -> Result<[u8; REPAIR_AUTHORIZATION_BYTES], Error> {
        let anchor = self.anchor.certificate.ok_or(Error::Quorum)?;
        let mut out = [0; REPAIR_AUTHORIZATION_BYTES];
        let split = super::metadata_bundle::BUNDLE_BYTES;
        out[..split].copy_from_slice(&self.anchor.encode()?);
        self.repair.encode(
            &self.anchor.group,
            &anchor,
            &self.anchor.manifest,
            now,
            (&mut out[split..]).try_into().map_err(|_| Error::Invalid)?,
        )?;
        Ok(out)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Validates the canonical owner anchor, explicit repair delegation and placement payload before any receiver-side allocation.
    // ------------------=
    pub fn decode(now: u64, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != REPAIR_AUTHORIZATION_BYTES {
            return Err(Error::Invalid);
        }
        let split = super::metadata_bundle::BUNDLE_BYTES;
        let anchor = super::metadata_bundle::Bundle::decode(&bytes[..split])?;
        let cert = anchor.certificate.ok_or(Error::Quorum)?;
        let repair =
            RepairBundle::decode(&anchor.group, &cert, &anchor.manifest, now, &bytes[split..])?;
        Ok(Self { anchor, repair })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairBundle {
    pub grant: RepairGrant,
    pub value: RepairRecord,
    pub certificate: Option<RepairCertificate>,
    pub manifest: Manifest,
    pub availability: Option<RepairAvailability>,
}
impl RepairBundle {
    // ------------------------=
    // FUNC: validate_available
    // DESC: Requires a recipient-signed persistence receipt for each newly verified copy before any quorum voter may attest to repair.
    // ------------------=
    pub fn validate_available(&self, base: &Manifest) -> Result<(), Error> {
        let mut added = 0;
        for p in self.manifest.placements.iter().flatten() {
            if p.state == super::manifest::PlacementState::Verified
                && !base.placements.contains(&Some(*p))
            {
                added += 1;
                self.availability
                    .ok_or(Error::Denied)?
                    .validate(&self.value, p)?;
            }
        }
        if added != 1 {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes one validated bounded placement overlay, keeping cryptographic records independent of Rust layout.
    // ------------------=
    pub fn encode(
        &self,
        g: &Group,
        anchor: &Certificate,
        base: &Manifest,
        now: u64,
        out: &mut [u8; REPAIR_BUNDLE_BYTES],
    ) -> Result<(), Error> {
        self.value
            .validate(g, anchor, &self.grant, base, &self.manifest, now)?;
        if let Some(c) = self.certificate {
            self.validate_available(base)?;
            c.validate(g, anchor, &self.grant, base, &self.manifest, now)?;
            if c.value != self.value {
                return Err(Error::Conflict);
            }
        }
        out.fill(0);
        out[..256].copy_from_slice(&self.grant.transcript());
        out[256..320].copy_from_slice(&self.grant.signature);
        out[320..480].copy_from_slice(&self.value.transcript());
        out[480..544].copy_from_slice(&self.value.signature);
        if let Some(c) = self.certificate {
            out[799] = 1;
            for (i, r) in c.prepared.iter().enumerate() {
                let at = 544 + i * 128;
                out[at..at + 48].copy_from_slice(&r.transcript());
                out[at + 48..at + 112].copy_from_slice(&r.signature);
            }
        }
        let mut manifest = [0; MANIFEST_BYTES];
        self.manifest
            .encode(&mut manifest)
            .map_err(|_| Error::Invalid)?;
        out[800..800 + MANIFEST_BYTES].copy_from_slice(&manifest);
        if let Some(a) = self.availability {
            out[798] = 1;
            let at = 800 + MANIFEST_BYTES;
            out[at..at + 128].copy_from_slice(&a.transcript());
            out[at + 128..at + 160].copy_from_slice(&a.public_key);
            out[at + 160..at + 224].copy_from_slice(&a.signature);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects malformed length, padding, discriminants, signature substitutions and payload mismatch before returning an overlay.
    // ------------------=
    pub fn decode(
        g: &Group,
        anchor: &Certificate,
        base: &Manifest,
        now: u64,
        b: &[u8],
    ) -> Result<Self, Error> {
        if b.len() != REPAIR_BUNDLE_BYTES {
            return Err(Error::Invalid);
        }
        let grant = RepairGrant {
            group: b[8..40].try_into().unwrap(),
            anchor: b[40..72].try_into().unwrap(),
            writer: NodeId(b[72..104].try_into().unwrap()),
            expires: u64::from_le_bytes(b[104..112].try_into().unwrap()),
            destinations: core::array::from_fn(|i| {
                NodeId(b[112 + i * 32..144 + i * 32].try_into().unwrap())
            }),
            signature: b[256..320].try_into().unwrap(),
        };
        let value = RepairRecord {
            anchor: b[328..360].try_into().unwrap(),
            grant: b[360..392].try_into().unwrap(),
            sequence: u64::from_le_bytes(b[392..400].try_into().unwrap()),
            previous: b[400..432].try_into().unwrap(),
            manifest: b[432..464].try_into().unwrap(),
            signature: b[480..544].try_into().unwrap(),
        };
        let prepared = core::array::from_fn(|i| {
            let at = 544 + i * 128;
            Receipt {
                member: b[at + 8],
                published: b[at + 9] != 0,
                digest: b[at + 16..at + 48].try_into().unwrap(),
                signature: b[at + 48..at + 112].try_into().unwrap(),
            }
        });
        let bundle = Self {
            grant,
            value,
            certificate: if b[799] == 1 {
                Some(RepairCertificate { value, prepared })
            } else {
                None
            },
            manifest: Manifest::decode(&b[800..800 + MANIFEST_BYTES])
                .map_err(|_| Error::Invalid)?,
            availability: if b[798] == 1 {
                let at = 800 + MANIFEST_BYTES;
                Some(RepairAvailability {
                    repair: b[at + 8..at + 40].try_into().unwrap(),
                    node: NodeId(b[at + 40..at + 72].try_into().unwrap()),
                    resource: b[at + 72..at + 88].try_into().unwrap(),
                    device: b[at + 88..at + 104].try_into().unwrap(),
                    public_key: b[at + 128..at + 160].try_into().unwrap(),
                    signature: b[at + 160..at + 224].try_into().unwrap(),
                })
            } else {
                None
            },
        };
        let mut canonical = [0; REPAIR_BUNDLE_BYTES];
        bundle.encode(g, anchor, base, now, &mut canonical)?;
        if canonical != b {
            return Err(Error::Invalid);
        }
        Ok(bundle)
    }
}
// ------------------------=
// FUNC: verify_receipt
// DESC: Counts only a distinct group's authenticated durable receipt for the exact repair and phase.
// ------------------=
fn verify_receipt(g: &Group, r: &Receipt, digest: [u8; 32], published: bool) -> Result<(), Error> {
    if r.digest != digest || r.published != published {
        return Err(Error::Conflict);
    }
    let key = g.keys.get(r.member as usize).ok_or(Error::Denied)?;
    NodeCrypto::verify(key, &r.transcript(), &r.signature).map_err(|_| Error::Signature)
}
impl RepairCertificate {
    // ------------------------=
    // FUNC: validate
    // DESC: Requires two different durable staging members in addition to narrowly delegated repair authority.
    // ------------------=
    pub fn validate(
        &self,
        g: &Group,
        anchor: &Certificate,
        grant: &RepairGrant,
        base: &Manifest,
        next: &Manifest,
        now: u64,
    ) -> Result<(), Error> {
        self.value.validate(g, anchor, grant, base, next, now)?;
        if self.prepared[0].member == self.prepared[1].member {
            return Err(Error::Quorum);
        }
        for r in &self.prepared {
            verify_receipt(g, r, self.value.digest(), false)?;
        }
        Ok(())
    }
}
pub struct RepairPublication {
    certificate: RepairCertificate,
    group: [u8; 32],
    published: u8,
}
/// One fresh authenticated repair-head observation round, never a cache probe.
pub struct RepairReadRound {
    group: [u8; 32],
    anchor: [u8; 32],
    seen: u8,
    best: Option<RepairBundle>,
    written: u8,
}
impl RepairReadRound {
    // ------------------------=
    // FUNC: new
    // DESC: Pins one certified owner generation before receiving any current repair-head observations.
    // ------------------=
    pub fn new(g: &Group, anchor: &Certificate) -> Result<Self, Error> {
        anchor.validate(g)?;
        Ok(Self {
            group: g.digest(),
            anchor: anchor.value.record.digest(),
            seen: 0,
            best: None,
            written: 0,
        })
    }
    // ------------------------=
    // FUNC: observe_authenticated
    // DESC: Counts only one response from each member bound by the caller to THIS live correlated request; unreachable is never absence.
    // ------------------=
    pub fn observe_authenticated(
        &mut self,
        g: &Group,
        anchor: &Certificate,
        base: &Manifest,
        member: NodeId,
        value: Option<RepairBundle>,
        _now: u64,
    ) -> Result<(), Error> {
        anchor.validate(g)?;
        if self.group != g.digest() || self.anchor != anchor.value.record.digest() {
            return Err(Error::Denied);
        }
        let index = g
            .members
            .iter()
            .position(|n| *n == member)
            .ok_or(Error::Denied)?;
        if self.seen & (1 << index) != 0 {
            return Err(Error::Conflict);
        }
        if let Some(b) = value {
            b.validate_available(base)?;
            let cert = b.certificate.ok_or(Error::Quorum)?;
            cert.validate(g, anchor, &b.grant, base, &b.manifest, 0)?;
            if b.value != cert.value {
                return Err(Error::Conflict);
            }
            if let Some(old) = self.best {
                if old.value.sequence == b.value.sequence && old.value != b.value {
                    return Err(Error::Conflict);
                }
            }
            if self
                .best
                .is_none_or(|old| b.value.sequence > old.value.sequence)
            {
                self.best = Some(b);
                self.written = 0;
            }
        }
        self.seen |= 1 << index;
        Ok(())
    }
    // ------------------------=
    // FUNC: selected
    // DESC: Returns a candidate only after R2; None means two authenticated absences, never a timeout or a failed lookup.
    // ------------------=
    pub fn selected(&self) -> Result<Option<RepairBundle>, Error> {
        if self.seen.count_ones() < 2 {
            Err(Error::Quorum)
        } else {
            Ok(self.best)
        }
    }
    // ------------------------=
    // FUNC: acknowledge_writeback
    // DESC: Requires W2 of the exact selected repair before effective placements become usable.
    // ------------------=
    pub fn acknowledge_writeback(&mut self, g: &Group, r: Receipt) -> Result<(), Error> {
        g.validate()?;
        if g.digest() != self.group {
            return Err(Error::Denied);
        }
        let b = self.selected()?.ok_or(Error::Quorum)?;
        verify_receipt(g, &r, b.value.digest(), true)?;
        let bit = 1u8 << r.member;
        if self.written & bit != 0 {
            return Err(Error::Conflict);
        }
        self.written |= bit;
        Ok(())
    }
    // ------------------------=
    // FUNC: resolved
    // DESC: Allows an initial CAS after quorum absence, or an existing effective manifest only after repair write-back quorum.
    // ------------------=
    pub fn resolved(&self) -> Result<Option<RepairBundle>, Error> {
        let b = self.selected()?;
        if b.is_some() && self.written.count_ones() < 2 {
            Err(Error::Quorum)
        } else {
            Ok(b)
        }
    }
}
impl RepairPublication {
    // ------------------------=
    // FUNC: new
    // DESC: Begins publication only after validating actual manifest payload and staging quorum.
    // ------------------=
    pub fn new(
        g: &Group,
        anchor: &Certificate,
        grant: &RepairGrant,
        base: &Manifest,
        next: &Manifest,
        certificate: RepairCertificate,
        now: u64,
    ) -> Result<Self, Error> {
        certificate.validate(g, anchor, grant, base, next, now)?;
        Ok(Self {
            certificate,
            group: g.digest(),
            published: 0,
        })
    }
    // ------------------------=
    // FUNC: acknowledge
    // DESC: Requires distinct durable publication receipts; staging alone never marks a repaired placement committed.
    // ------------------=
    pub fn acknowledge(&mut self, g: &Group, r: Receipt) -> Result<(), Error> {
        g.validate()?;
        if self.group != g.digest() {
            return Err(Error::Denied);
        }
        verify_receipt(g, &r, self.certificate.value.digest(), true)?;
        let bit = 1u8 << r.member;
        if self.published & bit != 0 {
            return Err(Error::Conflict);
        }
        self.published |= bit;
        Ok(())
    }
    // ------------------------=
    // FUNC: committed
    // DESC: Exposes a repair certificate only after two independent publication acknowledgements.
    // ------------------=
    pub fn committed(&self) -> Result<RepairCertificate, Error> {
        if self.published.count_ones() < 2 {
            Err(Error::Quorum)
        } else {
            Ok(self.certificate)
        }
    }
}
#[cfg(test)]
#[path = "metadata_repair_tests.rs"]
mod tests;
