//! One durable bounded upload per node. Each extent is independently allocated;
//! transport windows never exceed 1 KiB and final verification advances 1 KiB.
use super::*;
const PATH_UPLOAD: &[u8] = b"/system/storage/pool-upload";
const UPLOAD_BYTES: usize = 432;
pub(super) const INDEX_BYTES: usize = 272;
const EXTENT: usize = 16384;

#[derive(Clone)]
struct Upload {
    id: ObjectId,
    owner: NodeId,
    scope: u64,
    nonce: u64,
    length: u32,
    offset: u32,
    hash: [u8; 32],
    policy: StorageClass,
    target: ObjectId,
    expected: u64,
    ids: [ObjectId; 16],
    ticket: u64,
    complete: bool,
    result: ObjectId,
    result_generation: u64,
}
#[derive(Clone)]
pub(crate) struct UploadVerifier {
    id: ObjectId,
    hash: [u8; 32],
    at: u32,
    total: Sha256,
    chunk: Sha256,
    hashes: [[u8; 32]; 16],
    crc: u32,
    crcs: [u32; 16],
    ticket: u64,
}
impl UploadVerifier {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty incremental verifier; upload bytes remain durable when volatile verification restarts.
    // ------------------=
    pub(crate) fn new() -> Self {
        Self {
            id: ObjectId([0; 16]),
            hash: [0; 32],
            at: 0,
            total: Sha256::new(),
            chunk: Sha256::new(),
            hashes: [[0; 32]; 16],
            crc: !0,
            crcs: [0; 16],
            ticket: 0,
        }
    }
}
impl Upload {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a fixed-size authenticated upload descriptor and exact durable prefix.
    // ------------------=
    fn encode(&self) -> [u8; UPLOAD_BYTES] {
        let mut b = [0; UPLOAD_BYTES];
        b[..8].copy_from_slice(b"INFUPLD1");
        b[8..40].copy_from_slice(&self.owner.0);
        b[40..48].copy_from_slice(&self.scope.to_le_bytes());
        b[48..56].copy_from_slice(&self.nonce.to_le_bytes());
        b[56..60].copy_from_slice(&self.length.to_le_bytes());
        b[60..64].copy_from_slice(&self.offset.to_le_bytes());
        b[64..96].copy_from_slice(&self.hash);
        b[96] = self.policy.replicas() as u8;
        b[104..120].copy_from_slice(&self.target.0);
        b[120..128].copy_from_slice(&self.expected.to_le_bytes());
        for (i, id) in self.ids.iter().enumerate() {
            b[128 + i * 16..144 + i * 16].copy_from_slice(&id.0);
        }
        b[384..392].copy_from_slice(&self.ticket.to_le_bytes());
        b[392] = self.complete as u8;
        b[400..416].copy_from_slice(&self.result.0);
        b[416..424].copy_from_slice(&self.result_generation.to_le_bytes());
        b
    }
    // ------------------------=
    // FUNC: load
    // DESC: Rejects malformed or foreign persisted upload ownership before any extent access.
    // ------------------=
    fn load<D: BlockDevice>(
        store: &mut ObjectStore<D>,
        id: ObjectId,
        owner: NodeId,
        scope: u64,
    ) -> Result<Self, ObjectError> {
        if store.resolve(PATH_UPLOAD)? != id {
            return Err(ObjectError::Unauthorized);
        }
        let mut b = [0; UPLOAD_BYTES];
        if store.read(id, None, &mut b)? != UPLOAD_BYTES || &b[..8] != b"INFUPLD1" {
            return Err(ObjectError::NotFound);
        }
        let mut u = Self {
            id,
            owner: NodeId(b[8..40].try_into().unwrap()),
            scope: u64::from_le_bytes(b[40..48].try_into().unwrap()),
            nonce: u64::from_le_bytes(b[48..56].try_into().unwrap()),
            length: u32::from_le_bytes(b[56..60].try_into().unwrap()),
            offset: u32::from_le_bytes(b[60..64].try_into().unwrap()),
            hash: b[64..96].try_into().unwrap(),
            policy: match b[96] {
                1 => StorageClass::Temporary,
                2 => StorageClass::Protected,
                3 => StorageClass::Critical,
                _ => return Err(ObjectError::CorruptContent),
            },
            target: ObjectId(b[104..120].try_into().unwrap()),
            expected: u64::from_le_bytes(b[120..128].try_into().unwrap()),
            ids: [ObjectId([0; 16]); 16],
            ticket: u64::from_le_bytes(b[384..392].try_into().unwrap()),
            complete: b[392] == 1,
            result: ObjectId(b[400..416].try_into().unwrap()),
            result_generation: u64::from_le_bytes(b[416..424].try_into().unwrap()),
        };
        if u.owner != owner || u.scope != scope {
            return Err(ObjectError::Unauthorized);
        }
        if u.length > 262144 || u.offset > u.length || u.nonce == 0 {
            return Err(ObjectError::CorruptContent);
        }
        for (i, id) in u.ids.iter_mut().enumerate() {
            *id = ObjectId(b[128 + i * 16..144 + i * 16].try_into().unwrap());
            if (i < (u.length as usize + EXTENT - 1) / EXTENT) != (id.0 != [0; 16]) {
                return Err(ObjectError::CorruptContent);
            }
        }
        if u.encode() != b {
            return Err(ObjectError::CorruptContent);
        }
        Ok(u)
    }
}
impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: pool_verified_chunk
    // DESC: Fills a caller-owned bounded immutable extent cache after checking its index binding and manifest digest; no cache lives on the generic store stack.
    // ------------------=
    pub(crate) fn pool_verified_chunk(
        &mut self,
        m: &Manifest,
        chunk_index: usize,
        out: &mut [u8; 16384],
    ) -> Result<usize, ObjectError> {
        let chunk = m
            .chunks
            .get(chunk_index)
            .and_then(|c| *c)
            .ok_or(ObjectError::InvalidObject)?;
        if chunk.bytes as usize > out.len() {
            return Err(ObjectError::InvalidObject);
        }
        let version = u32::try_from(m.version).map_err(|_| ObjectError::InvalidVersion)?;
        if !self.pool_is_index(ObjectId(m.object), version) {
            if chunk_index != 0 {
                return Err(ObjectError::InvalidObject);
            }
            let n = self.read(ObjectId(m.object), Some(version), out)?;
            if n != chunk.bytes as usize
                || <[u8; 32]>::from(Sha256::digest(&out[..n])) != chunk.hash
            {
                return Err(ObjectError::CorruptContent);
            }
            return Ok(n);
        }
        let mut index = [0; INDEX_BYTES];
        let n = self.read(ObjectId(m.object), Some(version), &mut index)?;
        if n != INDEX_BYTES
            || chunk_index >= 16
            || &index[..8] != b"INFPIDX1"
            || index[16 + chunk_index * 16..32 + chunk_index * 16] != chunk.content
        {
            return Err(ObjectError::CorruptContent);
        }
        let mut at = 0;
        while at < chunk.bytes as usize {
            let n = (chunk.bytes as usize - at).min(1024);
            self.read_extent_range(ObjectId(chunk.content), at as u64, &mut out[at..at + n])?;
            at += n;
        }
        if <[u8; 32]>::from(Sha256::digest(&out[..at])) != chunk.hash {
            return Err(ObjectError::CorruptContent);
        }
        Ok(at)
    }
    // ------------------------=
    // FUNC: pool_upload_begin
    // DESC: Reserves independent bounded extents with durable owner/scope and generation fencing; repeated admission resumes only identical requests.
    // ------------------=
    pub(crate) fn pool_upload_begin(
        &mut self,
        owner: NodeId,
        scope: u64,
        nonce: u64,
        policy: StorageClass,
        length: u32,
        hash: [u8; 32],
        target: ObjectId,
        expected: u64,
    ) -> Result<(ObjectId, u32), ObjectError> {
        if owner.0 == [0; 32] || nonce == 0 || length > 262144 {
            return Err(ObjectError::InvalidObject);
        }
        if target.0 != [0; 16] {
            self.require_pool_mutation_authority(target)?;
            let previous = self.pool_manifest(target, owner, scope)?;
            if previous.generation != expected || previous.healing.is_some() {
                return Err(ObjectError::InvalidVersion);
            }
        } else if expected != 0 {
            return Err(ObjectError::InvalidVersion);
        }
        let id = match self.resolve(PATH_UPLOAD) {
            Ok(id) => id,
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => self.create_attached(
                b"pool-upload",
                ObjectType::Metadata,
                Space::System,
                &[],
                PATH_UPLOAD,
            )?,
            Err(e) => return Err(e),
        };
        if self.metadata(id)?.current_version != 0 {
            let mut b = [0; UPLOAD_BYTES];
            if self.read(id, None, &mut b)? != 0 {
                let u = Upload::load(self, id, owner, scope)?;
                if u.nonce == nonce
                    && u.length == length
                    && u.hash == hash
                    && u.target == target
                    && u.expected == expected
                    && u.policy == policy
                {
                    return Ok((id, u.offset));
                }
                if !u.complete {
                    return Err(ObjectError::InsufficientCapacity);
                }
            }
        }
        if target.0 == [0; 16]
            && Catalog::load(self)?
                .entries
                .iter()
                .flatten()
                .any(|e| e.owner == owner && e.scope == scope && e.nonce == nonce)
        {
            return Err(ObjectError::InvalidVersion);
        }
        let ticket = self
            .generation()
            .checked_add(1)
            .ok_or(ObjectError::InvalidVersion)?;
        self.pool_reserve_extents(id, length, |ids| {
            Upload {
                id,
                owner,
                scope,
                nonce,
                length,
                offset: 0,
                hash,
                policy,
                target,
                expected,
                ids,
                ticket,
                complete: false,
                result: ObjectId([0; 16]),
                result_generation: 0,
            }
            .encode()
        })?;
        Ok((id, 0))
    }
    // ------------------------=
    // FUNC: pool_upload_append
    // DESC: Durably advances one exact prefix after flushing bounded content; duplicate acknowledged windows are byte-verified and gaps are rejected.
    // ------------------=
    pub(crate) fn pool_upload_append(
        &mut self,
        id: ObjectId,
        owner: NodeId,
        scope: u64,
        offset: u32,
        bytes: &[u8],
    ) -> Result<u32, ObjectError> {
        let mut u = Upload::load(self, id, owner, scope)?;
        if u.complete {
            return Err(ObjectError::InvalidVersion);
        }
        if bytes.is_empty()
            || bytes.len() > 1024
            || offset
                .checked_add(bytes.len() as u32)
                .is_none_or(|end| end > u.length)
        {
            return Err(ObjectError::InvalidObject);
        }
        if offset < u.offset {
            if offset + bytes.len() as u32 > u.offset {
                return Err(ObjectError::InvalidVersion);
            }
            let mut old = [0; 1024];
            let mut at = 0;
            while at < bytes.len() {
                let absolute = offset as usize + at;
                let n = (bytes.len() - at).min(EXTENT - absolute % EXTENT);
                self.read_extent_range(
                    u.ids[absolute / EXTENT],
                    (absolute % EXTENT) as u64,
                    &mut old[at..at + n],
                )?;
                at += n;
            }
            return if old[..bytes.len()] == *bytes {
                Ok(u.offset)
            } else {
                Err(ObjectError::CorruptContent)
            };
        }
        if offset != u.offset {
            return Err(ObjectError::InvalidVersion);
        }
        let mut at = 0;
        while at < bytes.len() {
            let absolute = offset as usize + at;
            let n = (bytes.len() - at).min(EXTENT - absolute % EXTENT);
            self.write_staging_range(
                u.ids[absolute / EXTENT],
                (absolute % EXTENT) as u64,
                &bytes[at..at + n],
            )?;
            at += n;
        }
        u.offset += bytes.len() as u32;
        self.replace_state(id, &u.encode())?;
        Ok(u.offset)
    }
    // ------------------------=
    // FUNC: pool_upload_abort
    // DESC: Reclaims only this authenticated unpublished upload and atomically clears its durable descriptor.
    // ------------------=
    pub(crate) fn pool_upload_abort(
        &mut self,
        id: ObjectId,
        owner: NodeId,
        scope: u64,
    ) -> Result<(), ObjectError> {
        let u = Upload::load(self, id, owner, scope)?;
        if u.complete {
            return Err(ObjectError::InvalidVersion);
        }
        self.pool_retire_owned(&u.ids, id, &[])
    }
    // ------------------------=
    // FUNC: pool_upload_commit_step
    // DESC: Verifies at most one KiB per invocation and publishes all content and metadata only after final and per-extent integrity succeeds.
    // ------------------=
    pub(crate) fn pool_upload_commit_step(
        &mut self,
        id: ObjectId,
        owner: NodeId,
        scope: u64,
        verifier: &mut UploadVerifier,
        local: NodeId,
        resource: ResourceId,
        device: [u8; 16],
        resource_generation: u64,
    ) -> Result<Option<Manifest>, ObjectError> {
        let u = Upload::load(self, id, owner, scope)?;
        if u.complete {
            let m = self.pool_manifest(u.result, owner, scope)?;
            return if m.generation == u.result_generation && m.hash == u.hash {
                Ok(Some(m))
            } else {
                Err(ObjectError::InvalidVersion)
            };
        }
        if u.target.0 != [0; 16] { self.require_pool_mutation_authority(u.target)?; }
        if u.offset != u.length
            || local.0 == [0; 32]
            || resource.0 == [0; 16]
            || device == [0; 16]
            || resource_generation == 0
        {
            return Err(ObjectError::InvalidObject);
        }
        if verifier.id != id || verifier.hash != u.hash || verifier.ticket != u.ticket {
            *verifier = UploadVerifier::new();
            verifier.id = id;
            verifier.hash = u.hash;
            verifier.ticket = u.ticket;
        }
        if verifier.at < u.length {
            let i = verifier.at as usize / EXTENT;
            let within = verifier.at as usize % EXTENT;
            let n = (u.length - verifier.at)
                .min(1024)
                .min((EXTENT - within) as u32) as usize;
            let mut b = [0; 1024];
            self.read_extent_range(u.ids[i], within as u64, &mut b[..n])?;
            verifier.total.update(&b[..n]);
            verifier.chunk.update(&b[..n]);
            for byte in &b[..n] {
                verifier.crc ^= *byte as u32;
                for _ in 0..8 {
                    verifier.crc = (verifier.crc >> 1)
                        ^ (0xedb88320u32 & (0u32.wrapping_sub(verifier.crc & 1)));
                }
            }
            verifier.at += n as u32;
            if verifier.at == u.length || verifier.at as usize % EXTENT == 0 {
                verifier.hashes[i] = verifier.chunk.clone().finalize().into();
                verifier.chunk = Sha256::new();
                verifier.crcs[i] = !verifier.crc;
                verifier.crc = !0;
            }
            return Ok(None);
        }
        if <[u8; 32]>::from(verifier.total.clone().finalize()) != u.hash {
            *verifier = UploadVerifier::new();
            return Err(ObjectError::CorruptContent);
        }
        let mut catalog = Catalog::load(self)?;
        let previous = if u.target.0 == [0; 16] {
            None
        } else {
            Some(self.pool_manifest(u.target, owner, scope)?)
        };
        if previous.is_some_and(|m| m.generation != u.expected || m.healing.is_some()) {
            return Err(ObjectError::InvalidVersion);
        }
        if previous.is_none()
            && catalog
                .entries
                .iter()
                .flatten()
                .any(|e| e.owner == owner && e.scope == scope && e.nonce == u.nonce)
        {
            return Err(ObjectError::InvalidVersion);
        }
        let slot = if previous.is_some() {
            catalog
                .entries
                .iter()
                .position(|e| e.is_some_and(|e| e.object == u.target))
        } else {
            catalog.entries.iter().position(Option::is_none)
        }
        .ok_or(ObjectError::InsufficientCapacity)?;
        let backing = if previous.is_some() {
            catalog.entries[slot].map(|e| e.backing)
        } else {
            None
        };
        let mut index = [0; INDEX_BYTES];
        index[..8].copy_from_slice(b"INFPIDX1");
        index[8..12].copy_from_slice(&u.length.to_le_bytes());
        for (i, extent) in u.ids.iter().enumerate() {
            index[16 + i * 16..32 + i * 16].copy_from_slice(&extent.0);
        }
        let mut committed = None;
        self.pool_publish_extents(
            catalog.id,
            id,
            &u.ids,
            &verifier.crcs,
            previous.map(|_| u.target),
            backing,
            &index,
            |object, backing, version| {
                let generation = previous.map_or(1, |m| m.generation + 1);
                let mut m = Manifest {
                    object: object.0,
                    version: version as u64,
                    length: u.length as u64,
                    hash: u.hash,
                    policy: u.policy,
                    minimum_available: 1,
                    generation,
                    authority: owner,
                    authority_generation: 1,
                    chunks: [None; MAX_CHUNKS],
                    placements: [None; MAX_PLACEMENTS],
                    healing: None,
                };
                if let Some(p) = previous {
                    m.placements = p.placements;
                    for placement in m.placements.iter_mut().flatten() {
                        placement.state = PlacementState::Stale;
                    }
                }
                for (i, extent) in u.ids.iter().enumerate().filter(|(_, id)| id.0 != [0; 16]) {
                    m.chunks[i] = Some(Chunk {
                        content: extent.0,
                        bytes: (u.length as usize - i * EXTENT).min(EXTENT) as u32,
                        hash: verifier.hashes[i],
                    });
                }
                let placement = m
                    .placements
                    .iter()
                    .position(|p| p.is_some_and(|p| p.resource == resource))
                    .or_else(|| m.placements.iter().position(Option::is_none))
                    .ok_or(ObjectError::InsufficientCapacity)?;
                m.placements[placement] = Some(Placement {
                    node: local,
                    resource,
                    device,
                    generation: resource_generation,
                    version: m.version,
                    hash: m.hash,
                    state: PlacementState::Verified,
                    admission_generation: generation,
                });
                if let Some(p) = previous {
                    p.successor(&m).map_err(|_| ObjectError::InvalidVersion)?;
                }
                let mut encoded = [0; MANIFEST_BYTES];
                m.encode(&mut encoded)
                    .map_err(|_| ObjectError::InvalidObject)?;
                if previous.is_none() {
                    catalog.entries[slot] = Some(Entry {
                        object,
                        backing,
                        owner,
                        scope,
                        nonce: u.nonce,
                        creation_hash: u.hash,
                        creation_policy: u.policy.replicas() as u8,
                    });
                }
                let mut receipt = u.clone();
                receipt.result = object;
                receipt.result_generation = m.generation;
                receipt.complete = true;
                committed = Some(m);
                Ok((catalog.encode(), encoded, receipt.encode(), audit_record(previous.as_ref(),&m,5)))
            },
        )?;
        *verifier = UploadVerifier::new();
        Ok(committed)
    }
    // ------------------------=
    // FUNC: pool_read_index
    // DESC: Validates the immutable index and verifies only the bounded referenced extent before returning a requested range.
    // ------------------=
    pub(super) fn pool_read_index(
        &mut self,
        m: &Manifest,
        index: &[u8],
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), ObjectError> {
        if u32::from_le_bytes(index[8..12].try_into().unwrap()) as u64 != m.length {
            return Err(ObjectError::CorruptContent);
        }
        let mut done = 0;
        // Copy checks immutable index ownership without scanning the entire object
        // in the UI loop; every subsequent non-empty read checks its extent.
        for (i, chunk) in m
            .chunks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.map(|c| (i, c)))
        {
            if i >= 16 || index[16 + i * 16..32 + i * 16] != chunk.content {
                return Err(ObjectError::CorruptContent);
            }
            if out.is_empty() {
                continue;
            }
            let start = (i * EXTENT) as u64;
            let end = start + chunk.bytes as u64;
            if !out.is_empty() && (offset >= end || offset + out.len() as u64 <= start) {
                continue;
            }
            let mut bytes = [0; EXTENT];
            let mut at = 0;
            while at < chunk.bytes as usize {
                let n = (chunk.bytes as usize - at).min(1024);
                self.read_extent_range(ObjectId(chunk.content), at as u64, &mut bytes[at..at + n])?;
                at += n;
            }
            if <[u8; 32]>::from(Sha256::digest(&bytes[..at])) != chunk.hash {
                return Err(ObjectError::CorruptContent);
            }
            if !out.is_empty() {
                let from = offset.max(start);
                let to = (offset + out.len() as u64).min(end);
                let count = (to - from) as usize;
                out[(from - offset) as usize..(to - offset) as usize].copy_from_slice(
                    &bytes[(from - start) as usize..(from - start) as usize + count],
                );
                done += count;
            }
        }
        if done != out.len() {
            return Err(ObjectError::CorruptContent);
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: pool_index_references
    // DESC: Enumerates distinct immutable extent identities across all retained versions of one application object.
    // ------------------=
    fn pool_index_references(
        &mut self,
        object: ObjectId,
        out: &mut [ObjectId; 52],
    ) -> Result<usize, ObjectError> {
        let mut versions = [0; 64];
        let count = self.pool_version_numbers(object, &mut versions);
        let mut used = 0;
        for version in &versions[..count] {
            let mut bytes = [0; MAX_CONTENT];
            let len = self.read(object, Some(*version), &mut bytes)?;
            if len != INDEX_BYTES || &bytes[..8] != b"INFPIDX1" {
                continue;
            }
            for i in 0..16 {
                let id = ObjectId(bytes[16 + i * 16..32 + i * 16].try_into().unwrap());
                if id.0 != [0; 16] && !out[..used].contains(&id) {
                    if used == out.len() {
                        return Err(ObjectError::InsufficientCapacity);
                    }
                    out[used] = id;
                    used += 1;
                }
            }
        }
        Ok(used)
    }
    // ------------------------=
    // FUNC: pool_delete
    // DESC: Removes one generation-fenced object and reclaims only extents unreachable from all remaining live objects and their retained versions.
    // ------------------=
    pub(crate) fn pool_delete(
        &mut self,
        object: ObjectId,
        owner: NodeId,
        scope: u64,
        expected: u64,
    ) -> Result<(), ObjectError> {
        let manifest = self.pool_manifest(object, owner, scope)?;
        self.require_pool_mutation_authority(object)?;
        if manifest.generation != expected || manifest.healing.is_some() {
            return Err(ObjectError::InvalidVersion);
        }
        let mut catalog = Catalog::load(self)?;
        let slot = catalog
            .entries
            .iter()
            .position(|e| e.is_some_and(|e| e.object == object))
            .ok_or(ObjectError::NotFound)?;
        let backing = catalog.entries[slot].unwrap().backing;
        let mut candidates = [ObjectId([0; 16]); 52];
        let count = self.pool_index_references(object, &mut candidates)?;
        catalog.entries[slot] = None;
        let mut owners = [ObjectId([0; 16]); 52];
        let owners_count = self.pool_index_objects(&mut owners);
        for other in owners[..owners_count].iter().filter(|id| **id != object) {
            let mut references = [ObjectId([0; 16]); 52];
            let n = self.pool_index_references(*other, &mut references)?;
            for candidate in &mut candidates[..count] {
                if references[..n].contains(candidate) {
                    *candidate = ObjectId([0; 16]);
                }
            }
        }
        let mut retired = [ObjectId([0; 16]); 54];
        retired[0] = object;
        retired[1] = backing;
        retired[2..].copy_from_slice(&candidates);
        let (outbox, pending) = self.pool_stage_deletion(&manifest, scope)?;
        // Outbox initialization may have atomically added its catalog child reference.
        catalog.deletion = Catalog::load(self)?.deletion;
        self.pool_retire_owned_with_state(&retired, catalog.id, &catalog.encode(), Some((outbox, &pending)),Some(audit_record(Some(&manifest),&manifest,6)))
    }
}
