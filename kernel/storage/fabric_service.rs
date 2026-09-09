//! Durable recipient-side replica authority. The shared IOP router authenticates
//! every call before entry; this service additionally fences the full object,
//! immutable version, scope, owner and generations. Unsupported Pool operations
//! are explicit errors, not fabricated successes.
use super::*;
use super::extent::{ExtentVerification, NativeExtentReplica};
use crate::runtime::iop::{remote::{AuthenticatedStorageRequest, RemoteError},
    storage_protocol::{Operation, StorageOperationV1}};
use crate::storage::object::{ObjectError, ObjectType, Space};

pub(crate) const MAX_BINDINGS: usize = 4;
const CATALOG_BYTES: usize = 32 + MAX_BINDINGS * 208;
const PATH: &[u8] = b"/system/storage/replicas";

#[derive(Clone, Copy, PartialEq, Eq)]
struct Binding {
    backing: ObjectId, owner: [u8; 32], authority: u64, manifest: u64, scope: u64,
    descriptor: ReplicaDescriptor,
}
struct Catalog { id: ObjectId, epoch: u64, entries: [Option<Binding>; MAX_BINDINGS] }
impl Catalog {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes bounded native ownership records with full application identities and independent physical checkpoint references.
    // ------------------=
    fn encode(&self) -> [u8; CATALOG_BYTES] {
        let mut bytes = [0; CATALOG_BYTES]; bytes[..8].copy_from_slice(b"INFREP01");
        bytes[8..16].copy_from_slice(&self.epoch.to_le_bytes());
        for (index, binding) in self.entries.iter().enumerate() {
            if let Some(b) = binding {
                let at = 32 + index * 208;
                bytes[at..at+128].copy_from_slice(&{
                    let mut header = [0; 128];
                    encode(&Checkpoint { descriptor: b.descriptor, copied: 0, state: ReplicaState::Planned }, &mut header); header
                });
                bytes[at+128..at+144].copy_from_slice(&b.backing.0);
                bytes[at+144..at+176].copy_from_slice(&b.owner);
                for (offset, value) in [(176, b.authority), (184, b.manifest), (192, b.scope)] {
                    bytes[at+offset..at+offset+8].copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        bytes
    }
    // ------------------------=
    // FUNC: load
    // DESC: Reads or creates the native recipient catalog; malformed committed state fails closed rather than resetting ownership.
    // ------------------=
    fn load<D: BlockDevice>(store: &mut ObjectStore<D>) -> Result<Self, RemoteError> {
        let id = match store.resolve(PATH) {
            Ok(id) => id,
            Err(ObjectError::NotFound | ObjectError::NamespaceNotFound) => {
                let empty = Self { id: ObjectId([0; 16]), epoch: 0, entries: [None; MAX_BINDINGS] };
                store.create_attached(b"replica-authority", ObjectType::Metadata, Space::System,
                    &empty.encode(), PATH).map_err(storage_error)?
            },
            Err(error) => return Err(storage_error(error)),
        };
        let mut bytes = [0; CATALOG_BYTES];
        if store.read(id, None, &mut bytes).map_err(storage_error)? != CATALOG_BYTES
            || &bytes[..8] != b"INFREP01" || bytes[16..32] != [0; 16] { return Err(RemoteError::PersistenceFailed); }
        let mut catalog = Self { id, epoch: field(&bytes, 8), entries: [None; MAX_BINDINGS] };
        for index in 0..MAX_BINDINGS {
            let at = 32 + index * 208;
            if bytes[at..at+208].iter().all(|b| *b == 0) { continue; }
            let cp = decode_header(&bytes[at..at+128]).map_err(replica_error)?;
            let b = Binding { backing: ObjectId(bytes[at+128..at+144].try_into().unwrap()),
                owner: bytes[at+144..at+176].try_into().unwrap(),
                authority: field(&bytes, at+176), manifest: field(&bytes, at+184), scope: field(&bytes, at+192), descriptor: cp.descriptor };
            if cp.state != ReplicaState::Planned || cp.copied != 0 || b.backing.0 == [0; 16]
                || b.owner == [0; 32] || b.authority == 0 || b.manifest == 0 || bytes[at+200..at+208] != [0; 8]
                || catalog.entries.iter().flatten().any(|old| old.backing == b.backing
                    || (old.descriptor.object == b.descriptor.object
                        && (old.owner != b.owner || old.authority != b.authority || old.scope != b.scope
                            || old.descriptor.version == b.descriptor.version
                            || (old.descriptor.version < b.descriptor.version) != (old.manifest < b.manifest)
                            || old.manifest == b.manifest))) { return Err(RemoteError::PersistenceFailed); }
            catalog.entries[index] = Some(b);
        }
        Ok(catalog)
    }
}

pub(crate) struct ReplicaService {
    resource: ResourceId, generation: u64, verifiers: [Option<ExtentVerification>; MAX_BINDINGS],
    device: Option<[u8; 16]>, sequence: u64,
}
impl ReplicaService {
    // ------------------------=
    // FUNC: mount
    // DESC: Validates or initializes the native catalog before the runtime can announce this recipient service ready.
    // ------------------=
    pub(crate) fn mount<D: BlockDevice>(store: &mut ObjectStore<D>, resource: ResourceId,
        generation: u64) -> Result<Self, RemoteError> {
        if resource.0 == [0; 16] || generation == 0 { return Err(RemoteError::InvalidState); }
        store.initialize_pool_catalog().map_err(storage_error)?;
        let mut catalog = Catalog::load(store)?;
        catalog.epoch = catalog.epoch.checked_add(1).filter(|n| *n <= u32::MAX as u64)
            .ok_or(RemoteError::PersistenceFailed)?;
        store.replace_state(catalog.id, &catalog.encode()).map_err(storage_error)?;
        let mut service = Self::new(resource, generation); service.sequence = catalog.epoch << 32;
        Ok(service)
    }
    // ------------------------=
    // FUNC: new
    // DESC: Binds the native recipient to an actually discovered container identity and incarnation; no remote request selects the physical device.
    // ------------------=
    pub(crate) const fn new(resource: ResourceId, generation: u64) -> Self {
        Self { resource, generation, verifiers: [const { None }; MAX_BINDINGS], device: None, sequence: 0 }
    }
    // ------------------------=
    // FUNC: attach_device_identity
    // DESC: Supplies a disk identity obtained by native hardware discovery; without it storage advertisements remain unsupported.
    // ------------------=
    pub(crate) fn attach_device_identity(&mut self, identity: Option<[u8; 16]>) {
        self.device = identity.filter(|id| *id != [0; 16]);
    }
    // ------------------------=
    // FUNC: resource_observation
    // DESC: Measures mounted native capacity and committed reservations with a reboot-monotonic sequence; no host shell or invented device metrics are used.
    // ------------------=
    fn resource_observation<D: BlockDevice>(&mut self, store: &ObjectStore<D>, p: StorageOperationV1)
        -> Result<StorageOperationV1, RemoteError> {
        let device = self.device.ok_or(RemoteError::UnsupportedOperation)?;
        if p.length != 0 || (p.object != [0; 16] && p.object != self.resource.0) { return Err(RemoteError::NotFound); }
        if self.sequence == 0 || self.sequence as u32 == u32::MAX { return Err(RemoteError::ServiceUnavailable); }
        self.sequence += 1;
        let mut reply = p; reply.data = [0; 64]; reply.length = 48;
        reply.authority_generation = self.generation; reply.manifest_generation = self.sequence;
        reply.object_version = store.total_blocks() as u64 * 4096;
        reply.offset = (store.total_blocks().saturating_sub(store.usage_blocks())) as u64 * 4096;
        reply.value = 0;
        reply.data[..16].copy_from_slice(&self.resource.0); reply.data[16..32].copy_from_slice(&device);
        reply.data[32..40].copy_from_slice(&store.staging_reserved_bytes().to_le_bytes());
        reply.data[40..44].copy_from_slice(&1u32.to_le_bytes()); reply.data[44] = 1; reply.data[45] = 1;
        reply.data[46..48].copy_from_slice(&1u16.to_le_bytes());
        Ok(reply)
    }
    // ------------------------=
    // FUNC: execute
    // DESC: Revalidates persisted object authority on every authenticated operation and returns only durably committed copy state or observed verification progress.
    // ------------------=
    pub(crate) fn execute<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>, request: AuthenticatedStorageRequest)
        -> Result<StorageOperationV1, RemoteError> {
        let p = request.payload;
        p.encode().map_err(|_| RemoteError::MalformedRequest)?;
        if p.operation == Operation::ResourceInspect { return self.resource_observation(store, p); }
        if matches!(p.operation, Operation::PoolInspect | Operation::ObjectCreate | Operation::ObjectInspect | Operation::ObjectSetPolicy | Operation::ObjectUpdate | Operation::ObjectCopy)
            || (p.operation == Operation::ObjectRead && p.length == 0) {
            return self.pool_operation(store, request);
        }
        if !matches!(p.operation, Operation::TransferBegin | Operation::TransferChunk | Operation::TransferCommit | Operation::ReplicaInspect | Operation::ObjectRead) {
            return Err(RemoteError::UnsupportedOperation);
        }
        if request.peer.0 == [0; 32] || p.object == [0; 16] || p.authority_generation == 0
            || p.manifest_generation == 0 || p.object_version == 0
            || (p.value == 0 && p.operation != Operation::ReplicaInspect)
            || self.resource.0 == [0; 16] || self.generation == 0 { return Err(RemoteError::MalformedRequest); }
        let mut catalog = Catalog::load(store)?;
        let known = catalog.entries.iter().flatten().filter(|b| b.descriptor.object == p.object);
        // Versions retain distinct immutable physical bindings. A new version
        // cannot overwrite the old committed copy or seize its authority.
        for b in known.clone() {
            if b.owner != request.peer.0 || b.authority != p.authority_generation || b.scope != p.scope {
                return Err(RemoteError::AccessDenied);
            }
        }
        let existing = catalog.entries.iter().position(|b| b.is_some_and(|b|
            b.descriptor.object == p.object && b.descriptor.version == p.object_version));
        let index = if let Some(index) = existing {
            let b = catalog.entries[index].unwrap();
            if b.owner != request.peer.0 || b.authority != p.authority_generation || b.scope != p.scope {
                return Err(RemoteError::AccessDenied);
            }
            if b.manifest != p.manifest_generation || b.descriptor.version != p.object_version
                || (!matches!(p.operation, Operation::ObjectRead | Operation::ReplicaInspect) && b.descriptor.job != p.value)
                || b.descriptor.resource != self.resource
                || b.descriptor.generation != self.generation { return Err(RemoteError::Conflict); }
            index
        } else {
            if let Some(latest) = known.max_by_key(|b| b.descriptor.version) {
                if p.operation != Operation::TransferBegin || p.object_version <= latest.descriptor.version
                    || p.manifest_generation <= latest.manifest { return Err(RemoteError::Conflict); }
            }
            if p.operation != Operation::TransferBegin { return Err(RemoteError::NotFound); }
            let descriptor = self.begin_descriptor(p)?;
            let index = catalog.entries.iter().position(Option::is_none).ok_or(RemoteError::QueueFull)?;
            let cp = Checkpoint { descriptor, copied: 0, state: ReplicaState::Planned };
            let mut binding = Binding { backing: ObjectId([0; 16]), owner: request.peer.0,
                authority: p.authority_generation, manifest: p.manifest_generation, scope: p.scope, descriptor };
            store.create_replica_binding(catalog.id, descriptor.bytes as u32, |backing, extent| {
                binding.backing = backing; catalog.entries[index] = Some(binding);
                (catalog.encode(), NativeExtentReplica::<D>::record(&cp, extent))
            }).map_err(storage_error)?;
            self.verifiers[index] = None;
            index
        };
        let binding = catalog.entries[index].unwrap();
        if p.operation == Operation::ObjectRead {
            if p.length != 32 || !(1..=64).contains(&p.value)
                || p.offset.checked_add(p.value).is_none_or(|end| end > binding.descriptor.bytes) {
                return Err(RemoteError::MalformedRequest);
            }
            let base = p.offset / 1024 * 1024;
            let size = (binding.descriptor.bytes - base).min(1024) as usize;
            let within = (p.offset - base) as usize;
            if within + p.value as usize > size { return Err(RemoteError::MalformedRequest); }
            let mut native = NativeExtentReplica::open(store, binding.backing, self.resource, self.generation).map_err(replica_error)?;
            let mut chunk = [0; 1024];
            native.read_verified_chunk(base, &mut chunk[..size], p.data[..32].try_into().unwrap()).map_err(replica_error)?;
            let mut response = p; response.data = [0; 64]; response.length = p.value as u16;
            response.data[..p.value as usize].copy_from_slice(&chunk[within..within+p.value as usize]);
            return Ok(response);
        }
        match p.operation {
            Operation::TransferBegin => {
                if self.begin_descriptor(p)? != binding.descriptor { return Err(RemoteError::Conflict); }
            },
            Operation::TransferChunk => {
                if p.length == 0 { return Err(RemoteError::MalformedRequest); }
                let mut native = NativeExtentReplica::open(store, binding.backing, self.resource, self.generation).map_err(replica_error)?;
                let mut transfer = Transfer::resume(native.inspect().ok_or(RemoteError::InvalidState)?).map_err(replica_error)?;
                transfer.receive(&mut native, p.offset, &p.data[..p.length as usize]).map_err(replica_error)?;
            },
            Operation::TransferCommit => {
                if p.length != 0 { return Err(RemoteError::MalformedRequest); }
                if self.verifiers[index].is_none() {
                    self.verifiers[index] = Some(ExtentVerification::resume(store, binding.backing,
                        self.resource, self.generation).map_err(replica_error)?);
                }
                self.verifiers[index].as_mut().unwrap().tick(store).map_err(replica_error)?;
            },
            Operation::ReplicaInspect => { if p.length != 0 { return Err(RemoteError::MalformedRequest); } },
            _ => return Err(RemoteError::UnsupportedOperation),
        }
        let native = NativeExtentReplica::open(store, binding.backing, self.resource, self.generation).map_err(replica_error)?;
        let cp = native.inspect().ok_or(RemoteError::InvalidState)?;
        let mut response = p; response.data = [0; 64]; response.length = 49;
        response.value = cp.descriptor.job;
        response.offset = cp.copied;
        response.data[0] = match cp.state { ReplicaState::Planned => 1, ReplicaState::Copying => 2,
            ReplicaState::Verifying => 3, ReplicaState::Available => 4, ReplicaState::Failed => 5 };
        response.data[1..9].copy_from_slice(&cp.descriptor.bytes.to_le_bytes());
        response.data[9..41].copy_from_slice(&cp.descriptor.hash);
        let verified = if cp.state == ReplicaState::Available { cp.descriptor.bytes }
            else { self.verifiers[index].as_ref().map(|v| v.verified_bytes()).unwrap_or(0) };
        response.data[41..49].copy_from_slice(&verified.to_le_bytes());
        Ok(response)
    }
    // ------------------------=
    // FUNC: pool_operation
    // DESC: Executes authenticated application creation, canonical manifest inspection and generation-fenced policy mutation against the durable native Pool catalog.
    // ------------------=
    fn pool_operation<D: BlockDevice>(&self, store: &mut ObjectStore<D>, request: AuthenticatedStorageRequest)
        -> Result<StorageOperationV1, RemoteError> {
        use crate::runtime::fabric::{manifest::MANIFEST_BYTES, placement::{StorageClass, Availability}};
        let p = request.payload;
        if request.local.0 == [0; 32] || request.peer.0 == [0; 32] { return Err(RemoteError::AccessDenied); }
        if p.operation == Operation::PoolInspect {
            if p.length != 0 || p.object != [0; 16] || p.offset > 8 || p.value != 0 {
                return Err(RemoteError::MalformedRequest);
            }
            if p.manifest_generation != 0 && p.manifest_generation != store.generation() { return Err(RemoteError::Conflict); }
            let (count, manifest) = store.pool_inspect(request.peer, p.scope, p.offset as usize).map_err(storage_error)?;
            let mut response = p; response.data = [0; 64]; response.length = 0;
            response.value = count as u64; response.manifest_generation = store.generation();
            if let Some(m) = manifest {
                response.data[..16].copy_from_slice(&m.object); response.data[16..48].copy_from_slice(&m.hash);
                response.data[48] = m.policy.replicas() as u8;
                response.data[49] = match m.availability() { Availability::Healthy => 1, Availability::Degraded => 2, Availability::Offline => 3 };
                response.data[50] = m.minimum_available;
                response.data[56..64].copy_from_slice(&m.length.to_le_bytes());
                response.length = 64; response.object_version = m.version;
                response.authority_generation = m.authority_generation;
            }
            return Ok(response);
        }
        let policy = match p.value { 1 => Some(StorageClass::Temporary), 2 => Some(StorageClass::Protected),
            3 => Some(StorageClass::Critical), _ => None };
        let manifest = match p.operation {
            Operation::ObjectCreate => {
                if p.object != [0; 16] || p.object_version != 0 || p.manifest_generation != 0
                    || p.authority_generation != 1 || p.offset == 0 { return Err(RemoteError::MalformedRequest); }
                store.pool_create(request.peer, p.scope, p.offset, policy.ok_or(RemoteError::MalformedRequest)?,
                    &p.data[..p.length as usize], request.local, self.resource,
                    self.device.ok_or(RemoteError::ServiceUnavailable)?, self.generation).map_err(storage_error)?
            },
            Operation::ObjectInspect | Operation::ObjectSetPolicy | Operation::ObjectUpdate | Operation::ObjectRead | Operation::ObjectCopy => {
                if p.operation != Operation::ObjectUpdate && p.length != 0 { return Err(RemoteError::MalformedRequest); }
                let current = store.pool_manifest(ObjectId(p.object), request.peer, p.scope).map_err(storage_error)?;
                if current.authority_generation != p.authority_generation
                    || (p.manifest_generation != 0 && current.generation != p.manifest_generation)
                    || (p.object_version != 0 && current.version != p.object_version) { return Err(RemoteError::Conflict); }
                if p.operation == Operation::ObjectCopy {
                    if p.offset != 0 || p.manifest_generation == 0 || p.object_version == 0 || p.value == 0 {
                        return Err(RemoteError::MalformedRequest);
                    }
                    store.pool_copy(ObjectId(p.object), request.peer, p.scope, p.manifest_generation,
                        p.value, request.local, self.resource, self.device.ok_or(RemoteError::ServiceUnavailable)?,
                        self.generation).map_err(storage_error)?
                } else if matches!(p.operation, Operation::ObjectSetPolicy | Operation::ObjectUpdate) {
                    if p.offset != 0 || p.manifest_generation == 0 || p.object_version == 0 {
                        return Err(RemoteError::MalformedRequest);
                    }
                    if p.operation == Operation::ObjectSetPolicy {
                        store.pool_set_policy(ObjectId(p.object), request.peer, p.scope, p.manifest_generation,
                            policy.ok_or(RemoteError::MalformedRequest)?).map_err(storage_error)?
                    } else {
                        if p.value != 0 { return Err(RemoteError::MalformedRequest); }
                        store.pool_update(ObjectId(p.object), request.peer, p.scope, p.manifest_generation,
                            &p.data[..p.length as usize], request.local, self.resource,
                            self.device.ok_or(RemoteError::ServiceUnavailable)?, self.generation).map_err(storage_error)?
                    }
                } else { current }
            },
            _ => return Err(RemoteError::UnsupportedOperation),
        };
        let mut response = p; response.data = [0; 64];
        response.manifest_generation = manifest.generation;
        response.authority_generation = manifest.authority_generation;
        response.object_version = manifest.version;
        if p.operation == Operation::ObjectRead {
            if p.value > 64 || p.object_version == 0 || p.manifest_generation == 0 { return Err(RemoteError::MalformedRequest); }
            store.pool_read(&manifest, p.offset, &mut response.data[..p.value as usize]).map_err(storage_error)?;
            response.length = p.value as u16;
        } else if p.operation == Operation::ObjectInspect {
            if p.value == 0 || p.value > 64 || p.offset >= MANIFEST_BYTES as u64 {
                return Err(RemoteError::MalformedRequest);
            }
            let mut bytes = [0; MANIFEST_BYTES]; manifest.encode(&mut bytes).map_err(|_| RemoteError::PersistenceFailed)?;
            let length = (MANIFEST_BYTES - p.offset as usize).min(p.value as usize);
            response.data[..length].copy_from_slice(&bytes[p.offset as usize..p.offset as usize+length]);
            response.length = length as u16; response.value = MANIFEST_BYTES as u64;
        } else {
            response.data[..16].copy_from_slice(&manifest.object);
            response.data[16..48].copy_from_slice(&manifest.hash);
            response.data[48] = manifest.policy.replicas() as u8;
            response.data[49] = match manifest.availability() { Availability::Healthy => 1, Availability::Degraded => 2, Availability::Offline => 3 };
            response.length = 50; response.value = manifest.length;
        }
        Ok(response)
    }
    // ------------------------=
    // FUNC: begin_descriptor
    // DESC: Validates exact resource, incarnation, expected hash and length before any recipient capacity can be reserved.
    // ------------------=
    fn begin_descriptor(&self, p: StorageOperationV1) -> Result<ReplicaDescriptor, RemoteError> {
        if p.length != 56 || p.offset > 1024 * 1024 { return Err(RemoteError::MalformedRequest); }
        if p.data[..16] != self.resource.0 || field(&p.data, 16) != self.generation { return Err(RemoteError::Conflict); }
        Ok(ReplicaDescriptor { job: p.value, object: p.object, version: p.object_version,
            resource: self.resource, generation: self.generation, bytes: p.offset,
            hash: p.data[24..56].try_into().unwrap() })
    }
}

// ------------------------=
// FUNC: field
// DESC: Decodes one fixed field only after its bounded record length is known.
// ------------------=
fn field(bytes: &[u8], at: usize) -> u64 { u64::from_le_bytes(bytes[at..at+8].try_into().unwrap()) }
// ------------------------=
// FUNC: storage_error
// DESC: Maps native persistence failures to typed IOP errors without leaking device details.
// ------------------=
fn storage_error(error: ObjectError) -> RemoteError {
    match error { ObjectError::InsufficientCapacity => RemoteError::QueueFull,
        ObjectError::NotFound | ObjectError::NamespaceNotFound => RemoteError::NotFound,
        ObjectError::Unauthorized => RemoteError::AccessDenied,
        ObjectError::InvalidVersion => RemoteError::Conflict,
        ObjectError::InvalidObject => RemoteError::MalformedRequest,
        _ => RemoteError::PersistenceFailed }
}
// ------------------------=
// FUNC: replica_error
// DESC: Preserves authority, integrity, conflict and storage failure boundaries in the remote service contract.
// ------------------=
fn replica_error(error: ReplicaError) -> RemoteError {
    match error { ReplicaError::AccessDenied => RemoteError::AccessDenied,
        ReplicaError::Conflict | ReplicaError::Stale => RemoteError::Conflict,
        ReplicaError::Invalid => RemoteError::MalformedRequest,
        ReplicaError::Incomplete => RemoteError::InvalidState,
        ReplicaError::Storage => RemoteError::PersistenceFailed,
        ReplicaError::Integrity => RemoteError::RemoteFailure }
}
