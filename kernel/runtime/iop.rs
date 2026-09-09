//! Infinity Operation Protocol v1. The wire header is explicit little-endian;
//! Rust structure layout is never used as an ABI.

use super::capability::{CapabilityError, CapabilityId, CapabilityManager, CapabilityType};
use super::execution::SecurityIdentity;

#[path = "iop_remote.rs"]
pub mod remote;
#[path = "iop_storage.rs"]
pub mod storage_protocol;
#[path = "iop_local_node.rs"]
mod local_node;

pub const IOP_VERSION: u16 = 1;
pub const HEADER_BYTES: usize = 80;
pub const MAX_PAYLOAD: usize = 192;
pub const MAX_ENDPOINTS: usize = 16;
pub const ENDPOINT_QUEUE_CAPACITY: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MessageType {
    Request = 1,
    Response = 2,
    Error = 3,
    Cancel = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum OperationId {
    TestEcho = 1,
    ServiceList = 0x1001,
    ServiceInspect = 0x1002,
    ServiceRestart = 0x1003,
    CapabilityGrant = 0x2001,
    CapabilityRevoke = 0x2002,
    ObjectCreate = 0x3001,
    ObjectRead = 0x3002,
    ObjectUpdate = 0x3003,
    ObjectQuery = 0x3004,
    ObjectHistory = 0x3005,
    ObjectFilter = 0x3006,
    ObjectDestroy = 0x3007,
    ObjectCopy = 0x3008,
    ObjectDelete = 0x3009,
    ObjectInspect = 0x300a,
    ObjectSearch = 0x300b,
    ObjectRelationships = 0x300c,
    ObjectVersionRead = 0x300d,
    ObjectVersionRestore = 0x300e,
    NamespaceResolve = 0x4001,
    NamespaceMove = 0x4002,
    NamespaceList = 0x4003,
    NamespaceCreate = 0x4004,
    NamespaceAttach = 0x4005,
    NamespaceDetach = 0x4006,
    NamespaceDelete = 0x4007,
    NamespaceParent = 0x4008,
    NamespaceChildren = 0x4009,
    StorageQuery = 0x5001,
    StorageUsage = 0x5002,
    EventSubscribe = 0x6001,
    RuntimeContexts = 0x7001,
    RuntimeResources = 0x7002,
    SystemGenerationList = 0x8001,
    SystemGenerationInspect = 0x8002,
    SystemGenerationActivate = 0x8003,
    SystemBootStatus = 0x8004,
    SystemStatus = 0x8005,
    SystemInfo = 0x8006,
    DeviceList = 0x8007,
    MemoryStatus = 0x8008,
    DeviceInspect = 0x8009,
    ProjectList = 0xa001,
    ProjectInspect = 0xa002,
    ProjectCreate = 0xa003,
    CollectionList = 0xa101,
    CollectionInspect = 0xa102,
    CollectionCreate = 0xa103,
    CapabilityList = 0xa201,
    EventSubscriptions = 0xa301,
    VoiceStatus = 0xa401,
    ModelList = 0x9001,
    ModelInspect = 0x9002,
    ModelLoad = 0x9003,
    ModelUnload = 0x9004,
    ModelCapabilities = 0x9005,
    ModelInfer = 0x9006,
    IntentResolve = 0x9101,
    ContextRequest = 0x9201,
    ToolInvoke = 0x9301,
    VoiceSessionStart = 0x9401,
    VoiceSessionStop = 0x9402,
    SpeechRecognize = 0x9403,
    SpeechSynthesize = 0x9404,
    AgentList = 0x9501,
    AgentInspect = 0x9502,
    AgentRequestTask = 0x9503,
    AgentTaskResult = 0x9504,
    AgentCancelTask = 0x9505,
    IdentityCreate = 0xb001,
    IdentityRead = 0xb002,
    IdentityList = 0xb003,
    IdentityUpdate = 0xb004,
    IdentityDelete = 0xb005,
    MachineRead = 0xb101,
    MachineUpdate = 0xb102,
    CredentialCreate = 0xb201,
    CredentialList = 0xb202,
    CredentialDelete = 0xb203,
    AuthenticationVerify = 0xb301,
    SessionCreate = 0xb401,
    SessionRead = 0xb402,
    SessionList = 0xb403,
    SessionLock = 0xb404,
    SessionUnlock = 0xb405,
    SessionEnd = 0xb406,
    ProfileRead = 0xb501,
    ProfileUpdate = 0xb502,
    PersonalSpaceRead = 0xb601,
    AiProfileRead = 0xb701,
    AiProfileUpdate = 0xb702,
    VoiceProfileRead = 0xb801,
    VoiceProfileUpdate = 0xb802,
    SettingsRead = 0xb901,
    SettingsUpdate = 0xb902,
    OnboardingRead = 0xba01,
    OnboardingAdvance = 0xba02,
    ShellOpen = 0xbb01,
    FontList = 0xbc01,
    FontOpen = 0xbc02,
    SkinList = 0xbd01,
    SkinInspect = 0xbd02,
    SkinValidate = 0xbd03,
    AppearanceRead = 0xbd11,
    AppearanceSetSkin = 0xbd12,
    AppearanceSetScale = 0xbd13,
    AppearanceSetAccent = 0xbd14,
    AppearanceSetWallpaper = 0xbd15,
    UiInspectTree = 0xbe01,
    UiInspectFocus = 0xbe02,
    UiInspectDamage = 0xbe03,
    WindowList = 0xbe11,
    SurfaceCreate = 0xbe12,
    SurfaceDestroy = 0xbe13,
    SurfacePresent = 0xbe14,
    WindowCreate = 0xbe15,
    WindowClose = 0xbe16,
    WindowMove = 0xbe17,
    WindowResize = 0xbe18,
    WindowSetState = 0xbe19,
    WindowFocus = 0xbe1a,
    WindowCapturePointer = 0xbe1b,
    WindowReleasePointer = 0xbe1c,
    WindowInspect = 0xbe1d,
    SurfaceList = 0xbe1e,
    SurfaceInspect = 0xbe1f,
    SurfaceResize = 0xbe20,
    SurfaceCommit = 0xbe21,
    CompositorStatus = 0xbe22,
    CompositorDiagnostics = 0xbe23,
    DisplayQuery = 0xbe24,
    SecureInputStatus = 0xbe25,
    ClipboardRead = 0xbf01,
    ClipboardWrite = 0xbf02,
    TrashMove = 0xbf11,
    TrashList = 0xbf12,
    TrashRestore = 0xbf13,
    TrashDelete = 0xbf14,
    TrashEmpty = 0xbf15,
    ApplicationLaunch = 0xbf21,
    ApplicationAssociationResolve = 0xbf22,
    ShellProfileList = 0xbf31,
    ShellProfileInspect = 0xbf32,
    ShellProfileCreate = 0xbf33,
    ShellProfileClone = 0xbf34,
    ShellProfileUpdate = 0xbf35,
    ShellProfileDelete = 0xbf36,
    ShellProfileEnable = 0xbf37,
    ShellProfileDisable = 0xbf38,
    ShellProfileSetDefault = 0xbf39,
    ShellAliasList = 0xbf41,
    ShellAliasAdd = 0xbf42,
    ShellAliasDelete = 0xbf43,
    ShellAliasResolve = 0xbf44,
    SystemPowerOff = 0xbb02,
    SystemRestart = 0xbb03,
    NetworkInterfaceList = 0xc001,
    NetworkInterfaceInspect = 0xc002,
    NetworkInterfaceSetState = 0xc003,
    NetworkAddressList = 0xc011,
    NetworkAddressConfigure = 0xc012,
    NetworkAddressRemove = 0xc013,
    NetworkRouteList = 0xc021,
    NetworkRouteInspect = 0xc022,
    NetworkRouteAdd = 0xc023,
    NetworkRouteRemove = 0xc024,
    NetworkResolve = 0xc031,
    NetworkConnect = 0xc041,
    NetworkListen = 0xc042,
    NetworkAccept = 0xc043,
    NetworkSend = 0xc044,
    NetworkReceive = 0xc045,
    NetworkClose = 0xc046,
    NetworkConnectionInspect = 0xc047,
    NetworkConnectionList = 0xc048,
    NetworkPolicyList = 0xc051,
    NetworkPolicyInspect = 0xc052,
    NetworkPolicyCreate = 0xc053,
    NetworkPolicyUpdate = 0xc054,
    NetworkPolicyDelete = 0xc055,
    NetworkProfileList = 0xc061,
    NetworkProfileInspect = 0xc062,
    NetworkProfileActivate = 0xc063,
    NetworkProfileCreate = 0xc064,
    NetworkProfileUpdate = 0xc065,
    NetworkProfileDelete = 0xc066,
    NetworkStatus = 0xc071,
    NetworkDiagnostics = 0xc072,
    ServiceDiscoverLocal = 0xc081,
    ServiceAdvertiseLocal = 0xc082,
    NodeList = 0xd001,
    NodeInspect = 0xd002,
    NodeDiscoverStatus = 0xd008,
    NodePairBegin = 0xd003,
    NodePairConfirm = 0xd004,
    NodePairCancel = 0xd005,
    NodeTrustRead = 0xd006,
    NodeTrustUpdate = 0xd007,
    NodeRevokeTrust = 0xd009,
    NodeBlock = 0xd00a,
    NodeUnblock = 0xd00b,
    NodeSessionList = 0xd011,
    NodeSessionOpen = 0xd012,
    NodeSessionClose = 0xd013,
    NodeSessionInspect = 0xd014,
    NodeCapabilityList = 0xd021,
    NodeCapabilityGrant = 0xd022,
    NodeCapabilityRevoke = 0xd023,
    MeshStatus = 0xd031,
    MeshMemberList = 0xd032,
    MeshMemberAdd = 0xd033,
    MeshMemberRemove = 0xd034,
    MeshPolicyRead = 0xd041,
    MeshPolicyUpdate = 0xd042,
    NodeAuditList = 0xd051,
    NodeAuditInspect = 0xd052,
    NodeJoin = 0xd061,
    NodeLeave = 0xd062,
    NodeDomainList = 0xd063,
    NodeDomainInspect = 0xd064,
    NodeLinkConfigure = 0xd071,
    NodeLinkList = 0xd072,
    NodeLinkRemove = 0xd073,
    NodePolicyRead = 0xd065,
    NodePolicyUpdate = 0xd066,
    NodeHealth = 0xd067,
    NodeDiagnostics = 0xd068,
    ResourceAdvertise = 0xe001,
    ResourceInspect = 0xe002,
    PoolInspect = 0xe010,
    ObjectSetPolicy = 0xe011,
    PoolUploadBegin = 0xe012,
    PoolUploadAppend = 0xe013,
    PoolUploadCommit = 0xe014,
    PoolUploadAbort = 0xe015,
    ReplicaInspect = 0xe020,
    ReplicaTransferBegin = 0xe021,
    ReplicaTransferChunk = 0xe022,
    ReplicaTransferCommit = 0xe023,
    ReplicaDelete = 0xe024,
    PoolHeal = 0xe030,
    PoolMetadata = 0xe050,
}
impl OperationId {
    // ------------------------=
    // FUNC: machine_id
    // DESC: Implements the machine id operation.
    // ------------------=
    pub const fn machine_id(self) -> u32 {
        self as u32
    }
}

pub const WINDOW_MOVE_V1_BYTES: usize = 28;

pub const NODE_OPERATION_V1_BYTES: usize = 80;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeOperationV1 {
    pub node_id: [u8; 32],
    pub handle: u64,
    pub scope: u64,
    pub lease_deadline: u64,
    pub operation: u32,
    pub rights: u32,
    pub value: u32,
    pub flags: u32,
    pub schema_version: u16,
}

impl NodeOperationV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes one architecture-neutral versioned node request without Rust ABI layout.
    // ------------------=
    pub fn encode(self) -> [u8; NODE_OPERATION_V1_BYTES] {
        let mut out = [0u8; NODE_OPERATION_V1_BYTES];
        out[..32].copy_from_slice(&self.node_id);
        put_u64(&mut out, 32, self.handle);
        put_u64(&mut out, 40, self.scope);
        put_u64(&mut out, 48, self.lease_deadline);
        put_u32(&mut out, 56, self.operation);
        put_u32(&mut out, 60, self.rights);
        put_u32(&mut out, 64, self.value);
        put_u32(&mut out, 68, self.flags);
        put_u16(&mut out, 72, self.schema_version);
        out
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Validates and decodes the fixed-width node operation schema version.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, IopError> {
        if input.len() != NODE_OPERATION_V1_BYTES {
            return Err(IopError::InvalidPayload);
        }
        let operation = get_u32(input, 56);
        if get_u16(input, 72) != 1
            || input[74..80].iter().any(|byte| *byte != 0)
            || !is_node_operation(operation)
        {
            return Err(IopError::InvalidPayload);
        }
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&input[..32]);
        Ok(Self {
            node_id,
            handle: get_u64(input, 32),
            scope: get_u64(input, 40),
            lease_deadline: get_u64(input, 48),
            operation: get_u32(input, 56),
            rights: get_u32(input, 60),
            value: get_u32(input, 64),
            flags: get_u32(input, 68),
            schema_version: 1,
        })
    }
}

