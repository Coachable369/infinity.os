//! Shared human-interface broker for authoritative mounted storage inspection.
//! Queries use normal typed IOP queues and ephemeral narrow capabilities.
use super::*;
use identity::{StableId, SessionState, MAX_SESSIONS, SESSION_IDENTITY_MANAGE};
use iop::{IopError, IopMessage, OperationId, storage_protocol::{Operation, StorageOperationV1}};

pub enum Submission { Complete(StorageOperationV1), Pending(u64) }
// ------------------------=
// FUNC: submit
// DESC: Offers applications the ordinary object read contract while the service selects local or fresh replicated metadata resolution.
// ------------------=
pub fn submit(user:StableId,session:StableId,request:StorageOperationV1)->Result<Submission,iop::remote::RemoteError>{
    if matches!(request.operation,Operation::ObjectRead|Operation::ObjectInspect)&&(storage_metadata::bound(request.object)||storage_metadata::warming()){storage_metadata::read(user,session,request).map(Submission::Pending)}
    else if matches!(request.operation,Operation::ObjectUpdate|Operation::ObjectSetPolicy|Operation::ObjectDelete|Operation::ObjectCopy)&&storage_metadata::bound(request.object){storage_metadata::mutate(user,session,request).map(Submission::Pending)}
    else{execute(user,session,request).map(Submission::Complete).map_err(|_|iop::remote::RemoteError::RemoteFailure)}
}

// ------------------------=
// FUNC: query
// DESC: Authenticates the complete operator session before inspecting storage through the same broker for Console and Settings.
// ------------------=
pub fn query(user: StableId, session: StableId) -> Result<StorageOperationV1, IopError> {
    with_runtime(|runtime| query_from(runtime, user, session)).ok_or(IopError::UnknownEndpoint)?
}

// ------------------------=
// FUNC: execute
// DESC: Allows an authenticated operator to use the shared bounded storage IOP broker; neither Console nor GUI may call the native backend directly.
// ------------------=
pub fn execute(user: StableId, session: StableId, request: StorageOperationV1) -> Result<StorageOperationV1, IopError> {
    with_runtime(|runtime| {
        authorize(runtime, user, session)?;
        // Shared reads must use the asynchronous broker: a cached local copy is not a fresh quorum proof.
        if matches!(request.operation,Operation::ObjectRead|Operation::ObjectInspect|Operation::ObjectUpdate|Operation::ObjectSetPolicy|Operation::ObjectDelete|Operation::ObjectCopy)&&storage_metadata::bound_from(runtime,request.object){return Err(IopError::InvalidPayload)}
        let now = runtime.node_clock.ok_or(IopError::DeadlineExceeded)?;
        let result = perform(runtime, now, request);
        runtime.storage_last_observation = result.as_ref().ok().copied();
        result
    }).ok_or(IopError::UnknownEndpoint)?
}

// ------------------------=
// FUNC: authorize
// DESC: Requires the complete current privileged operator session for native Pool configuration and metadata inspection.
// ------------------=
pub(super) fn authorize(runtime: &InfinityRuntime, user: StableId, session: StableId) -> Result<(), IopError> {
    let active = (0..MAX_SESSIONS).filter_map(|index| runtime.identity.session_nth(index)).any(|candidate|
        candidate.id == session && candidate.user == user && candidate.state == SessionState::Active
            && candidate.capabilities & SESSION_IDENTITY_MANAGE != 0);
    if active { Ok(()) } else { Err(IopError::AccessDenied) }
}

// ------------------------=
// FUNC: query_from
// DESC: Runs the identical authenticated broker against an exclusively borrowed runtime for installed operation and behavioral verification.
// ------------------=
fn query_from(runtime: &mut InfinityRuntime, user: StableId, session: StableId) -> Result<StorageOperationV1, IopError> {
        authorize(runtime, user, session)?;
        let now = runtime.node_clock.ok_or(IopError::DeadlineExceeded)?;
        let result = read(runtime, now);
        runtime.storage_last_observation = result.as_ref().ok().copied();
        result
}

