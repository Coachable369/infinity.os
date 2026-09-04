use super::{broker::ContextEnvelope, types::*};
use crate::runtime::iop::OperationId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Consequence {
    ReadOnly,
    ReversibleMutation,
    Destructive,
    CapabilityGrant,
    ExternalDisclosure,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfirmationRequirement {
    None,
    Explicit,
    RemoteProcessingApproval,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlannedOperation {
    pub operation: OperationId,
    pub consequence: Consequence,
    pub reversible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IntentPlan {
    pub operations: [Option<PlannedOperation>; 4],
    pub operation_count: u8,
    pub arguments: [u8; 64],
    pub arguments_length: u8,
    pub object_refs: [[u8; 16]; 4],
    pub object_count: u8,
    pub confidence_milli: u16,
    pub ambiguous: bool,
    pub required_capability_types: u64,
    pub confirmation: ConfirmationRequirement,
    pub correlation_id: u64,
    pub context_classes: u32,
}

impl IntentPlan {
    // ------------------------=
    // FUNC: from_model_result
    // DESC: Converts model output into a closed typed plan without executing it.
    // ------------------=
    pub fn from_model_result(
        result: ModelExecutionResult,
        context: ContextEnvelope,
    ) -> Result<Self, AiError> {
        let operation = result.intent.operation().ok_or(AiError::LowConfidence)?;
        Ok(Self {
            operations: [
                Some(PlannedOperation {
                    operation,
                    consequence: Consequence::ReadOnly,
                    reversible: true,
                }),
                None,
                None,
                None,
            ],
            operation_count: 1,
            arguments: [0; 64],
            arguments_length: 0,
            object_refs: context.object_refs,
            object_count: context.object_count,
            confidence_milli: result.confidence_milli,
            ambiguous: result.confidence_milli < 700,
            required_capability_types: 0,
            confirmation: ConfirmationRequirement::None,
            correlation_id: result.correlation_id,
            context_classes: context.classes,
        })
    }
}

pub struct ConsequencePolicy;

impl ConsequencePolicy {
    // ------------------------=
    // FUNC: confirmation_for
    // DESC: Assigns confirmation from OS consequence semantics rather than model preference.
    // ------------------=
    pub const fn confirmation_for(consequence: Consequence) -> ConfirmationRequirement {
        match consequence {
            Consequence::ReadOnly | Consequence::ReversibleMutation => {
                ConfirmationRequirement::None
            }
            Consequence::Destructive | Consequence::CapabilityGrant => {
                ConfirmationRequirement::Explicit
            }
            Consequence::ExternalDisclosure => ConfirmationRequirement::RemoteProcessingApproval,
        }
    }

    // ------------------------=
    // FUNC: validate
    // DESC: Rejects a plan whose declared confirmation understates its consequences.
    // ------------------=
    pub fn validate(plan: &IntentPlan) -> Result<(), AiError> {
        for operation in plan.operations[..plan.operation_count as usize]
            .iter()
            .flatten()
        {
            let required = Self::confirmation_for(operation.consequence);
            if required != ConfirmationRequirement::None && plan.confirmation != required {
                return Err(AiError::AccessDenied);
            }
        }
        Ok(())
    }
}