// ------------------------=
// FUNC: is_node_operation
// DESC: Rejects payloads whose embedded operation is outside the version-one node and mesh registry.
// ------------------=
fn is_node_operation(operation: u32) -> bool {
    matches!(
        operation,
        0xd001..=0xd00b
            | 0xd011..=0xd014
            | 0xd021..=0xd023
            | 0xd031..=0xd034
            | 0xd041..=0xd042
            | 0xd051..=0xd052
            | 0xd061..=0xd068
            | 0xd071..=0xd073
    )
}

pub const NODE_OPERATION_HUMAN_APPROVED: u32 = 1;
pub const NODE_OPERATION_DURABLE: u32 = 2;

// ------------------------=
// FUNC: node_read_operation
// DESC: Enumerates side-effect-free node service reads; every other operation requires a durable transaction dispatcher.
// ------------------=
pub fn node_read_operation(operation: OperationId) -> bool {
    if operation == OperationId::NodeLinkList { return true; }
    matches!(operation, OperationId::NodeDiscoverStatus | OperationId::NodeList | OperationId::NodeInspect | OperationId::NodeTrustRead | OperationId::NodeHealth | OperationId::NodeDiagnostics | OperationId::NodePolicyRead | OperationId::NodeSessionList | OperationId::NodeSessionInspect | OperationId::NodeDomainList | OperationId::NodeDomainInspect | OperationId::NodeCapabilityList | OperationId::NodeAuditList | OperationId::NodeAuditInspect | OperationId::MeshStatus | OperationId::MeshMemberList | OperationId::MeshPolicyRead)
}

