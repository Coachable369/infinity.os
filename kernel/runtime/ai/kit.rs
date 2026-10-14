//! Internal InfinityAIKit boundaries for requests, routing, policy, execution, and audit.
//! This module is transport-neutral and deliberately exposes no MCP surface.

use super::{control, mapping};

pub use super::broker::ContextBroker;
pub use super::provider::ProviderRouter as ProviderRegistry;

pub const AUDIT_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestOrigin {
    DesktopChat,
    Voice,
    Application(usize),
    Model,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestEnvelope<'a> {
    pub input: &'a [u8],
    pub owner: [u8; 16],
    pub origin: RequestOrigin,
    pub correlation_id: u64,
}

pub struct RequestApi {
    next_correlation_id: u64,
}

impl RequestApi {
    // ------------------------=
    // FUNC: new
    // DESC: Creates the transport-neutral AI request ingress with nonzero correlation sequencing.
    // ------------------=
    pub const fn new() -> Self { Self { next_correlation_id: 1 } }

    // ------------------------=
    // FUNC: accept
    // DESC: Wraps caller input and authority metadata in the canonical typed AIKit request envelope.
    // ------------------=
    pub fn accept<'a>(&mut self, input: &'a [u8], owner: [u8; 16], origin: RequestOrigin) -> RequestEnvelope<'a> {
        let correlation_id = self.next_correlation_id;
        self.next_correlation_id = self.next_correlation_id.saturating_add(1).max(1);
        RequestEnvelope { input, owner, origin, correlation_id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceMode {
    Conversation,
    DesktopTools,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestRoute {
    Control(control::Command),
    Inference(InferenceMode),
}

pub struct InferenceBroker;

impl InferenceBroker {
    // ------------------------=
    // FUNC: route
    // DESC: Chooses plain conversation or the bounded desktop-tool prompt without granting execution authority.
    // ------------------=
    pub fn route(input: &[u8]) -> InferenceMode {
        if control::veto_action(input) {
            InferenceMode::Conversation
        } else {
            InferenceMode::DesktopTools
        }
    }
}

pub struct IntentRouter;

impl IntentRouter {
    // ------------------------=
    // FUNC: route
    // DESC: Resolves installed aliases and deterministic commands before delegating unresolved requests to inference.
    // ------------------=
    pub fn route(mapping: &mut mapping::ConfiguredMap, request: &RequestEnvelope<'_>) -> RequestRoute {
        mapping.refresh();
        if let Some(command) = mapping.resolve(request.input).or_else(|| control::Command::explicit(request.input)) {
            RequestRoute::Control(command)
        } else {
            RequestRoute::Inference(InferenceBroker::route(request.input))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityFamily {
    Open,
    Launch,
    Focus,
    Close,
    Settings,
    Surface,
    Menu,
    Shell,
    Context,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub index: usize,
    pub target: &'static [u8],
    pub family: CapabilityFamily,
}

pub struct CapabilityDirectory;

impl CapabilityDirectory {
    // ------------------------=
    // FUNC: count
    // DESC: Reports the authoritative installed AI capability count without maintaining a second inventory.
    // ------------------=
    pub const fn count() -> usize { mapping::CAPABILITY_TARGETS.len() }

    // ------------------------=
    // FUNC: descriptor
    // DESC: Projects one authoritative mapping target into a typed discoverable capability descriptor.
    // ------------------=
    pub fn descriptor(index: usize) -> Option<CapabilityDescriptor> {
        let target = *mapping::CAPABILITY_TARGETS.get(index)?;
        Some(CapabilityDescriptor { index, target, family: capability_family(target)? })
    }

    // ------------------------=
    // FUNC: contains
    // DESC: Checks whether an exact typed target is part of the complete installed capability directory.
    // ------------------=
    pub fn contains(target: &[u8]) -> bool {
        mapping::CAPABILITY_TARGETS.contains(&target)
    }
}

// ------------------------=
// FUNC: capability_family
// DESC: Classifies an authoritative protocol target into its stable typed AIKit family.
// ------------------=
fn capability_family(target: &[u8]) -> Option<CapabilityFamily> {
    let families: [(&[u8], CapabilityFamily); 9] = [
        (b"OS_OPEN:", CapabilityFamily::Open),
        (b"OS_LAUNCH:", CapabilityFamily::Launch),
        (b"OS_FOCUS:", CapabilityFamily::Focus),
        (b"OS_CLOSE:", CapabilityFamily::Close),
        (b"OS_SETTINGS:", CapabilityFamily::Settings),
        (b"OS_SURFACE:", CapabilityFamily::Surface),
        (b"OS_MENU:", CapabilityFamily::Menu),
        (b"OS_SHELL:", CapabilityFamily::Shell),
        (b"OS_CONTEXT:", CapabilityFamily::Context),
    ];
    families.iter().find(|(prefix, _)| target.starts_with(prefix)).map(|(_, family)| *family)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyContext {
    pub requester: [u8; 16],
    pub owner: [u8; 16],
    pub chat_enabled: bool,
    pub desktop_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDenial {
    OwnerChanged,
    PermissionDenied,
    AiDisabled,
}

pub struct PolicyEngine;

impl PolicyEngine {
    // ------------------------=
    // FUNC: authorize
    // DESC: Rechecks authenticated ownership and current AI and desktop permissions immediately before dispatch.
    // ------------------=
    pub fn authorize(context: PolicyContext) -> Result<(), PolicyDenial> {
        if context.requester != context.owner { return Err(PolicyDenial::OwnerChanged); }
        if !context.chat_enabled { return Err(PolicyDenial::AiDisabled); }
        if !context.desktop_allowed { return Err(PolicyDenial::PermissionDenied); }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub command: control::Command,
    pub owner: [u8; 16],
    pub origin: RequestOrigin,
    pub correlation_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditOutcome {
    Staged,
    Authorized,
    Denied(PolicyDenial),
    Completed,
    Failed,
    Deferred,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub request: ExecutionRequest,
    pub outcome: AuditOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    Busy,
    NoActiveRequest,
    CommandMismatch,
}

pub struct AuditLog {
    records: [Option<AuditRecord>; AUDIT_CAPACITY],
    cursor: usize,
    count: usize,
}

impl AuditLog {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded structured AI execution audit log.
    // ------------------=
    pub const fn new() -> Self { Self { records: [None; AUDIT_CAPACITY], cursor: 0, count: 0 } }

    // ------------------------=
    // FUNC: append
    // DESC: Appends one lifecycle event while evicting only the oldest record at fixed capacity.
    // ------------------=
    pub fn append(&mut self, record: AuditRecord) {
        self.records[self.cursor] = Some(record);
        self.cursor = (self.cursor + 1) % AUDIT_CAPACITY;
        self.count = self.count.saturating_add(1).min(AUDIT_CAPACITY);
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the bounded number of retained structured audit records.
    // ------------------=
    pub const fn count(&self) -> usize { self.count }

    // ------------------------=
    // FUNC: record
    // DESC: Reads retained audit records in chronological order without exposing mutable storage.
    // ------------------=
    pub fn record(&self, index: usize) -> Option<AuditRecord> {
        if index >= self.count { return None; }
        let oldest = if self.count < AUDIT_CAPACITY { 0 } else { self.cursor };
        self.records[(oldest + index) % AUDIT_CAPACITY]
    }
}

pub struct ExecutionBroker {
    pending: Option<ExecutionRequest>,
    active: Option<ExecutionRequest>,
    audit: AuditLog,
}

impl ExecutionBroker {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty exact-once execution broker with a bounded in-memory audit trail.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: None, active: None, audit: AuditLog::new() }
    }

    // ------------------------=
    // FUNC: stage
    // DESC: Stages one typed command for later policy-authorized native dispatch.
    // ------------------=
    pub fn stage(&mut self, request: ExecutionRequest) -> Result<(), ExecutionError> {
        if self.pending.is_some() || self.active.is_some() { return Err(ExecutionError::Busy); }
        self.pending = Some(request);
        self.record(request, AuditOutcome::Staged);
        Ok(())
    }

    // ------------------------=
    // FUNC: pending
    // DESC: Reports whether a command awaits the final session and permission policy check.
    // ------------------=
    pub const fn pending(&self) -> bool { self.pending.is_some() }

    // ------------------------=
    // FUNC: busy
    // DESC: Reports whether a command is waiting for policy or is currently owned by native dispatch.
    // ------------------=
    pub const fn busy(&self) -> bool { self.pending.is_some() || self.active.is_some() }

    // ------------------------=
    // FUNC: take
    // DESC: Moves one authorized request into active dispatch exactly once and audits any denial.
    // ------------------=
    pub fn take(&mut self, context: PolicyContext) -> Result<Option<ExecutionRequest>, PolicyDenial> {
        let Some(request) = self.pending.take() else { return Ok(None); };
        if context.owner != request.owner {
            self.record(request, AuditOutcome::Denied(PolicyDenial::OwnerChanged));
            return Err(PolicyDenial::OwnerChanged);
        }
        if let Err(denial) = PolicyEngine::authorize(context) {
            self.record(request, AuditOutcome::Denied(denial));
            return Err(denial);
        }
        self.active = Some(request);
        self.record(request, AuditOutcome::Authorized);
        Ok(Some(request))
    }

    // ------------------------=
    // FUNC: finish
    // DESC: Completes the matching active native dispatch and records its verified outcome.
    // ------------------=
    pub fn finish(&mut self, command: control::Command, success: bool) -> Result<(), ExecutionError> {
        let request = self.active.take().ok_or(ExecutionError::NoActiveRequest)?;
        if request.command != command {
            self.active = Some(request);
            return Err(ExecutionError::CommandMismatch);
        }
        self.record(request, if success { AuditOutcome::Completed } else { AuditOutcome::Failed });
        Ok(())
    }

    // ------------------------=
    // FUNC: defer
    // DESC: Releases a matching active request when a native confirmation or save workflow owns completion.
    // ------------------=
    pub fn defer(&mut self, command: control::Command) -> Result<(), ExecutionError> {
        let request = self.active.take().ok_or(ExecutionError::NoActiveRequest)?;
        if request.command != command {
            self.active = Some(request);
            return Err(ExecutionError::CommandMismatch);
        }
        self.record(request, AuditOutcome::Deferred);
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels pending and active requests so ownership or permission changes cannot leak authority.
    // ------------------=
    pub fn cancel(&mut self) -> bool {
        let mut cancelled = false;
        if let Some(request) = self.pending.take() {
            self.record(request, AuditOutcome::Cancelled);
            cancelled = true;
        }
        if let Some(request) = self.active.take() {
            self.record(request, AuditOutcome::Cancelled);
            cancelled = true;
        }
        cancelled
    }

    // ------------------------=
    // FUNC: audit_count
    // DESC: Reports the bounded number of currently retained structured audit records.
    // ------------------=
    pub const fn audit_count(&self) -> usize { self.audit.count() }

    // ------------------------=
    // FUNC: audit
    // DESC: Reads retained audit records in chronological order without exposing mutable storage.
    // ------------------=
    pub fn audit(&self, index: usize) -> Option<AuditRecord> {
        self.audit.record(index)
    }

    // ------------------------=
    // FUNC: record
    // DESC: Appends one structured lifecycle event while evicting only the oldest bounded audit record.
    // ------------------=
    fn record(&mut self, request: ExecutionRequest, outcome: AuditOutcome) {
        self.audit.append(AuditRecord { request, outcome });
    }
}
