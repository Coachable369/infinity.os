//! Infinity Operation Protocol v1. The wire header is explicit little-endian;
//! Rust structure layout is never used as an ABI.

use super::capability::{CapabilityError, CapabilityId, CapabilityManager, CapabilityType};
use super::execution::SecurityIdentity;

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
    NamespaceResolve = 0x4001,
    NamespaceMove = 0x4002,
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
    ClipboardRead = 0xbf01,
    ClipboardWrite = 0xbf02,
    SystemPowerOff = 0xbb02,
    SystemRestart = 0xbb03,
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
    UnsupportedVersion,
    UnknownEndpoint,
    AccessDenied,
    DeadlineExceeded,
    Cancelled,
    Backpressure,
    PayloadTooLarge,
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
    endpoints: [Option<Endpoint>; MAX_ENDPOINTS],
    cancelled: [u64; 16],
    cancelled_len: usize,
}
impl IopRouter {
    // ------------------------=
    // FUNC: new
    // DESC: Creates and initializes a new instance.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            endpoints: [None; MAX_ENDPOINTS],
            cancelled: [0; 16],
            cancelled_len: 0,
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
