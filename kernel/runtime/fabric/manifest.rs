//! Authoritative immutable-version description. Physical replica identity is
//! distinct from the application ObjectId and from every content reference.
use super::{placement::{StorageClass, Availability, availability}, resources::ResourceId};
use crate::runtime::node::types::NodeId;

pub const MAX_CHUNKS: usize = 64;
pub const MAX_PLACEMENTS: usize = 8;
pub const MANIFEST_BYTES: usize = 256 + MAX_CHUNKS * 64 + MAX_PLACEMENTS * 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk { pub content: [u8; 16], pub bytes: u32, pub hash: [u8; 32] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlacementState { Staging = 1, Verified = 2, Offline = 3, Stale = 4, Corrupt = 5 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub node: NodeId, pub resource: ResourceId, pub device: [u8; 16],
    pub generation: u64, pub version: u64, pub hash: [u8; 32], pub state: PlacementState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingClaim {
    pub owner: NodeId, pub token: u64, pub expires: u64,
    pub destination: ResourceId, pub reserved: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub object: [u8; 16], pub version: u64, pub length: u64, pub hash: [u8; 32],
    pub policy: StorageClass, pub minimum_available: u8, pub generation: u64,
    pub authority: NodeId, pub authority_generation: u64,
    pub chunks: [Option<Chunk>; MAX_CHUNKS],
    pub placements: [Option<Placement>; MAX_PLACEMENTS],
    pub healing: Option<HealingClaim>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestError { Invalid, Conflict, Stale, Storage }

impl Manifest {
    // ------------------------=
    // FUNC: validate
    // DESC: Checks bounded contiguous content, authoritative version, exact replica identity and independent physical records before any commit.
    // ------------------=
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.object == [0; 16] || self.version == 0 || self.generation == 0
            || self.authority.0 == [0; 32] || self.authority_generation == 0
            || self.minimum_available == 0 || self.minimum_available as usize > self.policy.replicas() {
            return Err(ManifestError::Invalid);
        }
        let mut size = 0u64;
        let mut ended = false;
        for chunk in self.chunks {
            match chunk {
                None => ended = true,
                Some(chunk) => {
                    if ended || chunk.content == [0; 16] || chunk.bytes == 0 || chunk.bytes > 16384 {
                        return Err(ManifestError::Invalid);
                    }
                    size = size.checked_add(chunk.bytes as u64).ok_or(ManifestError::Invalid)?;
                }
            }
        }
        if size != self.length { return Err(ManifestError::Invalid); }
        if size == 0 {
            use sha2::{Digest, Sha256};
            let empty: [u8; 32] = Sha256::digest([]).into();
            if self.hash != empty { return Err(ManifestError::Invalid); }
        }
        for (index, record) in self.placements.iter().enumerate() {
            let Some(record) = record else { continue; };
            if record.node.0 == [0; 32] || record.resource.0 == [0; 16] || record.device == [0; 16]
                || record.generation == 0 || record.version == 0
                || self.placements[..index].iter().flatten().any(|other|
                    other.resource == record.resource || (other.node == record.node && other.device == record.device)) {
                return Err(ManifestError::Invalid);
            }
            if record.state == PlacementState::Verified
                && (record.version != self.version || record.hash != self.hash) {
                return Err(ManifestError::Stale);
            }
        }
        if let Some(claim) = self.healing {
            if claim.owner != self.authority || claim.token == 0 || claim.expires == 0
                || claim.destination.0 == [0; 16] || claim.reserved < self.length {
                return Err(ManifestError::Invalid);
            }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: availability
    // DESC: Counts only current verified copies on distinct node failure domains; the resource observer must mark offline placements unavailable.
    // ------------------=
    pub fn availability(&self) -> Availability {
        let mut count = 0;
        for (index, record) in self.placements.iter().enumerate() {
            let Some(record) = record else { continue; };
            if record.state != PlacementState::Verified || record.version != self.version || record.hash != self.hash { continue; }
            if !self.placements[..index].iter().flatten().any(|prior|
                prior.state == PlacementState::Verified && prior.version == self.version
                && prior.hash == self.hash && prior.node == record.node) { count += 1; }
        }
        availability(self.policy, count)
    }

    // ------------------------=
    // FUNC: successor
    // DESC: Fences stale writers and prevents in-place content changes within an immutable object version.
    // ------------------=
    pub fn successor(&self, next: &Self) -> Result<(), ManifestError> {
        next.validate()?;
        if next.object != self.object || next.authority != self.authority
            || next.authority_generation != self.authority_generation { return Err(ManifestError::Conflict); }
        if self.generation.checked_add(1) != Some(next.generation)
            || next.version < self.version || next.version > self.version.saturating_add(1) {
            return Err(ManifestError::Stale);
        }
        if next.version == self.version && (next.length != self.length || next.hash != self.hash || next.chunks != self.chunks) {
            return Err(ManifestError::Conflict);
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Serializes a canonical fixed-size little-endian durable manifest; no Rust layout or pointers cross storage boundaries.
    // ------------------=
    pub fn encode(&self, out: &mut [u8; MANIFEST_BYTES]) -> Result<(), ManifestError> {
        self.validate()?;
        out.fill(0); out[..8].copy_from_slice(b"INFPMF01");
        out[8..24].copy_from_slice(&self.object);
        put(out, 24, self.version); put(out, 32, self.length); out[40..72].copy_from_slice(&self.hash);
        out[72] = self.policy.replicas() as u8;
        out[73] = self.minimum_available;
        put(out, 80, self.generation); out[88..120].copy_from_slice(&self.authority.0);
        put(out, 120, self.authority_generation);
        if let Some(claim) = self.healing {
            out[128] = 1; out[136..168].copy_from_slice(&claim.owner.0);
            put(out, 168, claim.token); put(out, 176, claim.expires);
            out[184..200].copy_from_slice(&claim.destination.0); put(out, 200, claim.reserved);
        }
        for (index, chunk) in self.chunks.iter().enumerate() {
            if let Some(chunk) = chunk {
                let at = 256 + index * 64; out[at] = 1;
                out[at+8..at+24].copy_from_slice(&chunk.content);
                put(out, at+24, chunk.bytes as u64); out[at+32..at+64].copy_from_slice(&chunk.hash);
            }
        }
        for (index, placement) in self.placements.iter().enumerate() {
            if let Some(p) = placement {
                let at = 256 + MAX_CHUNKS * 64 + index * 128;
                out[at] = p.state as u8; out[at+8..at+40].copy_from_slice(&p.node.0);
                out[at+40..at+56].copy_from_slice(&p.resource.0); out[at+56..at+72].copy_from_slice(&p.device);
                put(out, at+72, p.generation); put(out, at+80, p.version); out[at+88..at+120].copy_from_slice(&p.hash);
            }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Rejects malformed lengths, discriminants, reserved data and inconsistent manifests before constructing authoritative state.
    // ------------------=
    pub fn decode(bytes: &[u8]) -> Result<Self, ManifestError> {
        if bytes.len() != MANIFEST_BYTES || &bytes[..8] != b"INFPMF01" { return Err(ManifestError::Invalid); }
        let mut value = Self { object: bytes[8..24].try_into().unwrap(), version: get(bytes, 24),
            length: get(bytes, 32), hash: bytes[40..72].try_into().unwrap(),
            policy: match bytes[72] { 1 => StorageClass::Temporary, 2 => StorageClass::Protected,
                3 => StorageClass::Critical, _ => return Err(ManifestError::Invalid) },
            minimum_available: bytes[73], generation: get(bytes, 80), authority: NodeId(bytes[88..120].try_into().unwrap()),
            authority_generation: get(bytes, 120), chunks: [None; MAX_CHUNKS], placements: [None; MAX_PLACEMENTS], healing: None };
        if bytes[128] == 1 {
            value.healing = Some(HealingClaim { owner: NodeId(bytes[136..168].try_into().unwrap()),
                token: get(bytes, 168), expires: get(bytes, 176), destination: ResourceId(bytes[184..200].try_into().unwrap()), reserved: get(bytes, 200) });
        } else if bytes[128] != 0 { return Err(ManifestError::Invalid); }
        for (index, chunk) in value.chunks.iter_mut().enumerate() {
            let at = 256 + index * 64;
            if bytes[at] == 1 {
                *chunk = Some(Chunk { content: bytes[at+8..at+24].try_into().unwrap(),
                    bytes: u32::try_from(get(bytes, at+24)).map_err(|_| ManifestError::Invalid)?, hash: bytes[at+32..at+64].try_into().unwrap() });
            } else if bytes[at] != 0 { return Err(ManifestError::Invalid); }
        }
        for (index, placement) in value.placements.iter_mut().enumerate() {
            let at = 256 + MAX_CHUNKS * 64 + index * 128;
            let state = match bytes[at] { 0 => continue, 1 => PlacementState::Staging, 2 => PlacementState::Verified,
                3 => PlacementState::Offline, 4 => PlacementState::Stale, 5 => PlacementState::Corrupt, _ => return Err(ManifestError::Invalid) };
            *placement = Some(Placement { node: NodeId(bytes[at+8..at+40].try_into().unwrap()), resource: ResourceId(bytes[at+40..at+56].try_into().unwrap()),
                device: bytes[at+56..at+72].try_into().unwrap(), generation: get(bytes, at+72), version: get(bytes, at+80), hash: bytes[at+88..at+120].try_into().unwrap(), state });
        }
        let mut canonical = [0; MANIFEST_BYTES]; value.encode(&mut canonical)?;
        if canonical != bytes { return Err(ManifestError::Invalid); }
        Ok(value)
    }
}

// ------------------------=
// FUNC: put
// DESC: Writes a validated fixed-schema integer field.
// ------------------=
fn put(out: &mut [u8], at: usize, value: u64) { out[at..at+8].copy_from_slice(&value.to_le_bytes()); }
// ------------------------=
// FUNC: get
// DESC: Reads an integer only after the full fixed manifest extent has been checked.
// ------------------=
fn get(bytes: &[u8], at: usize) -> u64 { u64::from_le_bytes(bytes[at..at+8].try_into().unwrap()) }
