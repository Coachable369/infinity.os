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
struct Catalog { id: ObjectId, entries: [Option<Binding>; MAX_BINDINGS] }
impl Catalog {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes bounded native ownership records with full application identities and independent physical checkpoint references.
    // ------------------=
    fn encode(&self) -> [u8; CATALOG_BYTES] {
        let mut bytes = [0; CATALOG_BYTES]; bytes[..8].copy_from_slice(b"INFREP01");
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
                let empty = Self { id: ObjectId([0; 16]), entries: [None; MAX_BINDINGS] };
                store.create_attached(b"replica-authority", ObjectType::Metadata, Space::System,
                    &empty.encode(), PATH).map_err(storage_error)?
            },
            Err(error) => return Err(storage_error(error)),
        };
        let mut bytes = [0; CATALOG_BYTES];
        if store.read(id, None, &mut bytes).map_err(storage_error)? != CATALOG_BYTES
            || &bytes[..8] != b"INFREP01" || bytes[8..32] != [0; 24] { return Err(RemoteError::PersistenceFailed); }
        let mut catalog = Self { id, entries: [None; MAX_BINDINGS] };
        for index in 0..MAX_BINDINGS {
            let at = 32 + index * 208;
            if bytes[at..at+208].iter().all(|b| *b == 0) { continue; }
            let cp = decode_header(&bytes[at..at+128]).map_err(replica_error)?;
            let b = Binding { backing: ObjectId(bytes[at+128..at+144].try_into().unwrap()),
                owner: bytes[at+144..at+176].try_into().unwrap(),
                authority: field(&bytes, at+176), manifest: field(&bytes, at+184), scope: field(&bytes, at+192), descriptor: cp.descriptor };
            if cp.state != ReplicaState::Planned || cp.copied != 0 || b.backing.0 == [0; 16]
                || b.owner == [0; 32] || b.authority == 0 || b.manifest == 0 || bytes[at+200..at+208] != [0; 8]
                || catalog.entries.iter().flatten().any(|old| old.descriptor.object == b.descriptor.object
                    || old.backing == b.backing) { return Err(RemoteError::PersistenceFailed); }
            catalog.entries[index] = Some(b);
        }
        Ok(catalog)
    }
}

pub(crate) struct ReplicaService {
    resource: ResourceId, generation: u64, verifiers: [Option<ExtentVerification>; MAX_BINDINGS],
}
impl ReplicaService {
    // ------------------------=
    // FUNC: mount
    // DESC: Validates or initializes the native catalog before the runtime can announce this recipient service ready.
    // ------------------=
    pub(crate) fn mount<D: BlockDevice>(store: &mut ObjectStore<D>, resource: ResourceId,
        generation: u64) -> Result<Self, RemoteError> {
        if resource.0 == [0; 16] || generation == 0 { return Err(RemoteError::InvalidState); }
        Catalog::load(store)?;
        Ok(Self::new(resource, generation))
    }
    // ------------------------=
    // FUNC: new
    // DESC: Binds the native recipient to an actually discovered container identity and incarnation; no remote request selects the physical device.
    // ------------------=
    pub(crate) const fn new(resource: ResourceId, generation: u64) -> Self {
        Self { resource, generation, verifiers: [const { None }; MAX_BINDINGS] }
    }
    // ------------------------=
    // FUNC: execute
    // DESC: Revalidates persisted object authority on every authenticated operation and returns only durably committed copy state or observed verification progress.
    // ------------------=
    pub(crate) fn execute<D: BlockDevice>(&mut self, store: &mut ObjectStore<D>, request: AuthenticatedStorageRequest)
        -> Result<StorageOperationV1, RemoteError> {
        let p = request.payload;
        p.encode().map_err(|_| RemoteError::MalformedRequest)?;
        if !matches!(p.operation, Operation::TransferBegin | Operation::TransferChunk | Operation::TransferCommit | Operation::ReplicaInspect) {
            return Err(RemoteError::UnsupportedOperation);
        }
        if request.peer.0 == [0; 32] || p.object == [0; 16] || p.authority_generation == 0
            || p.manifest_generation == 0 || p.object_version == 0 || p.value == 0
            || self.resource.0 == [0; 16] || self.generation == 0 { return Err(RemoteError::MalformedRequest); }
        let mut catalog = Catalog::load(store)?;
        let existing = catalog.entries.iter().position(|b| b.is_some_and(|b| b.descriptor.object == p.object));
        let index = if let Some(index) = existing {
            let b = catalog.entries[index].unwrap();
            if b.owner != request.peer.0 || b.authority != p.authority_generation || b.scope != p.scope {
                return Err(RemoteError::AccessDenied);
            }
            if b.manifest != p.manifest_generation || b.descriptor.version != p.object_version
                || b.descriptor.job != p.value || b.descriptor.resource != self.resource
                || b.descriptor.generation != self.generation { return Err(RemoteError::Conflict); }
            index
        } else {
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
