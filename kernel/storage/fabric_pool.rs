//! Native application-object/manifest ownership catalog. All mutations use the
//! same transaction root as content, never a second configuration database.
use crate::storage::{BlockDevice, object::{ObjectId, ObjectStore, ObjectError, ObjectType, Space, MAX_CONTENT}};
use crate::runtime::{fabric::{manifest::{Chunk, Manifest, Placement, PlacementState,
    MANIFEST_BYTES, MAX_CHUNKS, MAX_PLACEMENTS}, placement::StorageClass,
    resources::ResourceId}, node::types::NodeId};
use sha2::{Digest, Sha256};

const LIMIT: usize = 8;
const BYTES: usize = 32 + LIMIT * 128;
const PATH: &[u8] = b"/system/storage/pool-manifests";

#[derive(Clone, Copy, PartialEq, Eq)]
struct Entry {
    object: ObjectId, backing: ObjectId, owner: NodeId, scope: u64,
    nonce: u64, creation_hash: [u8; 32], creation_policy: u8,
}
struct Catalog { id: ObjectId, entries: [Option<Entry>; LIMIT] }
impl Catalog {
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes stable application identities separately from private manifest objects and creation retry identities.
    // ------------------=
    fn encode(&self) -> [u8; BYTES] {
        let mut out = [0; BYTES]; out[..8].copy_from_slice(b"INFPOOL1");
        for (i, e) in self.entries.iter().enumerate() {
            if let Some(e) = e {
                let at = 32 + i * 128;
                out[at..at+16].copy_from_slice(&e.object.0);
                out[at+16..at+32].copy_from_slice(&e.backing.0);
                out[at+32..at+64].copy_from_slice(&e.owner.0);
                out[at+64..at+72].copy_from_slice(&e.scope.to_le_bytes());
                out[at+72..at+80].copy_from_slice(&e.nonce.to_le_bytes());
                out[at+80..at+112].copy_from_slice(&e.creation_hash);
                out[at+112] = e.creation_policy;
            }
        }
        out
    }
    // ------------------------=
    // FUNC: load
    // DESC: Validates a committed catalog without silently replacing malformed ownership or duplicate application identities.
    // ------------------=
    fn load<D: BlockDevice>(store: &mut ObjectStore<D>) -> Result<Self, ObjectError> {
        let id = store.resolve(PATH)?;
        let mut bytes = [0; BYTES];
        if store.read(id, None, &mut bytes)? != BYTES || &bytes[..8] != b"INFPOOL1" {
            return Err(ObjectError::CorruptContent);
        }
        let mut result = Self { id, entries: [None; LIMIT] };
        for i in 0..LIMIT {
            let at = 32 + i * 128;
            if bytes[at..at+128].iter().all(|b| *b == 0) { continue; }
            let e = Entry { object: ObjectId(bytes[at..at+16].try_into().unwrap()),
                backing: ObjectId(bytes[at+16..at+32].try_into().unwrap()),
                owner: NodeId(bytes[at+32..at+64].try_into().unwrap()),
                scope: u64::from_le_bytes(bytes[at+64..at+72].try_into().unwrap()),
                nonce: u64::from_le_bytes(bytes[at+72..at+80].try_into().unwrap()),
                creation_hash: bytes[at+80..at+112].try_into().unwrap(), creation_policy: bytes[at+112] };
            if e.object.0 == [0; 16] || e.backing.0 == [0; 16] || e.object == e.backing
                || e.owner.0 == [0; 32] || e.nonce == 0 || !(1..=3).contains(&e.creation_policy)
                || result.entries.iter().flatten().any(|p| p.object == e.object || p.backing == e.backing
                    || (p.owner == e.owner && p.scope == e.scope && p.nonce == e.nonce)) {
                return Err(ObjectError::CorruptContent);
            }
            result.entries[i] = Some(e);
        }
        if result.encode() != bytes { return Err(ObjectError::CorruptContent); }
        Ok(result)
    }
}

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: pool_copy
    // DESC: Creates a new application identity and manifest sharing verified immutable local content; remote source placements are not falsely counted as replicas of the new object.
    // ------------------=
    pub(crate) fn pool_copy(&mut self, source: ObjectId, owner: NodeId, scope: u64,
        expected: u64, nonce: u64, local: NodeId, resource: ResourceId,
        device: [u8; 16], resource_generation: u64) -> Result<Manifest, ObjectError> {
        let previous = self.pool_manifest(source, owner, scope)?;
        if previous.generation != expected { return Err(ObjectError::InvalidVersion); }
        if nonce == 0 || local.0 == [0; 32] || device == [0; 16] || resource.0 == [0; 16]
            || resource_generation == 0 { return Err(ObjectError::InvalidObject); }
        let mut digest = Sha256::new(); digest.update(b"InfinityOS/PoolCopy/v1");
        digest.update(source.0); digest.update(previous.version.to_le_bytes()); digest.update(previous.hash);
        let retry_hash: [u8; 32] = digest.finalize().into();
        let mut catalog = Catalog::load(self)?;
        if let Some(e) = catalog.entries.iter().flatten().find(|e| e.owner == owner && e.scope == scope && e.nonce == nonce) {
            if e.creation_hash != retry_hash { return Err(ObjectError::InvalidVersion); }
            return self.pool_manifest(e.object, owner, scope);
        }
        // Integrity-check the immutable source before admitting shared backing.
        self.pool_read(&previous, 0, &mut [])?;
        let slot = catalog.entries.iter().position(Option::is_none).ok_or(ObjectError::InsufficientCapacity)?;
        let mut committed_manifest = None;
        self.copy_owned_bundle(source, previous.version, catalog.id, |object, backing| {
            let mut copied = previous;
            copied.object = object.0; copied.version = 1; copied.generation = 1; copied.healing = None;
            copied.placements = [None; MAX_PLACEMENTS];
            copied.placements[0] = Some(Placement { node: local, resource, device, generation: resource_generation,
                version: 1, hash: copied.hash, state: PlacementState::Verified, admission_generation: 1 });
            let mut bytes = [0; MANIFEST_BYTES]; copied.encode(&mut bytes).map_err(|_| ObjectError::InvalidObject)?;
            catalog.entries[slot] = Some(Entry { object, backing, owner, scope, nonce,
                creation_hash: retry_hash, creation_policy: copied.policy.replicas() as u8 });
            committed_manifest = Some(copied);
            Ok((catalog.encode(), bytes))
        })?;
        committed_manifest.ok_or(ObjectError::TransactionFailed)
    }
    // ------------------------=
    // FUNC: pool_inspect
    // DESC: Enumerates one owner-scoped committed application manifest per call; other principals' identities and metadata are never returned.
    // ------------------=
    pub(crate) fn pool_inspect(&mut self, owner: NodeId, scope: u64, index: usize)
        -> Result<(usize, Option<Manifest>), ObjectError> {
        let catalog = Catalog::load(self)?;
        let mut selected = catalog.entries.iter().flatten().filter(|e| e.owner == owner && e.scope == scope);
        let count = selected.clone().count();
        let object = selected.nth(index).map(|e| e.object);
        Ok((count, match object { Some(id) => Some(self.pool_manifest(id, owner, scope)?), None => None }))
    }
    // ------------------------=
    // FUNC: initialize_pool_catalog
    // DESC: Initializes the CORE native Pool registry once; existing corrupt metadata prevents service readiness rather than resetting user objects.
    // ------------------=
    pub(crate) fn initialize_pool_catalog(&mut self) -> Result<(), ObjectError> {
        match self.resolve(PATH) {
            Ok(_) => { Catalog::load(self)?; },
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => {
                let empty = Catalog { id: ObjectId([0; 16]), entries: [None; LIMIT] };
                self.create_attached(b"pool-manifests", ObjectType::Metadata, Space::System, &empty.encode(), PATH)?;
            },
            Err(e) => return Err(e),
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: pool_manifest
    // DESC: Resolves full application identity through committed ownership and validates the immutable version independently from namespace and physical placement.
    // ------------------=
    pub(crate) fn pool_manifest(&mut self, object: ObjectId, owner: NodeId, scope: u64)
        -> Result<Manifest, ObjectError> {
        let catalog = Catalog::load(self)?;
        let e = catalog.entries.iter().flatten().find(|e| e.object == object).ok_or(ObjectError::NotFound)?;
        if e.owner != owner || e.scope != scope { return Err(ObjectError::Unauthorized); }
        let mut bytes = [0; MANIFEST_BYTES];
        let len = self.read(e.backing, None, &mut bytes)?;
        let manifest = Manifest::decode(&bytes[..len]).map_err(|_| ObjectError::CorruptContent)?;
        if manifest.object != object.0 || manifest.authority != owner { return Err(ObjectError::CorruptContent); }
        Ok(manifest)
    }
    // ------------------------=
    // FUNC: pool_create
    // DESC: Atomically commits content, stable application identity, immutable integrity metadata, local placement and retry binding; policy never fabricates remote copies.
    // ------------------=
    pub(crate) fn pool_create(&mut self, owner: NodeId, scope: u64, nonce: u64,
        policy: StorageClass, content: &[u8], local: NodeId, resource: ResourceId,
        device: [u8; 16], resource_generation: u64) -> Result<Manifest, ObjectError> {
        if owner.0 == [0; 32] || local.0 == [0; 32] || nonce == 0 || resource.0 == [0; 16]
            || device == [0; 16] || resource_generation == 0 || content.len() > MAX_CONTENT {
            return Err(ObjectError::InvalidObject);
        }
        let mut catalog = Catalog::load(self)?;
        let hash: [u8; 32] = Sha256::digest(content).into();
        if let Some(e) = catalog.entries.iter().flatten().find(|e| e.owner == owner && e.scope == scope && e.nonce == nonce) {
            if e.creation_hash != hash || e.creation_policy != policy.replicas() as u8 {
                return Err(ObjectError::InvalidVersion);
            }
            return self.pool_manifest(e.object, owner, scope);
        }
        let slot = catalog.entries.iter().position(Option::is_none).ok_or(ObjectError::InsufficientCapacity)?;
        let mut committed_manifest = None;
        self.create_owned_bundle(catalog.id, content, |object, backing| {
            let mut manifest = Manifest { object: object.0, version: 1, length: content.len() as u64, hash,
                policy, minimum_available: 1, generation: 1, authority: owner, authority_generation: 1,
                chunks: [None; MAX_CHUNKS], placements: [None; MAX_PLACEMENTS], healing: None };
            if !content.is_empty() {
                manifest.chunks[0] = Some(Chunk { content: hash[..16].try_into().unwrap(), bytes: content.len() as u32, hash });
            }
            manifest.placements[0] = Some(Placement { node: local, resource, device,
                generation: resource_generation, version: 1, hash, state: PlacementState::Verified,
                admission_generation: 1 });
            let mut bytes = [0; MANIFEST_BYTES];
            manifest.encode(&mut bytes).map_err(|_| ObjectError::InvalidObject)?;
            catalog.entries[slot] = Some(Entry { object, backing, owner, scope, nonce,
                creation_hash: hash, creation_policy: policy.replicas() as u8 });
            committed_manifest = Some(manifest);
            Ok((catalog.encode(), bytes))
        })?;
        committed_manifest.ok_or(ObjectError::TransactionFailed)
    }
    // ------------------------=
    // FUNC: pool_set_policy
    // DESC: Commits a generation-fenced protection contract without altering immutable content, namespace or admission authority of existing replicas.
    // ------------------=
    pub(crate) fn pool_set_policy(&mut self, object: ObjectId, owner: NodeId, scope: u64,
        expected: u64, policy: StorageClass) -> Result<Manifest, ObjectError> {
        let previous = self.pool_manifest(object, owner, scope)?;
        if previous.generation != expected { return Err(ObjectError::InvalidVersion); }
        if previous.policy == policy { return Ok(previous); }
        let catalog = Catalog::load(self)?;
        let e = catalog.entries.iter().flatten().find(|e| e.object == object).ok_or(ObjectError::NotFound)?;
        let mut next = previous;
        next.generation = next.generation.checked_add(1).ok_or(ObjectError::InvalidVersion)?;
        next.policy = policy; next.minimum_available = next.minimum_available.min(policy.replicas() as u8);
        previous.successor(&next).map_err(|_| ObjectError::InvalidVersion)?;
        let mut bytes = [0; MANIFEST_BYTES];
        next.encode(&mut bytes).map_err(|_| ObjectError::InvalidObject)?;
        self.replace_state(e.backing, &bytes)?;
        Ok(next)
    }
    // ------------------------=
    // FUNC: pool_update
    // DESC: Atomically advances content and its manifest while retaining stale physical placements as non-authoritative and preserving object identity.
    // ------------------=
    pub(crate) fn pool_update(&mut self, object: ObjectId, owner: NodeId, scope: u64,
        expected: u64, content: &[u8], local: NodeId, resource: ResourceId,
        device: [u8; 16], resource_generation: u64) -> Result<Manifest, ObjectError> {
        let previous = self.pool_manifest(object, owner, scope)?;
        if previous.generation != expected || previous.healing.is_some() { return Err(ObjectError::InvalidVersion); }
        if content.len() > MAX_CONTENT || local.0 == [0; 32] || device == [0; 16]
            || resource.0 == [0; 16] || resource_generation == 0 { return Err(ObjectError::InvalidObject); }
        let catalog = Catalog::load(self)?;
        let e = catalog.entries.iter().flatten().find(|e| e.object == object).ok_or(ObjectError::NotFound)?;
        let mut committed_manifest = None;
        self.update_owned_bundle(object, e.backing, content, |version| {
            if previous.version.checked_add(1) != Some(version as u64) { return Err(ObjectError::InvalidVersion); }
            let mut next = previous;
            next.version = version as u64;
            next.generation = next.generation.checked_add(1).ok_or(ObjectError::InvalidVersion)?;
            next.length = content.len() as u64; next.hash = Sha256::digest(content).into();
            next.chunks = [None; MAX_CHUNKS];
            if !content.is_empty() { next.chunks[0] = Some(Chunk { content: next.hash[..16].try_into().unwrap(),
                bytes: content.len() as u32, hash: next.hash }); }
            for p in next.placements.iter_mut().flatten() { p.state = PlacementState::Stale; }
            let slot = next.placements.iter().position(|p| p.is_some_and(|p| p.resource == resource))
                .or_else(|| next.placements.iter().position(Option::is_none)).ok_or(ObjectError::InsufficientCapacity)?;
            next.placements[slot] = Some(Placement { node: local, resource, device, generation: resource_generation,
                version: next.version, hash: next.hash, state: PlacementState::Verified, admission_generation: next.generation });
            previous.successor(&next).map_err(|_| ObjectError::InvalidVersion)?;
            let mut bytes = [0; MANIFEST_BYTES]; next.encode(&mut bytes).map_err(|_| ObjectError::InvalidObject)?;
            committed_manifest = Some(next);
            Ok(bytes)
        })?;
        committed_manifest.ok_or(ObjectError::TransactionFailed)
    }
    // ------------------------=
    // FUNC: pool_read
    // DESC: Returns a bounded range only after native content verification and the authoritative manifest digest both pass; no partial or changed backing content is returned.
    // ------------------=
    pub(crate) fn pool_read(&mut self, manifest: &Manifest, offset: u64, out: &mut [u8]) -> Result<(), ObjectError> {
        if out.len() > 64 || offset.checked_add(out.len() as u64).is_none_or(|n| n > manifest.length) {
            return Err(ObjectError::InvalidObject);
        }
        let mut content = [0; MAX_CONTENT];
        let version = u32::try_from(manifest.version).map_err(|_| ObjectError::InvalidVersion)?;
        let size = self.read(ObjectId(manifest.object), Some(version), &mut content)?;
        if size as u64 != manifest.length || <[u8; 32]>::from(Sha256::digest(&content[..size])) != manifest.hash {
            return Err(ObjectError::CorruptContent);
        }
        out.copy_from_slice(&content[offset as usize..offset as usize+out.len()]);
        Ok(())
    }
}