// ------------------------=
// FUNC: execute_node_operation
// DESC: Executes the decoded NodeOperationV1 service contract against authoritative node state.
// ------------------=
pub fn execute_node_operation(
    nodes: &mut super::node::NodeRuntime,
    operation: OperationId,
    request: NodeOperationV1,
    now: u64,
    correlation_id: u64,
) -> Result<NodeOperationV1, IopError> {
    use super::node::types::{MeshRole, NodeId, PolicyDecision, TrustState};
    if request.operation != operation.machine_id() || request.schema_version != 1 {
        return Err(IopError::InvalidPayload);
    }
    let peer = NodeId(request.node_id);
    let mut response = request;
    match operation {
        OperationId::NodeDiscoverStatus => {
            response.value = nodes.discovered_nodes().iter().flatten().count() as u32;
            response.scope = nodes.control_version();
        }
        OperationId::NodePolicyRead => {
            let node = nodes.discovered_nodes().iter().flatten().find(|node| node.id == peer)
                .ok_or(IopError::InvalidPayload)?;
            response.handle = nodes.control_version();
            response.scope = node.policy.scope;
            response.lease_deadline = node.policy.expires_at;
            response.value = node.policy.version;
            response.flags = 12;
            response.rights = 0;
            for (index, category) in node.policy.categories.iter().enumerate() {
                let value = match category {
                    PolicyDecision::Deny => 0, PolicyDecision::Allow => 1,
                    PolicyDecision::SessionOnly => 2, PolicyDecision::Leased => 3,
                };
                response.rights |= value << (index * 2);
            }
        }
        OperationId::NodeInspect | OperationId::NodeSessionInspect | OperationId::NodeDomainInspect | OperationId::NodeDomainList | OperationId::NodeList | OperationId::NodeSessionList | OperationId::NodeLinkList => {
            return super::node::inspection::inspect(nodes, operation, request);
        }
        OperationId::NodeTrustRead
        | OperationId::NodeHealth | OperationId::NodeDiagnostics => {
            let node = nodes.discovered_nodes().iter().flatten().find(|node| node.id == peer)
                .ok_or(IopError::InvalidPayload)?;
            response.value = trust_value(node.trust);
            response.flags = reachability_value(node.reachability);
            response.scope = node.policy.scope;
            response.lease_deadline = node.policy.expires_at;
        }
        OperationId::NodePairBegin => {
            let pairing = nodes.begin_pairing(peer, now).map_err(map_node_error)?;
            response.handle = pairing.id;
            response.value = pairing.verification_code;
            response.lease_deadline = pairing.expires_at;
        }
        OperationId::NodePairConfirm => {
            let approved = request.flags & NODE_OPERATION_HUMAN_APPROVED != 0;
            nodes.confirm_pairing(request.handle, request.value, approved, now, correlation_id)
                .map_err(map_node_error)?;
        }
        OperationId::NodePairCancel => nodes.cancel_pairing(request.handle).map_err(map_node_error)?,
        OperationId::NodeTrustUpdate => {
            let state = trust_from_value(request.value).ok_or(IopError::InvalidPayload)?;
            nodes.set_trust(peer, state, now, correlation_id).map_err(map_node_error)?;
        }
        OperationId::NodeRevokeTrust => nodes.revoke_trust(peer, now, correlation_id).map_err(map_node_error)?,
        OperationId::NodeBlock => nodes.set_trust(peer, TrustState::Blocked, now, correlation_id).map_err(map_node_error)?,
        OperationId::NodeUnblock => nodes.set_trust(peer, TrustState::Untrusted, now, correlation_id).map_err(map_node_error)?,
        OperationId::NodeSessionClose => nodes.close_session(request.handle, now, correlation_id).map_err(map_node_error)?,
        OperationId::NodeCapabilityList => response.value = (nodes.remote_grants().iter().flatten().count()+nodes.durable_approvals().iter().flatten().count()) as u32,
        OperationId::NodeCapabilityGrant => {
            if remote::operation(request.value).is_err()
                && storage_protocol::Operation::decode(request.value).is_err() { return Err(IopError::InvalidPayload); }
            if request.flags == NODE_OPERATION_HUMAN_APPROVED | NODE_OPERATION_DURABLE {
                if request.rights!=1||request.lease_deadline!=0{return Err(IopError::AccessDenied)}
                response.handle=nodes.grant_durable(peer,request.value,request.scope,request.rights,now,correlation_id).map_err(map_node_error)?;
            } else {
            if request.flags != NODE_OPERATION_HUMAN_APPROVED || request.rights != 1
                || request.lease_deadline <= now || request.lease_deadline - now > 3600 {
                return Err(IopError::AccessDenied);
            }
            response.handle = nodes.grant_remote(peer, request.value, request.scope, request.rights,
                request.lease_deadline, now, correlation_id).map_err(map_node_error)?;
            }
        }
        OperationId::NodeCapabilityRevoke => nodes.revoke_remote(request.handle, now, correlation_id).map_err(map_node_error)?,
        OperationId::MeshStatus | OperationId::MeshMemberList | OperationId::MeshPolicyRead => {
            response.value = nodes.mesh_members().iter().flatten().filter(|member| member.enabled).count() as u32;
        }
        OperationId::MeshMemberAdd | OperationId::NodeJoin => {
            let role = match request.value { 0 => MeshRole::Member, 1 => MeshRole::Operator, 2 => MeshRole::Gateway, 3 => MeshRole::Compute, 4 => MeshRole::Storage, _ => return Err(IopError::InvalidPayload) };
            nodes.join_mesh(peer, role, now, correlation_id).map_err(map_node_error)?;
        }
        OperationId::MeshMemberRemove | OperationId::NodeLeave => nodes.leave_mesh(peer, now, correlation_id).map_err(map_node_error)?,
        OperationId::NodePolicyUpdate | OperationId::MeshPolicyUpdate => {
            let category = (request.flags & 0xff) as usize;
            let reset = request.flags == 0xffff && request.value == 0;
            if category >= 12 && !reset { return Err(IopError::InvalidPayload); }
            let current = nodes.discovered_nodes().iter().flatten().find(|node| node.id == peer)
                .ok_or(IopError::InvalidPayload)?;
            let mut policy = current.policy;
            if reset { policy.categories = [PolicyDecision::Deny; 12]; }
            else { policy.categories[category] = match request.value { 0 => PolicyDecision::Deny, 1 => PolicyDecision::Allow, 2 => PolicyDecision::SessionOnly, 3 => PolicyDecision::Leased, _ => return Err(IopError::InvalidPayload) }; }
            policy.scope = request.scope;
            policy.expires_at = request.lease_deadline;
            policy.version = policy.version.saturating_add(1);
            nodes.update_policy(peer, policy, now, correlation_id).map_err(map_node_error)?;
        }
        OperationId::NodeAuditList | OperationId::NodeAuditInspect => response.value = nodes.audit_records().iter().flatten().count() as u32,
        OperationId::NodeSessionOpen => return Err(IopError::InvalidPayload),
        _ => return Err(IopError::InvalidPayload),
    }
    Ok(response)
}

