use super::types::{AgentId, AiError, ModelId, ProviderId, ResourcePolicy};
use crate::runtime::{execution::SecurityIdentity, iop::OperationId};

pub const MAX_AGENTS: usize = 4;
pub const MAX_AGENT_TOOLS: usize = 8;
pub const MAX_AGENT_TASKS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgentState {
    Defined,
    Ready,
    Running,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AgentDescriptor {
    pub id: AgentId,
    pub identity: SecurityIdentity,
    pub purpose: u32,
    pub model: ModelId,
    pub provider: ProviderId,
    pub allowed_tools: [Option<OperationId>; MAX_AGENT_TOOLS],
    pub resource_policy: ResourcePolicy,
    pub state: AgentState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AgentTask {
    pub id: u64,
    pub agent: AgentId,
    pub operation: OperationId,
    pub correlation_id: u64,
    pub deadline: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AgentTaskResult {
    pub task_id: u64,
    pub agent: AgentId,
    pub operation: OperationId,
    pub correlation_id: u64,
    pub success: bool,
}

pub struct AgentManager {
    agents: [Option<AgentDescriptor>; MAX_AGENTS],
    tasks: [Option<AgentTask>; MAX_AGENT_TASKS],
}

impl AgentManager {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded native agent registry and task queue.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            agents: [None; MAX_AGENTS],
            tasks: [None; MAX_AGENT_TASKS],
        }
    }

    // ------------------------=
    // FUNC: define
    // DESC: Defines an agent with explicit tools and an independent security identity.
    // ------------------=
    pub fn define(&mut self, agent: AgentDescriptor) -> Result<(), AiError> {
        let slot = self
            .agents
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(AiError::RegistryFull)?;
        *slot = Some(agent);
        Ok(())
    }

    // ------------------------=
    // FUNC: request_task
    // DESC: Enqueues a typed agent task only when the operation is explicitly allowed.
    // ------------------=
    pub fn request_task(&mut self, task: AgentTask, now: u64) -> Result<(), AiError> {
        if now >= task.deadline {
            return Err(AiError::DeadlineExceeded);
        }
        let agent = self
            .agents
            .iter_mut()
            .flatten()
            .find(|agent| agent.id == task.agent)
            .ok_or(AiError::InvalidRequest)?;
        if !agent.allowed_tools.contains(&Some(task.operation)) {
            return Err(AiError::AccessDenied);
        }
        let agent_queue_limit = (agent.resource_policy.queue_limit as usize)
            .min(MAX_AGENT_TASKS)
            .max(1);
        if self
            .tasks
            .iter()
            .flatten()
            .filter(|queued| queued.agent == task.agent)
            .count()
            >= agent_queue_limit
        {
            return Err(AiError::QueueFull);
        }
        let slot = self
            .tasks
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(AiError::QueueFull)?;
        *slot = Some(task);
        agent.state = AgentState::Running;
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel_task
    // DESC: Removes a queued task by stable task identity.
    // ------------------=
    pub fn cancel_task(&mut self, id: u64) -> Result<(), AiError> {
        let task = self
            .tasks
            .iter_mut()
            .find(|entry| entry.map(|task| task.id) == Some(id))
            .ok_or(AiError::InvalidRequest)?;
        let agent_id = task.map(|task| task.agent).ok_or(AiError::InvalidRequest)?;
        *task = None;
        if !self
            .tasks
            .iter()
            .flatten()
            .any(|task| task.agent == agent_id)
        {
            if let Some(agent) = self
                .agents
                .iter_mut()
                .flatten()
                .find(|agent| agent.id == agent_id)
            {
                agent.state = AgentState::Ready;
            }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: complete_task
    // DESC: Completes a queued typed task and returns its machine-readable result envelope.
    // ------------------=
    pub fn complete_task(&mut self, id: u64, success: bool) -> Result<AgentTaskResult, AiError> {
        let task = self
            .tasks
            .iter_mut()
            .find(|entry| entry.map(|task| task.id) == Some(id))
            .and_then(Option::take)
            .ok_or(AiError::InvalidRequest)?;
        if !self
            .tasks
            .iter()
            .flatten()
            .any(|queued| queued.agent == task.agent)
        {
            if let Some(agent) = self
                .agents
                .iter_mut()
                .flatten()
                .find(|agent| agent.id == task.agent)
            {
                agent.state = if success {
                    AgentState::Ready
                } else {
                    AgentState::Failed
                };
            }
        }
        Ok(AgentTaskResult {
            task_id: task.id,
            agent: task.agent,
            operation: task.operation,
            correlation_id: task.correlation_id,
            success,
        })
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns typed agent metadata for human projection and audit.
    // ------------------=
    pub fn inspect(&self, id: AgentId) -> Option<&AgentDescriptor> {
        self.agents.iter().flatten().find(|agent| agent.id == id)
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the number of defined agents.
    // ------------------=
    pub fn count(&self) -> usize {
        self.agents.iter().flatten().count()
    }

    // ------------------------=
    // FUNC: queue_depth
    // DESC: Reports the bounded typed task queue depth.
    // ------------------=
    pub fn queue_depth(&self) -> usize {
        self.tasks.iter().flatten().count()
    }
}
