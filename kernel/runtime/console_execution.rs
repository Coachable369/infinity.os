//! Shared native Console execution. Transport and graphical rendering are callers,
//! never authority. All supported commands use the existing schema parser and IOP.
use super::{InfinityRuntime, console_language::{self as language, ParseOutcome},
    identity::{StableId, SessionOrigin}, execution::SecurityIdentity,
    capability::CapabilityType, iop::{IopError, IopMessage, OperationId}, service::SERVICE_CONSOLE};

pub const OUTPUT_BYTES: usize = 8192;
#[derive(Clone, Copy)]
pub struct Environment { pub architecture: &'static [u8], pub ready_devices: u8 }
impl Environment {
    // ------------------------=
    // FUNC: new
    // DESC: Supplies a neutral snapshot until the native Console publishes its boot device summary.
    // ------------------=
    pub const fn new() -> Self { Self { architecture: b"unavailable", ready_devices: 0 } }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Diagnostic { Success, Syntax, Unsupported, Denied, Unavailable, Capacity, Cancelled, Deadline }
pub struct Bytes<const N: usize> { data: [u8; N], length: usize, overflow: bool }
impl<const N: usize> Bytes<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a fixed output budget with observable overflow.
    // ------------------=
    pub const fn new() -> Self { Self { data: [0; N], length: 0, overflow: false } }
    // ------------------------=
    // FUNC: bytes
    // DESC: Exposes only initialized output to presentation or SSH backpressure handling.
    // ------------------=
    pub fn bytes(&self) -> &[u8] { &self.data[..self.length] }
    // ------------------------=
    // FUNC: truncated
    // DESC: Exposes output backpressure so callers never report a partial catalog as complete.
    // ------------------=
    pub fn truncated(&self)->bool {self.overflow}
    // ------------------------=
    // FUNC: write
    // DESC: Bounds every write and marks truncation as an execution failure.
    // ------------------=
    pub fn write(&mut self, bytes: &[u8]) {
        let length = bytes.len().min(N - self.length);
        self.data[self.length..self.length + length].copy_from_slice(&bytes[..length]);
        self.length += length;
        self.overflow |= length != bytes.len();
    }
    // ------------------------=
    // FUNC: write_line
    // DESC: Emits canonical LF output; the terminal adapter owns CRLF conversion.
    // ------------------=
    pub fn write_line(&mut self, bytes: &[u8]) { self.write(bytes); self.write(b"\n"); }
    // ------------------------=
    // FUNC: write_segments
    // DESC: Formats native fragments without allocation or reparsing machine output.
    // ------------------=
    pub fn write_segments(&mut self, parts: &[&[u8]]) { for part in parts { self.write(part); } self.write(b"\n"); }
    // ------------------------=
    // FUNC: write_number
    // DESC: Emits a bounded unsigned value using the shared native output representation.
    // ------------------=
    pub fn write_number(&mut self, label: &[u8], mut value: u64) {
        let mut buffer = [0; 20]; let mut at = buffer.len();
        loop { at -= 1; buffer[at] = b'0' + (value % 10) as u8; value /= 10; if value == 0 { break; } }
        self.write_segments(&[label, &buffer[at..]]);
    }
    // ------------------------=
    // FUNC: write_hex
    // DESC: Formats public identifiers, never secret key material.
    // ------------------=
    pub fn write_hex(&mut self, label: &[u8], bytes: &[u8]) {
        self.write(label);
        for byte in bytes { self.write(&[b"0123456789abcdef"[(byte >> 4) as usize], b"0123456789abcdef"[(byte & 15) as usize]]); }
        self.write(b"\n");
    }
}
pub struct Outcome {
    pub stdout: Bytes<OUTPUT_BYTES>, pub stderr: Bytes<1024>, pub status: u32,
    pub diagnostic: Diagnostic, pub operation: Option<OperationId>,
}
impl Outcome {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an empty result; handlers must explicitly complete before success reaches a caller.
    // ------------------=
    pub const fn new() -> Self { Self { stdout: Bytes::new(), stderr: Bytes::new(), status: 0, diagnostic: Diagnostic::Success, operation: None } }
    // ------------------------=
    // FUNC: failure
    // DESC: Produces typed failure, stderr and nonzero status without interpreting display text.
    // ------------------=
    pub fn failure(diagnostic: Diagnostic) -> Self {
        let mut result = Self::new(); result.diagnostic = diagnostic;
        let (status, message): (u32, &[u8]) = match diagnostic {
            Diagnostic::Syntax => (2, b"Invalid native Console command."),
            Diagnostic::Unsupported => (126, b"This operation is not available in the headless Console."),
            Diagnostic::Denied => (126, b"Native execution authority denied."),
            Diagnostic::Unavailable => (69, b"The native service is unavailable."),
            Diagnostic::Capacity => (75, b"The bounded execution or output capacity was exceeded."),
            Diagnostic::Cancelled => (130, b"Command interrupted."),
            Diagnostic::Deadline => (124, b"Command deadline expired."),
            Diagnostic::Success => (0, b""),
        };
        result.status = status; result.stderr.write_line(message); result
    }
}
// ------------------------=
// FUNC: supported
// DESC: Defines a remote query ceiling over existing operation IDs; it grants no AI inference, GUI or administrative operations.
// ------------------=
pub fn supported(operation: OperationId) -> bool {
    matches!(operation, OperationId::SystemStatus | OperationId::NodeList | OperationId::IntentResolve)
}
// ------------------------=
// FUNC: handles
// DESC: Lets graphical Console retain all other existing handlers while sharing exactly these parsed operations.
// ------------------=
pub fn handles(command: &[u8]) -> bool {
    if super::console_help::requested(command) { return true; }
    match language::parse(command) {
        Ok(ParseOutcome::Help(_, _)) => true,
        Ok(ParseOutcome::Graph(graph)) => graph.node_count == 1 && !graph.plan_only && graph.assignment.is_none()
            && graph.nodes[0].is_some_and(|node| supported(node.schema.operation)),
        _ => false,
    }
}
// ------------------------=
// FUNC: execute
// DESC: Uses the one native parser and operation schemas, then returns actual bounded IOP completion instead of command recognition.
// ------------------=
pub fn execute(runtime: &mut InfinityRuntime, user: StableId, session: StableId, command: &[u8], now: u64) -> Outcome {
    if !runtime.identity.console_session_authorized(session, user) { return Outcome::failure(Diagnostic::Denied); }
    let mut result = Outcome::new();
    if super::console_help::requested(command) {
        if !super::console_help::render(command, &mut result.stdout) { result.status=2; result.diagnostic=Diagnostic::Syntax; }
        if result.stdout.truncated(){result.status=75;result.diagnostic=Diagnostic::Capacity;result.stderr.write_line(b"Result exceeds the output budget. Narrow the help topic or search.");}
        return result;
    }
    let parsed = match language::parse(command) { Ok(parsed) => parsed, Err(_) => return Outcome::failure(Diagnostic::Syntax) };
    match parsed {
        ParseOutcome::Help(domain, operation) => render_help(&mut result.stdout, domain, operation),
        ParseOutcome::Graph(graph) if graph.node_count == 1 && !graph.plan_only && graph.assignment.is_none() => {
            let Some(node) = graph.nodes[0] else { return Outcome::failure(Diagnostic::Syntax); };
            if !supported(node.schema.operation) { return Outcome::failure(Diagnostic::Denied); }
            result.operation = Some(node.schema.operation);
            if let Err(error) = dispatch(runtime, user, session, node.schema.operation, now, &mut result.stdout) {
                return Outcome::failure(match error { IopError::AccessDenied => Diagnostic::Denied,
                    IopError::DeadlineExceeded => Diagnostic::Deadline, IopError::Cancelled => Diagnostic::Cancelled,
                    IopError::Backpressure | IopError::PayloadTooLarge => Diagnostic::Capacity, _ => Diagnostic::Unavailable });
            }
        }
        _ => return Outcome::failure(Diagnostic::Unsupported),
    }
    if result.stdout.overflow { return Outcome::failure(Diagnostic::Capacity); }
    result
}
// ------------------------=
// FUNC: dispatch
// DESC: Issues and retires one nondelegable exact-operation capability, validates IOP and live identity again, and invokes shared handlers.
// ------------------=
fn dispatch(runtime: &mut InfinityRuntime, user: StableId, session: StableId, operation: OperationId, now: u64, output: &mut Bytes<OUTPUT_BYTES>) -> Result<(), IopError> {
    let owner = runtime.service_identity(SERVICE_CONSOLE).ok_or(IopError::UnknownEndpoint)?;
    let caller = SecurityIdentity(session.0);
    runtime.iop.ensure_owned_endpoint(0xe501, owner)?;
    let id = runtime.iop.next_node_request()?;
    let deadline = now.checked_add(2).ok_or(IopError::DeadlineExceeded)?;
    let grant = runtime.capabilities.grant(CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, owner, caller, Some(deadline), 0)?;
    let result = (|| {
        let message = IopMessage::request(operation, id, caller, grant, deadline, id, &[])?;
        runtime.iop.send(0xe501, message, &runtime.capabilities, now)?;
        let admitted = runtime.iop.receive(0xe501, now)?;
        if admitted.header.request_id != id || admitted.header.caller_identity != caller
            || !runtime.identity.console_session_authorized(session, user) { return Err(IopError::AccessDenied); }
        runtime.capabilities.validate(grant, caller, CapabilityType::ServiceCall, operation.machine_id() as u64, 1, 0, now)?;
        match operation {
            OperationId::SystemStatus => render_system(output, runtime.console_environment),
            OperationId::IntentResolve => render_ai(output),
            OperationId::NodeList => {
                let active = (0..super::identity::MAX_SESSIONS).filter_map(|i| runtime.identity.session_nth(i))
                    .find(|candidate| candidate.id == session).ok_or(IopError::AccessDenied)?;
                if active.origin != SessionOrigin::Ssh && !runtime.identity.is_administrator(user) { return Err(IopError::AccessDenied); }
                let mut bytes = [0; 640];
                let length = super::node::reconciliation::collect(super::node::reconciliation::request([0; 32], operation),
                    &mut bytes, &mut |request| super::node_client::read(runtime, request, now))?;
                if length % 32 != 0 { return Err(IopError::InvalidPayload); }
                render_nodes(output, &bytes[..length]);
            }
            _ => return Err(IopError::AccessDenied),
        }
        Ok(())
    })();
    let retired = runtime.capabilities.retire_leaf(grant, owner);
    if retired.is_err() { return Err(IopError::AccessDenied); }
    result
}

// ------------------------=
// FUNC: render_system
// DESC: Shares the native system status handler with graphical and headless callers.
// ------------------=
pub fn render_system(output: &mut Bytes<OUTPUT_BYTES>, environment: Environment) {

        output.write_line(b"InfinityOS System Status");
        output
            .write_segments(&[b"Architecture: ", environment.architecture]);
        output.write_line(b"Kernel: online");
        output.write_line(b"Drivers: online");
        output.write_line(b"Intent Runtime: online");
        output.write_segments(&[
            b"Ready devices: ",
            &[b'0' + environment.ready_devices as u8],
            b"/3",
        ]);
    }

// ------------------------=
// FUNC: render_ai
// DESC: Reads existing AI status metrics without submitting inference or granting access to AI operations.
// ------------------=
pub fn render_ai(output: &mut Bytes<OUTPUT_BYTES>) {
            crate::runtime::ai::with_ai_runtime(|ai| {
                output.write_line(if ai.initialized() {
                    b"Infinity AI Service: ready"
                } else {
                    b"Infinity AI Service: degraded"
                });
                output.write_line(b"Default policy: local-only");
                output.write_line(b"CPU backend: TESTED");
                output.write_line(b"GPU/NPU backends: UNSUPPORTED");
                output
                    .write_number(b"Inference queue depth: ", ai.queue_depth() as u64);
                output
                    .write_number(b"Completed inference: ", ai.inference_count());
                output
                    .write_number(b"Inference failures: ", ai.inference_failures());
                output.write_number(b"Native Qwen output tokens: ",ai.qwen_tokens);
                output.write_number(b"Native Qwen decode milliseconds: ",ai.qwen_decode_ns/1_000_000);
                output.write_number(b"Model load milliseconds: ",ai.qwen_metrics.load_ns/1_000_000);
                output.write_number(b"First token milliseconds: ",ai.qwen_metrics.first_token_ns/1_000_000);
                output.write_number(b"Prefill compute milliseconds: ",ai.qwen_metrics.prefill_work_ns/1_000_000);
                output.write_number(b"Decode compute milliseconds: ",ai.qwen_metrics.decode_work_ns/1_000_000);
                output.write_number(b"Maximum slice microseconds: ",ai.qwen_metrics.max_slice_ns/1_000);
                output.write_number(b"Maximum AI pump microseconds: ",ai.qwen_metrics.max_pump_ns/1_000);
                output.write_number(b"Cached prompt tokens: ",ai.qwen_metrics.reused_tokens as u64);
                output.write_number(b"Online inference workers: ",crate::runtime::ai::qwen::workers::online() as u64);
                output.write_number(b"Completed worker jobs: ",crate::runtime::ai::qwen::workers::completed() as u64);
                output.write_number(b"New prompt tokens: ",ai.qwen_metrics.prefill_tokens as u64);
            });
}

// ------------------------=
// FUNC: render_help
// DESC: Renders the existing native schema documentation without registering a second command vocabulary.
// ------------------=
pub fn render_help(output: &mut Bytes<OUTPUT_BYTES>, domain: Option<&language::DomainSchema>, operation: Option<&language::OperationSchema>) {
                if let Some(operation) = operation {
                    output
                        .write_segments(&[operation.domain, b" ", operation.action]);
                    output.write_line(operation.description);
                    if operation.target.is_some() { output.write_line(b"Target: typed reference (see example)"); }
                    for argument in operation.arguments {
                        output.write_segments(&[b"  ",argument.name,if argument.required {b"=<value> (required)"}else{b"=<value> (optional)"}]);
                    }
                    output.write_number(b"Required capability: ",operation.capability);
                    output.write_line(if operation.side_effect==language::SideEffectClass::Query {b"Effect: read only"}else{b"Effect: changes state; native authorization applies"});
                    output
                        .write_segments(&[b"Result type: ", value_type_text(operation.output)]);
                    output
                        .write_segments(&[b"Example: ", operation.example]);
                } else if let Some(domain) = domain {
                    output
                        .write_segments(&[domain.name, b" - ", domain.description]);
                    for operation in language::OPERATIONS
                        .iter()
                        .filter(|item| item.domain == domain.name)
                    {
                        output.write_segments(&[
                            b"  ",
                            operation.action,
                            b" - ",
                            operation.description,
                        ]);
                    }
                } else {
                    for domain in language::DOMAINS { output.write_segments(&[domain.name,b" - ",domain.description]); }
                    output.write_line(b"Use help commands, help DOMAIN ACTION, or help search WORD.");
                }
}

// ------------------------=
// FUNC: value_type_text
// DESC: Preserves the shared typed result names used by Console schema discovery.
// ------------------=
pub fn value_type_text(value: crate::runtime::console_language::ValueType) -> &'static [u8] {
    use crate::runtime::console_language::ValueType::*;
    match value {
        Unit => b"Unit",
        SystemStatus => b"SystemStatus",
        DeviceSet => b"DeviceSet",
        StorageStatus => b"StorageStatus",
        Object => b"Object",
        ObjectSet => b"ObjectSet",
        NamespaceResult => b"NamespaceResult",
        Project => b"Project",
        ProjectSet => b"ProjectSet",
        Collection => b"Collection",
        CollectionSet => b"CollectionSet",
        ServiceSet => b"ServiceSet",
        RuntimeContextSet => b"RuntimeContextSet",
        CapabilitySet => b"CapabilitySet",
        EventSet => b"EventSet",
        ModelSet => b"ModelSet",
        AgentSet => b"AgentSet",
        VoiceStatus => b"VoiceStatus",
        OperationPlan => b"OperationPlan",
        User => b"User",
        UserSet => b"UserSet",
        Group => b"Group",
        GroupSet => b"GroupSet",
        MembershipSet => b"MembershipSet",
        Machine => b"Machine",
        CredentialSet => b"CredentialSet",
        Session => b"Session",
        SessionSet => b"SessionSet",
        Profile => b"Profile",
        PersonalSpace => b"PersonalSpace",
        Settings => b"Settings",
        SkinSet => b"SkinSet",
        UiTree => b"UiTree",
        WindowSet => b"WindowSet",
        ClipboardData => b"ClipboardData",
        NetworkStatus => b"NetworkStatus",
        NetworkInterfaceSet => b"NetworkInterfaceSet",
        NetworkAddressSet => b"NetworkAddressSet",
        NetworkRouteSet => b"NetworkRouteSet",
        NetworkConnectionSet => b"NetworkConnectionSet",
        NetworkPolicySet => b"NetworkPolicySet",
        NetworkProfileSet => b"NetworkProfileSet",
        NetworkDiagnostics => b"NetworkDiagnostics",
        ServiceDiscoverySet => b"ServiceDiscoverySet",
        NodeSet => b"NodeSet",
        NodeSessionSet => b"NodeSessionSet",
        NodePolicy => b"NodePolicy",
        NodeAuditSet => b"NodeAuditSet",
        MeshDomainSet => b"MeshDomainSet",
    }
}


// ------------------------=
// FUNC: render_nodes
// DESC: Renders the existing bounded Node.List collection without exposing mutation authority.
// ------------------=
pub fn render_nodes(output: &mut Bytes<OUTPUT_BYTES>, bytes: &[u8]) {
    for id in bytes.chunks_exact(32) { output.write_hex(b"Identity: ", id); }
    if bytes.is_empty() { output.write_line(b"No records."); }
}

impl<const N: usize> core::fmt::Write for Bytes<N> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Adapts bounded native output to allocation-free structured formatting.
    // ------------------=
    fn write_str(&mut self,text:&str)->core::fmt::Result {self.write(text.as_bytes());if self.overflow{Err(core::fmt::Error)}else{Ok(())}}
}