// ------------------------=
// FUNC: dispatch_node_operation
// DESC: Receives, decodes, executes, and returns one capability-validated node operation through the bounded IOP router.
// ------------------=
pub fn dispatch_node_operation(
    router: &mut IopRouter,
    capabilities: &CapabilityManager,
    nodes: &mut super::node::NodeRuntime,
    operation: OperationId,
    service_endpoint: u16,
    response_endpoint: u16,
    service_identity: SecurityIdentity,
    now: u64,
) -> Result<NodeOperationV1, IopError> {
    let request = router.receive(service_endpoint, now)?;
    if request.header.message_type != MessageType::Request
        || request.header.operation_type_id != operation.machine_id()
    {
        return Err(IopError::InvalidPayload);
    }
    let decoded = NodeOperationV1::decode(request.bytes())?;
    // Authority may have expired or been revoked while this request was queued.
    capabilities.validate(
        request.header.capability_ref,
        request.header.caller_identity,
        CapabilityType::ServiceCall,
        operation.machine_id() as u64,
        1,
        0,
        now,
    )?;
    if !node_read_operation(operation) { return Err(IopError::AccessDenied); }
    router.ensure_owned_endpoint(service_endpoint, service_identity)?;
    router.ensure_owned_endpoint(response_endpoint, request.header.caller_identity)?;
    if router.is_cancelled(request.header.request_id) { return Err(IopError::Cancelled); }
    let response = execute_node_operation(
        nodes,
        operation,
        decoded,
        now,
        request.header.correlation_id,
    )?;
    router.respond(
        response_endpoint,
        &request,
        service_identity,
        &response.encode(),
        now,
    )?;
    Ok(response)
}

