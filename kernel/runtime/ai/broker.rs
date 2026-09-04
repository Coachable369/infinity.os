use super::types::AiError;
use crate::runtime::{
    capability::{CapabilityId, CapabilityManager, CapabilityType},
    execution::SecurityIdentity,
    iop::OperationId,
};

pub const CONTEXT_SYSTEM_STATE: u32 = 1 << 0;
pub const CONTEXT_SELECTED_DEVICES: u32 = 1 << 1;
pub const CONTEXT_NAMESPACE: u32 = 1 << 2;
pub const CONTEXT_PERSONAL_OBJECTS: u32 = 1 << 3;
pub const CONTEXT_CONVERSATION: u32 = 1 << 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContextEnvelope {
    pub classes: u32,
    pub object_refs: [[u8; 16]; 4],
    pub object_count: u8,
    pub correlation_id: u64,
}

pub struct ContextBroker;

impl ContextBroker {
    // ------------------------=
    // FUNC: request
    // DESC: Builds only the explicitly authorized typed context classes.
    // ------------------=
    pub fn request(
        requested: u32,
        authorized: u32,
        correlation_id: u64,
    ) -> Result<ContextEnvelope, AiError> {
        if requested & !authorized != 0 {
            return Err(AiError::AccessDenied);
        }
        Ok(ContextEnvelope {
            classes: requested,
            object_refs: [[0; 16]; 4],
            object_count: 0,
            correlation_id,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ToolInvocation {
    pub operation: OperationId,
    pub target: u64,
    pub rights: u32,
    pub constraints: u64,
    pub capability: CapabilityId,
    pub caller: SecurityIdentity,
}

pub struct ToolBroker;

impl ToolBroker {
    // ------------------------=
    // FUNC: validate
    // DESC: Validates a closed typed tool invocation with no shell or raw-memory escape hatch.
    // ------------------=
    pub fn validate(
        invocation: &ToolInvocation,
        capabilities: &CapabilityManager,
        now: u64,
    ) -> Result<(), AiError> {
        let kind = match invocation.operation {
            OperationId::SystemStatus
            | OperationId::SystemInfo
            | OperationId::SystemBootStatus
            | OperationId::MemoryStatus => CapabilityType::SystemInspect,
            OperationId::DeviceList => CapabilityType::DeviceInspect,
            OperationId::ObjectRead | OperationId::ObjectQuery => CapabilityType::ObjectRead,
            OperationId::NamespaceResolve => CapabilityType::NamespaceRead,
            OperationId::ServiceInspect => CapabilityType::ServiceInspect,
            _ => return Err(AiError::AccessDenied),
        };
        capabilities
            .validate(
                invocation.capability,
                invocation.caller,
                kind,
                invocation.target,
                invocation.rights,
                invocation.constraints,
                now,
            )
            .map_err(|_| AiError::AccessDenied)
    }
}