// ------------------------=
// FUNC: read
// DESC: Performs canonical capability-validated request, dequeue, execution and correlated reply without a GUI-only database or shell parsing.
// ------------------=
pub(super) fn read(runtime: &mut InfinityRuntime, now: u64) -> Result<StorageOperationV1, IopError> {
    perform(runtime, now, StorageOperationV1 { operation: Operation::ResourceInspect, object: [0; 16],
        authority_generation: 0, manifest_generation: 0, object_version: 0,
        offset: 0, scope: 0, value: 0, length: 0, data: [0; 64] })
}

// ------------------------=
// FUNC: perform
// DESC: Routes one exact operation through owned endpoints, live capability validation and a correlated reply; committed mutations publish through ordinary IEF authority.
// ------------------=
pub(super) fn perform(runtime: &mut InfinityRuntime, now: u64, request: StorageOperationV1) -> Result<StorageOperationV1, IopError> {
    let operation = match request.operation {
        Operation::ResourceInspect => OperationId::ResourceInspect,
        Operation::ObjectCreate => OperationId::ObjectCreate,
        Operation::ObjectCopy => OperationId::ObjectCopy,
        Operation::ObjectInspect => OperationId::ObjectInspect,
        Operation::ObjectRead => OperationId::ObjectRead,
        Operation::ObjectUpdate => OperationId::ObjectUpdate,
        Operation::ObjectSetPolicy => OperationId::ObjectSetPolicy,
        Operation::PoolInspect => OperationId::PoolInspect,
        Operation::ObjectDelete => OperationId::ObjectDelete,
        Operation::PoolUploadBegin => OperationId::PoolUploadBegin,
        Operation::PoolUploadAppend => OperationId::PoolUploadAppend,
        Operation::PoolUploadCommit => OperationId::PoolUploadCommit,
        Operation::PoolUploadAbort => OperationId::PoolUploadAbort,
        _ => return Err(IopError::InvalidPayload),
    };
    if runtime.services.inspect(SERVICE_REPLICA_STORAGE).is_none_or(|s| s.state != ServiceState::Ready) {
        return Err(IopError::UnknownEndpoint);
    }
    let handler = runtime.storage_handler.ok_or(IopError::UnknownEndpoint)?;
    let local = runtime.nodes.local_id().ok_or(IopError::UnknownEndpoint)?;
    let caller = runtime.service_identity(SERVICE_SETTINGS).ok_or(IopError::AccessDenied)?;
    let service = runtime.service_identity(SERVICE_REPLICA_STORAGE).ok_or(IopError::AccessDenied)?;
    runtime.iop.ensure_owned_endpoint(0xe101, service)?;
    runtime.iop.ensure_owned_endpoint(0xe102, caller)?;
    let id = runtime.iop.next_node_request()?;
    let deadline = now.checked_add(30).ok_or(IopError::DeadlineExceeded)?;
    let capability = runtime.capabilities.grant(CapabilityType::ServiceCall, operation as u64,
        1, 0, service, caller, Some(deadline), 0)?;
    let result = (|| {
        let payload = request.encode().map_err(|_| IopError::InvalidPayload)?;
        let message = IopMessage::request(operation, id, caller, capability, deadline, id, &payload)?;
        runtime.iop.send(0xe101, message, &runtime.capabilities, now)?;
        let incoming = runtime.iop.receive(0xe101, now)?;
        if incoming.header.request_id != id || incoming.header.operation_type_id != operation as u32
            || incoming.header.caller_identity != caller || runtime.iop.is_cancelled(id) { return Err(IopError::InvalidHeader); }
        runtime.capabilities.validate(incoming.header.capability_ref, caller, CapabilityType::ServiceCall,
            operation as u64, 1, 0, now)?;
        let decoded = StorageOperationV1::decode(incoming.bytes()).map_err(|_| IopError::InvalidPayload)?;
        if decoded != request { return Err(IopError::InvalidPayload); }
        let (response, notice) = handler(iop::remote::AuthenticatedStorageRequest {
            local, peer: local, session_reference: [0; 16], grant: capability, request_id: id,
            correlation: id, causation: id, payload: decoded,
        }).map_err(|_| IopError::InvalidPayload)?;
        if response.operation != request.operation || response.object != request.object {
            return Err(IopError::InvalidPayload);
        }
        if let Some(notice) = notice { let _ = publish_storage_commit_from(runtime, notice, now); }
        let encoded = response.encode().map_err(|_| IopError::InvalidPayload)?;
        runtime.iop.respond(0xe102, &incoming, service, &encoded, now)?;
        let reply = runtime.iop.receive(0xe102, now)?;
        if reply.header.request_id != id || reply.header.causation_id != id { return Err(IopError::InvalidHeader); }
        StorageOperationV1::decode(reply.bytes()).map_err(|_| IopError::InvalidPayload)
    })();
    let _ = runtime.capabilities.retire_leaf(capability, service);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: observation_fixture
    // DESC: Returns an explicitly synthetic typed backend observation; native allocator and GPT measurements are verified in storage-device tests.
    // ------------------=
    fn observation_fixture(request: iop::remote::AuthenticatedStorageRequest)
        -> Result<(StorageOperationV1, Option<iop::storage_protocol::StorageCommit>), iop::remote::RemoteError> {
        assert_eq!(request.payload.operation, Operation::ResourceInspect);
        assert_ne!(request.peer.0, [0; 32]); assert_ne!(request.grant, 0);
        let mut value = request.payload; value.length = 48; value.data[..16].fill(7);
        value.data[16..32].fill(8); value.data[44] = 1; value.data[45] = 1;
        value.data[46..48].copy_from_slice(&1u16.to_le_bytes());
        value.object_version = 65536; value.offset = 32768; value.authority_generation = 1;
        value.manifest_generation = 17;
        Ok((value, None))
    }
    // ------------------------=
    // FUNC: local_storage_broker_uses_owned_queues_and_reclaims_query_authority
    // DESC: Exercises active-session checks, backend readiness, typed correlation and hundreds of repeated reads without exhausting capability or IOP slots.
    // ------------------=
    #[test]
    fn local_storage_broker_uses_owned_queues_and_reclaims_query_authority() {
        let mut runtime = InfinityRuntime::new(false); runtime.define_bootstrap().unwrap(); runtime.start_all(0);
        runtime.nodes.initialize(&[61; 32], true).unwrap(); runtime.node_clock = Some(10);
        runtime.identity.create_machine(b"storage-fixture", 64, 1, 0).unwrap();
        let user = runtime.identity.create_user(b"operator", b"Operator", 0).unwrap().id;
        runtime.identity.create_password(user, b"Fixture901", 0).unwrap();
        let session = runtime.identity.create_session(user, b"Fixture901", 1).unwrap().id;
        assert_eq!(query_from(&mut runtime, user, session), Err(IopError::UnknownEndpoint));
        runtime.storage_handler = Some(observation_fixture); runtime.start_all(10);
        assert_eq!(query_from(&mut runtime, StableId([0; 16]), session), Err(IopError::AccessDenied));
        assert_eq!(query_from(&mut runtime, user, StableId([0; 16])), Err(IopError::AccessDenied));
        for _ in 0..200 {
            let result = query_from(&mut runtime, user, session).unwrap();
            assert_eq!(result.object_version, 65536); assert_eq!(result.offset, 32768);
            assert_eq!(result.operation, Operation::ResourceInspect);
            assert_eq!(runtime.storage_last_observation, Some(result));
        }
        runtime.storage_handler = None;
        assert_eq!(query_from(&mut runtime, user, session), Err(IopError::UnknownEndpoint));
        assert!(runtime.storage_last_observation.is_none());
    }
}