// ------------------------=
// FUNC: trust_value
// DESC: Projects a trust state into the stable NodeOperationV1 value field.
// ------------------=
fn trust_value(state: super::node::types::TrustState) -> u32 {
    use super::node::types::TrustState;
    match state { TrustState::Discovered => 0, TrustState::Untrusted => 1, TrustState::PairingPending => 2, TrustState::Trusted => 3, TrustState::Restricted => 4, TrustState::Revoked => 5, TrustState::Blocked => 6, TrustState::Incompatible => 7 }
}

// ------------------------=
// FUNC: trust_from_value
// DESC: Validates a stable NodeOperationV1 trust-state discriminator.
// ------------------=
fn trust_from_value(value: u32) -> Option<super::node::types::TrustState> {
    use super::node::types::TrustState;
    match value { 0 => Some(TrustState::Discovered), 1 => Some(TrustState::Untrusted), 2 => Some(TrustState::PairingPending), 3 => Some(TrustState::Trusted), 4 => Some(TrustState::Restricted), 5 => Some(TrustState::Revoked), 6 => Some(TrustState::Blocked), 7 => Some(TrustState::Incompatible), _ => None }
}

// ------------------------=
// FUNC: reachability_value
// DESC: Projects observed reachability into the stable NodeOperationV1 flags field.
// ------------------=
fn reachability_value(state: super::node::types::Reachability) -> u32 {
    use super::node::types::Reachability;
    match state { Reachability::Unknown => 0, Reachability::Online => 1, Reachability::Degraded => 2, Reachability::Offline => 3 }
}

// ------------------------=
// FUNC: map_node_error
// DESC: Maps node-service validation and authority failures to the typed IOP error boundary.
// ------------------=
fn map_node_error(error: super::node::types::NodeError) -> IopError {
    use super::node::types::NodeError;
    match error {
        NodeError::CapabilityDenied | NodeError::CapabilityExpired | NodeError::CapabilityRevoked
        | NodeError::Blocked | NodeError::NotTrusted | NodeError::HumanApprovalRequired => IopError::AccessDenied,
        _ => IopError::InvalidPayload,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowMoveV1 {
    pub window_id: u32,
    pub x: i32,
    pub y: i32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: u32,
    pub work_height: u32,
}

impl WindowMoveV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes the version-one Window.Move request schema in stable little-endian field order.
    // ------------------=
    pub fn encode(&self, out: &mut [u8; WINDOW_MOVE_V1_BYTES]) {
        put_u32(out, 0, self.window_id);
        put_i32(out, 4, self.x);
        put_i32(out, 8, self.y);
        put_i32(out, 12, self.work_x);
        put_i32(out, 16, self.work_y);
        put_u32(out, 20, self.work_width);
        put_u32(out, 24, self.work_height);
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes an exact-length Window.Move request and rejects malformed payload framing.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, IopError> {
        if input.len() != WINDOW_MOVE_V1_BYTES {
            return Err(IopError::InvalidPayload);
        }
        Ok(Self {
            window_id: get_u32(input, 0),
            x: get_i32(input, 4),
            y: get_i32(input, 8),
            work_x: get_i32(input, 12),
            work_y: get_i32(input, 16),
            work_width: get_u32(input, 20),
            work_height: get_u32(input, 24),
        })
    }
}

pub const SURFACE_COMMIT_V1_BYTES: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SurfaceCommitV1 {
    pub surface_id: u32,
    pub generation: u64,
    pub damage_count: u16,
}

impl SurfaceCommitV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a bounded Surface.Commit generation and semantic-damage count.
    // ------------------=
    pub fn encode(&self, out: &mut [u8; SURFACE_COMMIT_V1_BYTES]) {
        out.fill(0);
        put_u32(out, 0, self.surface_id);
        put_u64(out, 4, self.generation);
        put_u16(out, 12, self.damage_count);
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes an exact version-one Surface.Commit request payload.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, IopError> {
        if input.len() != SURFACE_COMMIT_V1_BYTES {
            return Err(IopError::InvalidPayload);
        }
        Ok(Self {
            surface_id: get_u32(input, 0),
            generation: get_u64(input, 4),
            damage_count: get_u16(input, 12),
        })
    }
}

pub const NETWORK_CONNECT_V1_BYTES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NetworkConnectV1 {
    pub address_family: u8,
    pub remote_address: [u8; 16],
    pub remote_port: u16,
    pub protocol: u8,
    pub secure: bool,
    pub queue_limit: u16,
    pub expected_identity: u64,
}

impl NetworkConnectV1 {
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes the version-one typed Network.Connect request without native-structure layout dependencies.
    // ------------------=
    pub fn encode(&self, out: &mut [u8; NETWORK_CONNECT_V1_BYTES]) {
        out.fill(0);
        out[0] = self.address_family;
        out[1] = self.protocol;
        out[2] = self.secure as u8;
        out[4..20].copy_from_slice(&self.remote_address);
        put_u16(out, 20, self.remote_port);
        put_u16(out, 22, self.queue_limit);
        put_u64(out, 24, self.expected_identity);
    }

    // ------------------------=
    // FUNC: decode
    // DESC: Decodes and validates one exact-length version-one Network.Connect request.
    // ------------------=
    pub fn decode(input: &[u8]) -> Result<Self, IopError> {
        if input.len() != NETWORK_CONNECT_V1_BYTES
            || !matches!(input[0], 4 | 6)
            || !matches!(input[1], 1 | 2)
            || input[2] > 1
            || get_u16(input, 22) == 0
        {
            return Err(IopError::InvalidPayload);
        }
        let mut remote_address = [0u8; 16];
        remote_address.copy_from_slice(&input[4..20]);
        Ok(Self {
            address_family: input[0],
            remote_address,
            remote_port: get_u16(input, 20),
            protocol: input[1],
            secure: input[2] == 1,
            queue_limit: get_u16(input, 22),
            expected_identity: get_u64(input, 24),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IopHeader {
    pub protocol_version: u16,
    pub message_type: MessageType,
    pub schema_version: u8,
    pub operation_type_id: u32,
    pub request_id: u64,
    pub caller_identity: SecurityIdentity,
    pub capability_ref: CapabilityId,
    pub payload_length: u32,
    pub flags: u32,
    pub deadline: u64,
    pub correlation_id: u64,
    pub causation_id: u64,
}

impl IopHeader {
    // ------------------------=
    // FUNC: encode
    // DESC: Implements the encode operation.
    // ------------------=
    pub fn encode(&self, out: &mut [u8; HEADER_BYTES]) {
        out.fill(0);
        out[..4].copy_from_slice(b"IOP1");
        put_u16(out, 4, self.protocol_version);
        out[6] = self.message_type as u8;
        out[7] = self.schema_version;
        put_u32(out, 8, self.operation_type_id);
        put_u64(out, 12, self.request_id);
        out[20..36].copy_from_slice(&self.caller_identity.0);
        put_u64(out, 36, self.capability_ref);
        put_u32(out, 44, self.payload_length);
        put_u32(out, 48, self.flags);
        put_u64(out, 52, self.deadline);
        put_u64(out, 60, self.correlation_id);
        put_u64(out, 68, self.causation_id);
        let header_checksum = checksum(&out[..76]);
        put_u32(out, 76, header_checksum);
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Implements the decode operation.
    // ------------------=
    pub fn decode(data: &[u8; HEADER_BYTES]) -> Result<Self, IopError> {
        if &data[..4] != b"IOP1"
            || get_u16(data, 4) != IOP_VERSION
            || checksum(&data[..76]) != get_u32(data, 76)
        {
            return Err(IopError::InvalidHeader);
        }
        let message_type = match data[6] {
            1 => MessageType::Request,
            2 => MessageType::Response,
            3 => MessageType::Error,
            4 => MessageType::Cancel,
            _ => return Err(IopError::InvalidHeader),
        };
        let mut identity = [0u8; 16];
        identity.copy_from_slice(&data[20..36]);
        Ok(Self {
            protocol_version: get_u16(data, 4),
            message_type,
            schema_version: data[7],
            operation_type_id: get_u32(data, 8),
            request_id: get_u64(data, 12),
            caller_identity: SecurityIdentity(identity),
            capability_ref: get_u64(data, 36),
            payload_length: get_u32(data, 44),
            flags: get_u32(data, 48),
            deadline: get_u64(data, 52),
            correlation_id: get_u64(data, 60),
            causation_id: get_u64(data, 68),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PayloadRef {
    Inline,
    Object([u8; 16]),
    SharedBuffer {
        capability: CapabilityId,
        offset: u32,
        length: u32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IopMessage {
    pub header: IopHeader,
    pub payload: [u8; MAX_PAYLOAD],
    pub payload_ref: PayloadRef,
}
impl IopMessage {
    // ------------------------=
    // FUNC: request
    // DESC: Implements the request operation.
    // ------------------=
    pub fn request(
        operation: OperationId,
        request_id: u64,
        caller: SecurityIdentity,
        capability: CapabilityId,
        deadline: u64,
        correlation_id: u64,
        payload: &[u8],
    ) -> Result<Self, IopError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(IopError::PayloadTooLarge);
        }
        let mut bytes = [0u8; MAX_PAYLOAD];
        bytes[..payload.len()].copy_from_slice(payload);
        Ok(Self {
            header: IopHeader {
                protocol_version: IOP_VERSION,
                message_type: MessageType::Request,
                schema_version: 1,
                operation_type_id: operation.machine_id(),
                request_id,
                caller_identity: caller,
                capability_ref: capability,
                payload_length: payload.len() as u32,
                flags: 0,
                deadline,
                correlation_id,
                causation_id: 0,
            },
            payload: bytes,
            payload_ref: PayloadRef::Inline,
        })
    }
    // ------------------------=
    // FUNC: response
    // DESC: Implements the response operation.
    // ------------------=
    pub fn response(
        request: &IopMessage,
        responder: SecurityIdentity,
        payload: &[u8],
    ) -> Result<Self, IopError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(IopError::PayloadTooLarge);
        }
        let mut bytes = [0u8; MAX_PAYLOAD];
        bytes[..payload.len()].copy_from_slice(payload);
        Ok(Self {
            header: IopHeader {
                protocol_version: IOP_VERSION,
                message_type: MessageType::Response,
                schema_version: request.header.schema_version,
                operation_type_id: request.header.operation_type_id,
                request_id: request.header.request_id,
                caller_identity: responder,
                capability_ref: 0,
                payload_length: payload.len() as u32,
                flags: 0,
                deadline: request.header.deadline,
                correlation_id: request.header.correlation_id,
                causation_id: request.header.request_id,
            },
            payload: bytes,
            payload_ref: PayloadRef::Inline,
        })
    }
    // ------------------------=
    // FUNC: bytes
    // DESC: Implements the bytes operation.
    // ------------------=
    pub fn bytes(&self) -> &[u8] {
        &self.payload[..self.header.payload_length.min(MAX_PAYLOAD as u32) as usize]
    }
}

#[derive(Clone, Copy)]
struct Endpoint {
    id: u16,
    owner: SecurityIdentity,
    queue: [Option<IopMessage>; ENDPOINT_QUEUE_CAPACITY],
    head: usize,
    len: usize,
}
impl Endpoint {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    const fn new(id: u16, owner: SecurityIdentity) -> Self {
        Self {
            id,
            owner,
            queue: [None; ENDPOINT_QUEUE_CAPACITY],
            head: 0,
            len: 0,
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Implements the push operation.
    // ------------------=
    fn push(&mut self, message: IopMessage) -> Result<(), IopError> {
        if self.len == ENDPOINT_QUEUE_CAPACITY {
            return Err(IopError::Backpressure);
        }
        let at = (self.head + self.len) % ENDPOINT_QUEUE_CAPACITY;
        self.queue[at] = Some(message);
        self.len += 1;
        Ok(())
    }
    // ------------------------=
    // FUNC: pop
    // DESC: Implements the pop operation.
    // ------------------=
    fn pop(&mut self) -> Option<IopMessage> {
        if self.len == 0 {
            return None;
        }
        let message = self.queue[self.head].take();
        self.head = (self.head + 1) % ENDPOINT_QUEUE_CAPACITY;
        self.len -= 1;
        message
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IopError {
    InvalidHeader,
    InvalidPayload,
    UnsupportedVersion,
    UnknownEndpoint,
    AccessDenied,
    DeadlineExceeded,
    Cancelled,
    Backpressure,
    PayloadTooLarge,
    PersistenceFailed,
}
impl From<CapabilityError> for IopError {
    // ------------------------=
    // FUNC: from
    // DESC: Implements the from operation.
    // ------------------=
    fn from(_: CapabilityError) -> Self {
        IopError::AccessDenied
    }
}

pub struct IopRouter {
    pub remote: remote::RemoteState,
    endpoints: [Option<Endpoint>; MAX_ENDPOINTS],
    cancelled: [u64; 16],
    cancelled_len: usize,
    next_local_request: u64,
    membership_tick: Option<u64>,
    membership_cursor: usize,
}
impl IopRouter {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            remote: remote::RemoteState::new(),
            endpoints: [None; MAX_ENDPOINTS],
            cancelled: [0; 16],
            cancelled_len: 0,
            next_local_request: 1u64 << 63,
            membership_tick: None,
            membership_cursor: 0,
        }
    }
    // ------------------------=
    // FUNC: register_endpoint
    // DESC: Writes or updates register endpoint data.
    // ------------------=
    pub fn register_endpoint(&mut self, id: u16, owner: SecurityIdentity) -> Result<(), IopError> {
        if self.endpoints.iter().flatten().any(|e| e.id == id) {
            return Err(IopError::Backpressure);
        }
        let slot = self
            .endpoints
            .iter()
            .position(Option::is_none)
            .ok_or(IopError::Backpressure)?;
        self.endpoints[slot] = Some(Endpoint::new(id, owner));
        Ok(())
    }
    // ------------------------=
    // FUNC: send
    // DESC: Implements the send operation.
    // ------------------=
    pub fn send(
        &mut self,
        target_endpoint: u16,
        message: IopMessage,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<(), IopError> {
        if message.header.protocol_version != IOP_VERSION {
            return Err(IopError::UnsupportedVersion);
        }
        if message.header.deadline != 0 && now >= message.header.deadline {
            return Err(IopError::DeadlineExceeded);
        }
        if self.is_cancelled(message.header.request_id) {
            return Err(IopError::Cancelled);
        }
        let target = self
            .endpoints
            .iter_mut()
            .flatten()
            .find(|e| e.id == target_endpoint)
            .ok_or(IopError::UnknownEndpoint)?;
        capabilities.validate(
            message.header.capability_ref,
            message.header.caller_identity,
            CapabilityType::ServiceCall,
            message.header.operation_type_id as u64,
            1,
            0,
            now,
        )?;
        target.push(message)
    }
    // ------------------------=
    // FUNC: receive
    // DESC: Implements the receive operation.
    // ------------------=
    pub fn receive(&mut self, endpoint: u16, now: u64) -> Result<IopMessage, IopError> {
        let target = self
            .endpoints
            .iter_mut()
            .flatten()
            .find(|e| e.id == endpoint)
            .ok_or(IopError::UnknownEndpoint)?;
        let message = target.pop().ok_or(IopError::Backpressure)?;
        if message.header.deadline != 0 && now >= message.header.deadline {
            return Err(IopError::DeadlineExceeded);
        }
        Ok(message)
    }
    // ------------------------=
    // FUNC: receive_into
    // DESC: Implements the receive into operation.
    // ------------------=
    pub fn receive_into(
        &mut self,
        endpoint: u16,
        now: u64,
        out: &mut IopMessage,
    ) -> Result<(), IopError> {
        let target = self
            .endpoints
            .iter_mut()
            .flatten()
            .find(|e| e.id == endpoint)
            .ok_or(IopError::UnknownEndpoint)?;
        if target.len == 0 {
            return Err(IopError::Backpressure);
        }
        let at = target.head;
        let Some(message) = target.queue[at].take() else {
            return Err(IopError::Backpressure);
        };
        target.head = (target.head + 1) % ENDPOINT_QUEUE_CAPACITY;
        target.len -= 1;
        if message.header.deadline != 0 && now >= message.header.deadline {
            return Err(IopError::DeadlineExceeded);
        }
        *out = message;
        Ok(())
    }
    // ------------------------=
    // FUNC: respond
    // DESC: Implements the respond operation.
    // ------------------=
    pub fn respond(
        &mut self,
        target_endpoint: u16,
        request: &IopMessage,
        responder: SecurityIdentity,
        payload: &[u8],
        now: u64,
    ) -> Result<(), IopError> {
        if request.header.deadline != 0 && now >= request.header.deadline {
            return Err(IopError::DeadlineExceeded);
        }
        if self.is_cancelled(request.header.request_id) {
            return Err(IopError::Cancelled);
        }
        let target = self
            .endpoints
            .iter_mut()
            .flatten()
            .find(|e| e.id == target_endpoint && e.owner == request.header.caller_identity)
            .ok_or(IopError::UnknownEndpoint)?;
        if payload.len() > MAX_PAYLOAD {
            return Err(IopError::PayloadTooLarge);
        }
        let mut response = *request;
        response.header.message_type = MessageType::Response;
        response.header.caller_identity = responder;
        response.header.capability_ref = 0;
        response.header.payload_length = payload.len() as u32;
        response.header.causation_id = request.header.request_id;
        response.payload.fill(0);
        response.payload[..payload.len()].copy_from_slice(payload);
        target.push(response)
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Implements the cancel operation.
    // ------------------=
    pub fn cancel(&mut self, request_id: u64) {
        if self.cancelled_len < self.cancelled.len() {
            self.cancelled[self.cancelled_len] = request_id;
            self.cancelled_len += 1
        }
    }
    // ------------------------=
    // FUNC: is_cancelled
    // DESC: Reports whether is cancelled.
    // ------------------=
    pub fn is_cancelled(&self, request_id: u64) -> bool {
        self.cancelled[..self.cancelled_len].contains(&request_id)
    }
    // ------------------------=
    // FUNC: queue_depth
    // DESC: Implements the queue depth operation.
    // ------------------=
    pub fn queue_depth(&self, endpoint: u16) -> Option<usize> {
        self.endpoints
            .iter()
            .flatten()
            .find(|e| e.id == endpoint)
            .map(|e| e.len)
    }
    // ------------------------=
    // FUNC: endpoint_owner
    // DESC: Implements the endpoint owner operation.
    // ------------------=
    pub fn endpoint_owner(&self, endpoint: u16) -> Option<SecurityIdentity> {
        self.endpoints
            .iter()
            .flatten()
            .find(|e| e.id == endpoint)
            .map(|e| e.owner)
    }
}

// ------------------------=
// FUNC: checksum
// DESC: Calculates and returns checksum.
// ------------------=
fn checksum(data: &[u8]) -> u32 {
    let mut h = 0x811c9dc5u32;
    for b in data {
        h ^= *b as u32;
        h = h.wrapping_mul(0x01000193)
    }
    h
}
// ------------------------=
// FUNC: put_u16
// DESC: Implements the put u16 operation.
// ------------------=
fn put_u16(o: &mut [u8], a: usize, v: u16) {
    o[a..a + 2].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: put_u32
// DESC: Implements the put u32 operation.
// ------------------=
fn put_u32(o: &mut [u8], a: usize, v: u32) {
    o[a..a + 4].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: put_u64
// DESC: Implements the put u64 operation.
// ------------------=
fn put_u64(o: &mut [u8], a: usize, v: u64) {
    o[a..a + 8].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: put_i32
// DESC: Encodes one signed 32-bit schema field in little-endian order.
// ------------------=
fn put_i32(o: &mut [u8], a: usize, v: i32) {
    o[a..a + 4].copy_from_slice(&v.to_le_bytes())
}
// ------------------------=
// FUNC: get_u16
// DESC: Reads get u16 data.
// ------------------=
fn get_u16(d: &[u8], a: usize) -> u16 {
    u16::from_le_bytes([d[a], d[a + 1]])
}
// ------------------------=
// FUNC: get_u32
// DESC: Reads get u32 data.
// ------------------=
fn get_u32(d: &[u8], a: usize) -> u32 {
    u32::from_le_bytes([d[a], d[a + 1], d[a + 2], d[a + 3]])
}
// ------------------------=
// FUNC: get_u64
// DESC: Reads get u64 data.
// ------------------=
fn get_u64(d: &[u8], a: usize) -> u64 {
    u64::from_le_bytes([
        d[a],
        d[a + 1],
        d[a + 2],
        d[a + 3],
        d[a + 4],
        d[a + 5],
        d[a + 6],
        d[a + 7],
    ])
}
// ------------------------=
// FUNC: get_i32
// DESC: Decodes one signed 32-bit schema field from little-endian bytes.
// ------------------=
fn get_i32(d: &[u8], a: usize) -> i32 {
    i32::from_le_bytes([d[a], d[a + 1], d[a + 2], d[a + 3]])
}
