//! IntelligenceKit v1: bounded goal planning and verified orchestration.
//!
//! The service deliberately contains no model, command dispatcher, storage
//! implementation, or distributed scheduler. It discovers those facilities
//! through adapters and treats every generated plan as untrusted data until
//! native authorization and execution adapters accept each step.

#[path = "software_execution.rs"]
pub mod software;

pub const API_VERSION: u16 = 1;
pub const STATE_SCHEMA_VERSION: u16 = 1;
pub const MAX_GOALS: usize = 4;
pub const MAX_TASKS: usize = 16;
pub const MAX_DEPENDENCIES: usize = 4;
pub const MAX_EVENTS: usize = 32;
pub const MAX_OBJECTIVE_BYTES: usize = 512;
pub const MAX_TEXT_BYTES: usize = 192;
pub const MAX_STATE_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestOrigin {
    Voice,
    Text,
    Application(usize),
    NativeService,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestClass {
    Conversation,
    SimpleAction,
    ComplexGoal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityId {
    AiAction(u16),
    DevKit,
    ObjectStorage,
    UiServices,
    InfinityPool,
    ApplicationInteraction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contract {
    None,
    Text,
    ObjectReference,
    ApplicationArtifact,
    ExecutionReceipt,
    ComputeResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    ReadOnly,
    Reversible,
    Sensitive,
    Destructive,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Ready,
    Busy,
    Offline,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub input: Contract,
    pub output: Contract,
    pub required_permission: u32,
    pub risk: Risk,
    pub reversible: bool,
    pub dependency: Option<CapabilityId>,
    pub availability: Availability,
}

pub trait CapabilityRegistry {
    // ------------------------=
    // FUNC: describe
    // DESC: Returns the current typed contract for one installed native capability.
    // ------------------=
    fn describe(&self, id: CapabilityId) -> Option<CapabilityDescriptor>;

    // ------------------------=
    // FUNC: resolve_action
    // DESC: Resolves human action words through an existing authoritative action directory.
    // ------------------=
    fn resolve_action(&self, words: &[u8]) -> Option<CapabilityId>;
}

pub struct InstalledCapabilities;

impl CapabilityRegistry for InstalledCapabilities {
    // ------------------------=
    // FUNC: describe
    // DESC: Projects authoritative AIKit, DevKit, storage, UIKit, and Pool contracts without maintaining a parallel action inventory.
    // ------------------=
    fn describe(&self, id: CapabilityId) -> Option<CapabilityDescriptor> {
        let descriptor = match id {
            CapabilityId::AiAction(index) => {
                super::kit::CapabilityDirectory::descriptor(index as usize)?;
                CapabilityDescriptor {
                    id,
                    input: Contract::Text,
                    output: Contract::ExecutionReceipt,
                    required_permission: 1,
                    risk: Risk::Reversible,
                    reversible: true,
                    dependency: None,
                    availability: Availability::Ready,
                }
            }
            CapabilityId::DevKit => CapabilityDescriptor {
                id,
                input: Contract::Text,
                output: Contract::ApplicationArtifact,
                required_permission: 2,
                risk: Risk::Reversible,
                reversible: true,
                dependency: Some(CapabilityId::ObjectStorage),
                availability: Availability::Ready,
            },
            CapabilityId::ObjectStorage => CapabilityDescriptor {
                id,
                input: Contract::ObjectReference,
                output: Contract::ObjectReference,
                required_permission: 4,
                risk: Risk::Sensitive,
                reversible: true,
                dependency: None,
                availability: Availability::Ready,
            },
            CapabilityId::UiServices => CapabilityDescriptor {
                id,
                input: Contract::ApplicationArtifact,
                output: Contract::ExecutionReceipt,
                required_permission: 8,
                risk: Risk::Reversible,
                reversible: true,
                dependency: None,
                availability: Availability::Ready,
            },
            CapabilityId::InfinityPool => CapabilityDescriptor {
                id,
                input: Contract::ObjectReference,
                output: Contract::ComputeResult,
                required_permission: 16,
                risk: Risk::External,
                reversible: false,
                dependency: None,
                availability: Availability::Ready,
            },
            CapabilityId::ApplicationInteraction => CapabilityDescriptor {
                id,
                input: Contract::ExecutionReceipt,
                output: Contract::ExecutionReceipt,
                required_permission: 32,
                risk: Risk::Reversible,
                reversible: true,
                dependency: Some(CapabilityId::UiServices),
                availability: Availability::Ready,
            },
        };
        Some(descriptor)
    }

    // ------------------------=
    // FUNC: resolve_action
    // DESC: Resolves clauses through the existing command decoder and authoritative AIKit target list.
    // ------------------=
    fn resolve_action(&self, words: &[u8]) -> Option<CapabilityId> {
        let command = super::control::Command::explicit(words)?;
        super::mapping::CAPABILITY_TARGETS
            .iter()
            .position(|target| super::control::Command::decode(target) == Some(command))
            .map(|index| CapabilityId::AiAction(index as u16))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedText<const N: usize> {
    bytes: [u8; N],
    length: u16,
}

impl<const N: usize> BoundedText<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Copies one bounded text value without allocation or silent truncation.
    // ------------------=
    pub fn new(value: &[u8]) -> Option<Self> {
        if value.len() > N || value.len() > u16::MAX as usize {
            return None;
        }
        let mut text = Self {
            bytes: [0; N],
            length: value.len() as u16,
        };
        text.bytes[..value.len()].copy_from_slice(value);
        Some(text)
    }

    // ------------------------=
    // FUNC: empty
    // DESC: Creates a stable empty bounded text value.
    // ------------------=
    pub const fn empty() -> Self {
        Self {
            bytes: [0; N],
            length: 0,
        }
    }

    // ------------------------=
    // FUNC: as_bytes
    // DESC: Borrows only initialized text bytes.
    // ------------------=
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    pub id: u64,
    pub owner: [u8; 16],
    pub origin: RequestOrigin,
    pub objective: BoundedText<MAX_OBJECTIVE_BYTES>,
    pub desired_outcome: BoundedText<MAX_TEXT_BYTES>,
    pub success_criteria: BoundedText<MAX_TEXT_BYTES>,
    pub constraints: BoundedText<MAX_TEXT_BYTES>,
    pub context: BoundedText<MAX_TEXT_BYTES>,
    pub missing_information: BoundedText<MAX_TEXT_BYTES>,
    pub required_permissions: u32,
    pub created_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePolicy {
    Stop,
    RetryOnce,
    ReplanAffected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    WaitingApproval,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: u16,
    pub description: BoundedText<MAX_TEXT_BYTES>,
    pub capability: CapabilityId,
    pub dependencies: [u16; MAX_DEPENDENCIES],
    pub dependency_count: u8,
    pub inputs: BoundedText<MAX_TEXT_BYTES>,
    pub expected_output: Contract,
    pub verification: BoundedText<MAX_TEXT_BYTES>,
    pub state: TaskState,
    pub failure_policy: FailurePolicy,
    pub attempts: u8,
    pub execution_token: u64,
    pub evidence: [u8; 16],
}

impl Task {
    // ------------------------=
    // FUNC: depends_on
    // DESC: Checks one task dependency without exposing unused bounded slots.
    // ------------------=
    pub fn depends_on(&self, task: u16) -> bool {
        self.dependencies[..self.dependency_count as usize].contains(&task)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanState {
    Proposed,
    AwaitingApproval,
    Executing,
    Interrupted,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionLimits {
    pub maximum_tasks: u8,
    pub maximum_parallel: u8,
    pub maximum_replans: u8,
    pub deadline: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub goal: Goal,
    pub tasks: [Option<Task>; MAX_TASKS],
    pub task_count: u8,
    pub state: PlanState,
    pub limits: ExecutionLimits,
    pub replans: u8,
    pub revision: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Proposed,
    ApprovalRequired,
    Started,
    Progress,
    Verified,
    Recovery,
    Interrupted,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressEvent {
    pub sequence: u64,
    pub goal_id: u64,
    pub task_id: Option<u16>,
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceError {
    Full,
    InvalidGoal,
    InvalidPlan,
    NotFound,
    AccessDenied,
    ApprovalRequired,
    Busy,
    Deadline,
    Persistence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    Busy,
    AccessDenied,
    InvalidInput,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Running,
    Succeeded([u8; 16]),
    Failed(u16),
}

pub trait Executor {
    // ------------------------=
    // FUNC: authorize
    // DESC: Revalidates caller authority immediately before native execution.
    // ------------------=
    fn authorize(&self, owner: [u8; 16], capability: CapabilityDescriptor, now: u64) -> bool;

    // ------------------------=
    // FUNC: start
    // DESC: Starts one validated task and returns an exact execution lease.
    // ------------------=
    fn start(
        &mut self,
        goal: &Goal,
        task: &Task,
        capability: CapabilityDescriptor,
        now: u64,
    ) -> Result<u64, StartError>;

    // ------------------------=
    // FUNC: observe
    // DESC: Reads structured native execution state and verification evidence.
    // ------------------=
    fn observe(&mut self, token: u64, now: u64) -> Observation;

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels one exact active native execution lease.
    // ------------------=
    fn cancel(&mut self, token: u64);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanSummary {
    pub goal_id: u64,
    pub state: PlanState,
    pub completed: u8,
    pub total: u8,
    pub failed_task: Option<u16>,
}

pub struct Service {
    plans: [Option<Plan>; MAX_GOALS],
    events: [Option<ProgressEvent>; MAX_EVENTS],
    event_cursor: usize,
    event_count: usize,
    next_goal_id: u64,
    next_event_sequence: u64,
}

impl Service {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded IntelligenceKit service with stable nonzero identifiers.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            plans: [None; MAX_GOALS],
            events: [None; MAX_EVENTS],
            event_cursor: 0,
            event_count: 0,
            next_goal_id: 1,
            next_event_sequence: 1,
        }
    }

    // ------------------------=
    // FUNC: classify
    // DESC: Preserves direct-action latency and selects planning only for explicit multi-step or outcome-oriented requests.
    // ------------------=
    pub fn classify(input: &[u8], direct_action: bool) -> RequestClass {
        if direct_action {
            return RequestClass::SimpleAction;
        }
        let input = input.trim_ascii();
        if input.is_empty() {
            return RequestClass::Conversation;
        }
        let complex = contains_case(input, b" and then ")
            || contains_case(input, b" after that ")
            || contains_case(input, b"create and ")
            || contains_case(input, b"build and ")
            || starts_case(input, b"my goal is ")
            || starts_case(input, b"i need you to ")
            || crate::ui::app_assistant::editor_compile_intent::create_application(input);
        if complex {
            RequestClass::ComplexGoal
        } else {
            RequestClass::Conversation
        }
    }

    // ------------------------=
    // FUNC: propose
    // DESC: Converts an interpreted objective into a bounded dependency graph using discoverable native capabilities first.
    // ------------------=
    pub fn propose<R: CapabilityRegistry>(
        &mut self,
        owner: [u8; 16],
        origin: RequestOrigin,
        objective: &[u8],
        registry: &R,
        now: u64,
        limits: ExecutionLimits,
    ) -> Result<u64, ServiceError> {
        if objective.is_empty()
            || limits.maximum_tasks == 0
            || limits.maximum_tasks as usize > MAX_TASKS
            || limits.maximum_parallel == 0
            || limits.maximum_replans > 3
        {
            return Err(ServiceError::InvalidGoal);
        }
        let objective_text = BoundedText::new(objective).ok_or(ServiceError::InvalidGoal)?;
        let goal_id = self.next_goal_id;
        self.next_goal_id = self.next_goal_id.saturating_add(1).max(1);
        let software = crate::ui::app_assistant::editor_compile_intent::create_application(objective);
        let interaction = software && crate::ui::app_assistant::editor_compile_intent::requests_opponent(objective);
        let goal = Goal {
            id: goal_id,
            owner,
            origin,
            objective: objective_text,
            desired_outcome: BoundedText::new(if software {
                b"A compiled, launchable native application"
            } else {
                b"The requested objective is achieved"
            })
            .unwrap(),
            success_criteria: BoundedText::new(if software {
                b"Compiler succeeds and the application lifecycle reports a successful launch"
            } else {
                b"Every planned step returns native verification evidence"
            })
            .unwrap(),
            constraints: BoundedText::new(
                b"Use existing authorized InfinityOS capabilities before generating software",
            )
            .unwrap(),
            context: BoundedText::empty(),
            missing_information: BoundedText::empty(),
            required_permissions: 0,
            created_at: now,
        };
        let mut plan = Plan {
            goal,
            tasks: [None; MAX_TASKS],
            task_count: 0,
            state: PlanState::Proposed,
            limits,
            replans: 0,
            revision: 1,
        };
        if software {
            if registry.describe(CapabilityId::DevKit).is_none() {
                return Err(ServiceError::InvalidPlan);
            }
            push_task(
                &mut plan,
                CapabilityId::DevKit,
                b"Generate and compile the required native C application",
                b"Use this goal's original objective; preserve all requested functionality",
                Contract::ApplicationArtifact,
                b"Use compiler diagnostics and a completed native artifact",
                &[],
                FailurePolicy::Stop,
            )?;
            push_task(
                &mut plan,
                CapabilityId::UiServices,
                b"Launch the generated application",
                b"verified artifact",
                Contract::ExecutionReceipt,
                b"Observe the native application lifecycle launch event",
                &[1],
                FailurePolicy::Stop,
            )?;
            if interaction {
                push_task(
                    &mut plan,
                    CapabilityId::ApplicationInteraction,
                    b"Enable the requested AI interaction with the launched application",
                    b"Use the explicitly requested interaction in the original objective",
                    Contract::ExecutionReceipt,
                    b"Observe an owner-granted live application publishing its versioned action contract",
                    &[2],
                    FailurePolicy::Stop,
                )?;
            }
        } else if contains_case(objective, b"organize")
            || contains_case(objective, b"catalog")
            || contains_case(objective, b"archive")
        {
            push_task(
                &mut plan,
                CapabilityId::ObjectStorage,
                b"Inspect and update native objects needed by the objective",
                objective,
                Contract::ObjectReference,
                b"Verify committed object versions and namespace state",
                &[],
                FailurePolicy::ReplanAffected,
            )?;
        } else if contains_case(objective, b"distributed")
            || contains_case(objective, b"accelerator")
            || contains_case(objective, b"infinity pool")
        {
            push_task(
                &mut plan,
                CapabilityId::InfinityPool,
                b"Request bounded resources through Infinity Pool",
                objective,
                Contract::ComputeResult,
                b"Verify the native compute result and accounting record",
                &[],
                FailurePolicy::RetryOnce,
            )?;
        } else {
            let mut previous = None;
            for clause in clauses(objective) {
                if let Some(capability) = registry.resolve_action(clause) {
                    if registry.describe(capability).is_none() {
                        continue;
                    }
                    let deps = previous.map(|id| [id]).unwrap_or([0]);
                    let dependency_slice = if previous.is_some() {
                        &deps[..1]
                    } else {
                        &deps[..0]
                    };
                    push_task(
                        &mut plan,
                        capability,
                        b"Execute an existing InfinityOS capability",
                        clause,
                        Contract::ExecutionReceipt,
                        b"Require a successful native execution receipt",
                        dependency_slice,
                        FailurePolicy::RetryOnce,
                    )?;
                    previous = Some(plan.task_count as u16);
                }
            }
            if plan.task_count == 0 {
                return Err(ServiceError::InvalidPlan);
            }
        }
        plan.goal = goal;
        validate_plan(&plan, registry)?;
        for task in plan.tasks.iter().flatten() {
            plan.goal.required_permissions |= registry.describe(task.capability)
                .ok_or(ServiceError::InvalidPlan)?.required_permission;
        }
        let slot = self
            .plans
            .iter()
            .position(Option::is_none)
            .or_else(|| self.plans.iter().enumerate()
                .filter_map(|(index,entry)|entry.as_ref().filter(|plan|matches!(plan.state,
                    PlanState::Completed | PlanState::Failed | PlanState::Cancelled))
                    .map(|plan|(index,plan.goal.id)))
                .min_by_key(|(_,id)|*id).map(|(index,_)|index))
            .ok_or(ServiceError::Full)?;
        self.plans[slot] = Some(plan);
        self.emit(goal_id, None, EventKind::Proposed);
        Ok(goal_id)
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts an inspectable plan while leaving high-risk tasks paused for explicit approval.
    // ------------------=
    pub fn begin<R: CapabilityRegistry>(
        &mut self,
        goal_id: u64,
        owner: [u8; 16],
        registry: &R,
    ) -> Result<(), ServiceError> {
        let index = self.plan_index(goal_id, owner)?;
        let mut approval = false;
        let plan = self.plans[index].as_mut().unwrap();
        if !matches!(plan.state, PlanState::Proposed | PlanState::Interrupted) {
            return Err(ServiceError::Busy);
        }
        for task in plan.tasks.iter_mut().flatten() {
            if task.state == TaskState::Succeeded { continue; }
            let descriptor = registry
                .describe(task.capability)
                .ok_or(ServiceError::InvalidPlan)?;
            if matches!(
                descriptor.risk,
                Risk::Sensitive | Risk::Destructive | Risk::External
            ) {
                task.state = TaskState::WaitingApproval;
                approval = true;
            } else if task.dependency_count == 0 {
                task.state = TaskState::Ready;
            }
        }
        plan.state = if approval {
            PlanState::AwaitingApproval
        } else {
            PlanState::Executing
        };
        self.emit(
            goal_id,
            None,
            if approval {
                EventKind::ApprovalRequired
            } else {
                EventKind::Started
            },
        );
        Ok(())
    }

    // ------------------------=
    // FUNC: approve
    // DESC: Applies explicit owner approval to one waiting task without broadening any native capability grant.
    // ------------------=
    pub fn approve(
        &mut self,
        goal_id: u64,
        task_id: u16,
        owner: [u8; 16],
    ) -> Result<(), ServiceError> {
        let index = self.plan_index(goal_id, owner)?;
        let plan = self.plans[index].as_mut().unwrap();
        let task = task_mut(plan, task_id).ok_or(ServiceError::NotFound)?;
        if task.state != TaskState::WaitingApproval {
            return Err(ServiceError::InvalidPlan);
        }
        task.state = TaskState::Pending;
        plan.state = PlanState::Executing;
        self.emit(goal_id, Some(task_id), EventKind::Progress);
        Ok(())
    }

    // ------------------------=
    // FUNC: poll
    // DESC: Advances dependency-ready tasks asynchronously and trusts only executor observations as outcome evidence.
    // ------------------=
    pub fn poll<R: CapabilityRegistry, E: Executor>(
        &mut self,
        goal_id: u64,
        owner: [u8; 16],
        registry: &R,
        executor: &mut E,
        now: u64,
    ) -> Result<PlanSummary, ServiceError> {
        let index = self.plan_index(goal_id, owner)?;
        if self.plans[index].as_ref().unwrap().limits.deadline != 0
            && now >= self.plans[index].as_ref().unwrap().limits.deadline
        {
            self.fail_running(index, executor);
            self.plans[index].as_mut().unwrap().state = PlanState::Failed;
            self.emit(goal_id, None, EventKind::Failed);
            return Err(ServiceError::Deadline);
        }
        let mut emits = [(None, EventKind::Progress); MAX_TASKS];
        let mut emit_count = 0;
        {
            let plan = self.plans[index].as_mut().unwrap();
            if !matches!(
                plan.state,
                PlanState::Executing | PlanState::AwaitingApproval
            ) {
                return Ok(summary(plan));
            }
            for task_index in 0..plan.task_count as usize {
                let task = plan.tasks[task_index].as_mut().unwrap();
                if task.state == TaskState::Running {
                    match executor.observe(task.execution_token, now) {
                        Observation::Running => (),
                        Observation::Succeeded(evidence) => {
                            task.evidence = evidence;
                            task.state = TaskState::Succeeded;
                            emits[emit_count] = (Some(task.id), EventKind::Verified);
                            emit_count += 1;
                        }
                        Observation::Failed(_)
                            if task.attempts <= 1
                                && task.failure_policy != FailurePolicy::Stop
                                && plan.replans < plan.limits.maximum_replans =>
                        {
                            task.state = TaskState::Pending;
                            task.execution_token = 0;
                            plan.replans = plan.replans.saturating_add(1);
                            emits[emit_count] = (Some(task.id), EventKind::Recovery);
                            emit_count += 1;
                        }
                        Observation::Failed(_) => {
                            task.state = TaskState::Failed;
                            emits[emit_count] = (Some(task.id), EventKind::Failed);
                            emit_count += 1;
                        }
                    }
                }
            }
            refresh_ready(plan);
            let mut running = plan
                .tasks
                .iter()
                .flatten()
                .filter(|task| task.state == TaskState::Running)
                .count();
            for task_index in 0..plan.task_count as usize {
                if running >= plan.limits.maximum_parallel as usize {
                    break;
                }
                let task = plan.tasks[task_index].as_mut().unwrap();
                if task.state != TaskState::Ready {
                    continue;
                }
                let capability = registry
                    .describe(task.capability)
                    .ok_or(ServiceError::InvalidPlan)?;
                if capability.availability != Availability::Ready {
                    task.state = TaskState::Blocked;
                    continue;
                }
                if !executor.authorize(plan.goal.owner, capability, now) {
                    task.state = TaskState::Failed;
                    emits[emit_count] = (Some(task.id), EventKind::Failed);
                    emit_count += 1;
                    continue;
                }
                match executor.start(&plan.goal, task, capability, now) {
                    Ok(token) => {
                        task.execution_token = token;
                        task.attempts = task.attempts.saturating_add(1);
                        task.state = TaskState::Running;
                        running += 1;
                        emits[emit_count] = (Some(task.id), EventKind::Progress);
                        emit_count += 1;
                    }
                    Err(StartError::Busy) => (),
                    Err(_) => {
                        task.attempts = task.attempts.saturating_add(1);
                        task.state = TaskState::Failed;
                        emits[emit_count] = (Some(task.id), EventKind::Failed);
                        emit_count += 1;
                    }
                }
            }
            if plan
                .tasks
                .iter()
                .flatten()
                .any(|task| matches!(task.state, TaskState::Failed | TaskState::Blocked))
            {
                plan.state = PlanState::Failed;
            } else if plan
                .tasks
                .iter()
                .flatten()
                .all(|task| task.state == TaskState::Succeeded)
            {
                plan.state = PlanState::Completed;
            }
        }
        for (task, kind) in emits[..emit_count].iter().copied() {
            self.emit(goal_id, task, kind);
        }
        let state = self.plans[index].as_ref().unwrap().state;
        if state == PlanState::Completed {
            self.emit(goal_id, None, EventKind::Completed);
        } else if state == PlanState::Failed {
            self.emit(goal_id, None, EventKind::Failed);
        }
        Ok(summary(self.plans[index].as_ref().unwrap()))
    }

    // ------------------------=
    // FUNC: interrupt
    // DESC: Cancels running adapter work and checkpoints completed steps for safe later resumption.
    // ------------------=
    pub fn interrupt<E: Executor>(
        &mut self,
        goal_id: u64,
        owner: [u8; 16],
        executor: &mut E,
    ) -> Result<(), ServiceError> {
        let index = self.plan_index(goal_id, owner)?;
        let plan = self.plans[index].as_mut().unwrap();
        for task in plan.tasks.iter_mut().flatten() {
            if task.state == TaskState::Running {
                executor.cancel(task.execution_token);
                task.execution_token = 0;
                task.state = TaskState::Pending;
            }
        }
        plan.state = PlanState::Interrupted;
        self.emit(goal_id, None, EventKind::Interrupted);
        Ok(())
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Cancels one owner-bound goal and every active execution token.
    // ------------------=
    pub fn cancel<E: Executor>(
        &mut self,
        goal_id: u64,
        owner: [u8; 16],
        executor: &mut E,
    ) -> Result<(), ServiceError> {
        let index = self.plan_index(goal_id, owner)?;
        self.fail_running(index, executor);
        self.plans[index].as_mut().unwrap().state = PlanState::Cancelled;
        self.emit(goal_id, None, EventKind::Cancelled);
        Ok(())
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns an immutable owner-scoped plan for UI, console, or service projection.
    // ------------------=
    pub fn inspect(&self, goal_id: u64, owner: [u8; 16]) -> Option<&Plan> {
        self.plans
            .iter()
            .flatten()
            .find(|plan| plan.goal.id == goal_id && plan.goal.owner == owner)
    }

    // ------------------------=
    // FUNC: event
    // DESC: Reads retained progress events in chronological order.
    // ------------------=
    pub fn event(&self, index: usize) -> Option<ProgressEvent> {
        if index >= self.event_count {
            return None;
        }
        let oldest = if self.event_count < MAX_EVENTS {
            0
        } else {
            self.event_cursor
        };
        self.events[(oldest + index) % MAX_EVENTS]
    }

    // ------------------------=
    // FUNC: event_count
    // DESC: Reports retained structured progress event count.
    // ------------------=
    pub const fn event_count(&self) -> usize {
        self.event_count
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Encodes owner-bound goal state in a versioned architecture-neutral checkpoint format.
    // ------------------=
    pub fn encode(
        &self,
        goal_id: u64,
        owner: [u8; 16],
        out: &mut [u8],
    ) -> Result<usize, ServiceError> {
        const GOAL_BYTES: usize = 1552;
        const TASK_BYTES: usize = 640;
        let plan = self.inspect(goal_id, owner).ok_or(ServiceError::NotFound)?;
        let needed = GOAL_BYTES + plan.task_count as usize * TASK_BYTES;
        if needed > out.len() {
            return Err(ServiceError::Persistence);
        }
        out[..needed].fill(0);
        out[..4].copy_from_slice(b"IKIT");
        put_u16(out, 4, STATE_SCHEMA_VERSION);
        put_u64(out, 8, goal_id);
        out[16..32].copy_from_slice(&owner);
        out[32] = origin_code(plan.goal.origin);
        if let RequestOrigin::Application(id)=plan.goal.origin{out[76..80].copy_from_slice(&(id as u32).to_le_bytes());}
        out[33] = plan.state as u8;
        out[34] = plan.task_count;
        out[35] = plan.replans;
        put_u16(out, 36, plan.revision);
        put_u64(out, 40, plan.goal.created_at);
        put_u64(out, 48, plan.limits.deadline);
        put_u16(out, 56, plan.goal.objective.as_bytes().len() as u16);
        out[58] = plan.limits.maximum_tasks;
        out[59] = plan.limits.maximum_parallel;
        out[60] = plan.limits.maximum_replans;
        put_u16(out, 62, plan.goal.desired_outcome.as_bytes().len() as u16);
        put_u16(out, 64, plan.goal.success_criteria.as_bytes().len() as u16);
        put_u16(out, 66, plan.goal.constraints.as_bytes().len() as u16);
        put_u16(out, 68, plan.goal.context.as_bytes().len() as u16);
        put_u16(
            out,
            70,
            plan.goal.missing_information.as_bytes().len() as u16,
        );
        out[72..76].copy_from_slice(&plan.goal.required_permissions.to_le_bytes());
        out[80..80 + plan.goal.objective.as_bytes().len()]
            .copy_from_slice(plan.goal.objective.as_bytes());
        out[592..592 + plan.goal.desired_outcome.as_bytes().len()]
            .copy_from_slice(plan.goal.desired_outcome.as_bytes());
        out[784..784 + plan.goal.success_criteria.as_bytes().len()]
            .copy_from_slice(plan.goal.success_criteria.as_bytes());
        out[976..976 + plan.goal.constraints.as_bytes().len()]
            .copy_from_slice(plan.goal.constraints.as_bytes());
        out[1168..1168 + plan.goal.context.as_bytes().len()]
            .copy_from_slice(plan.goal.context.as_bytes());
        out[1360..1360 + plan.goal.missing_information.as_bytes().len()]
            .copy_from_slice(plan.goal.missing_information.as_bytes());
        let mut at = GOAL_BYTES;
        for task in plan.tasks.iter().flatten() {
            put_u16(out, at, task.id);
            out[at + 2] = task.state as u8;
            out[at + 3] = task.attempts;
            out[at + 4] = failure_code(task.failure_policy);
            out[at + 5] = task.dependency_count;
            encode_capability(task.capability, &mut out[at + 6..at + 10]);
            for i in 0..MAX_DEPENDENCIES {
                put_u16(out, at + 10 + i * 2, task.dependencies[i]);
            }
            out[at + 18..at + 34].copy_from_slice(&task.evidence);
            out[at + 34] = contract_code(task.expected_output);
            put_u16(out, at + 36, task.description.as_bytes().len() as u16);
            put_u16(out, at + 38, task.inputs.as_bytes().len() as u16);
            put_u16(out, at + 40, task.verification.as_bytes().len() as u16);
            out[at + 44..at + 44 + task.description.as_bytes().len()]
                .copy_from_slice(task.description.as_bytes());
            out[at + 236..at + 236 + task.inputs.as_bytes().len()]
                .copy_from_slice(task.inputs.as_bytes());
            out[at + 428..at + 428 + task.verification.as_bytes().len()]
                .copy_from_slice(task.verification.as_bytes());
            at += TASK_BYTES;
        }
        Ok(at)
    }

    // ------------------------=
    // FUNC: restore
    // DESC: Restores a checkpoint for the same owner while converting in-flight work to resumable pending state.
    // ------------------=
    pub fn restore<R: CapabilityRegistry>(
        &mut self,
        input: &[u8],
        owner: [u8; 16],
        registry: &R,
    ) -> Result<u64, ServiceError> {
        const GOAL_BYTES: usize = 1552;
        const TASK_BYTES: usize = 640;
        if input.len() < GOAL_BYTES
            || &input[..4] != b"IKIT"
            || read_u16(input, 4) != STATE_SCHEMA_VERSION
            || input[16..32] != owner
        {
            return Err(ServiceError::Persistence);
        }
        let count = input[34] as usize;
        let lengths = [
            read_u16(input, 56) as usize,
            read_u16(input, 62) as usize,
            read_u16(input, 64) as usize,
            read_u16(input, 66) as usize,
            read_u16(input, 68) as usize,
            read_u16(input, 70) as usize,
        ];
        if count == 0
            || count > MAX_TASKS
            || lengths[0] > MAX_OBJECTIVE_BYTES
            || lengths[1..].iter().any(|length| *length > MAX_TEXT_BYTES)
            || GOAL_BYTES + count * TASK_BYTES > input.len()
        {
            return Err(ServiceError::Persistence);
        }
        let goal_id = read_u64(input, 8);
        let goal = Goal {
            id: goal_id,
            owner,
            origin: decode_origin(input[32],u32::from_le_bytes(input[76..80].try_into().unwrap())as usize)?,
            objective: BoundedText::new(&input[80..80 + lengths[0]])
                .ok_or(ServiceError::Persistence)?,
            desired_outcome: BoundedText::new(&input[592..592 + lengths[1]])
                .ok_or(ServiceError::Persistence)?,
            success_criteria: BoundedText::new(&input[784..784 + lengths[2]])
                .ok_or(ServiceError::Persistence)?,
            constraints: BoundedText::new(&input[976..976 + lengths[3]])
                .ok_or(ServiceError::Persistence)?,
            context: BoundedText::new(&input[1168..1168 + lengths[4]])
                .ok_or(ServiceError::Persistence)?,
            missing_information: BoundedText::new(&input[1360..1360 + lengths[5]])
                .ok_or(ServiceError::Persistence)?,
            required_permissions: u32::from_le_bytes(input[72..76].try_into().unwrap()),
            created_at: read_u64(input, 40),
        };
        let mut plan = Plan {
            goal,
            tasks: [None; MAX_TASKS],
            task_count: count as u8,
            state: PlanState::Interrupted,
            limits: ExecutionLimits {
                maximum_tasks: input[58],
                maximum_parallel: input[59],
                maximum_replans: input[60],
                deadline: read_u64(input, 48),
            },
            replans: input[35],
            revision: read_u16(input, 36),
        };
        let mut at = GOAL_BYTES;
        for slot in 0..count {
            let capability = decode_capability(&input[at + 6..at + 10])?;
            if registry.describe(capability).is_none() {
                return Err(ServiceError::InvalidPlan);
            }
            let mut deps = [0; MAX_DEPENDENCIES];
            for i in 0..MAX_DEPENDENCIES {
                deps[i] = read_u16(input, at + 10 + i * 2);
            }
            let stored = decode_task_state(input[at + 2])?;
            let state = if stored == TaskState::Succeeded {
                stored
            } else {
                TaskState::Pending
            };
            let mut evidence = [0; 16];
            evidence.copy_from_slice(&input[at + 18..at + 34]);
            let dl = read_u16(input, at + 36) as usize;
            let il = read_u16(input, at + 38) as usize;
            let vl = read_u16(input, at + 40) as usize;
            if dl > MAX_TEXT_BYTES || il > MAX_TEXT_BYTES || vl > MAX_TEXT_BYTES {
                return Err(ServiceError::Persistence);
            }
            plan.tasks[slot] = Some(Task {
                id: read_u16(input, at),
                description: BoundedText::new(&input[at + 44..at + 44 + dl])
                    .ok_or(ServiceError::Persistence)?,
                capability,
                dependencies: deps,
                dependency_count: input[at + 5],
                inputs: BoundedText::new(&input[at + 236..at + 236 + il])
                    .ok_or(ServiceError::Persistence)?,
                expected_output: decode_contract(input[at + 34])?,
                verification: BoundedText::new(&input[at + 428..at + 428 + vl])
                    .ok_or(ServiceError::Persistence)?,
                state,
                failure_policy: decode_failure(input[at + 4])?,
                attempts: input[at + 3],
                execution_token: 0,
                evidence,
            });
            at += TASK_BYTES;
        }
        validate_plan(&plan, registry)?;
        let slot = self
            .plans
            .iter()
            .position(Option::is_none)
            .ok_or(ServiceError::Full)?;
        self.plans[slot] = Some(plan);
        self.next_goal_id = self.next_goal_id.max(goal_id.saturating_add(1));
        self.emit(goal_id, None, EventKind::Interrupted);
        Ok(goal_id)
    }

    // ------------------------=
    // FUNC: plan_index
    // DESC: Resolves one exact owner-bound plan slot.
    // ------------------=
    fn plan_index(&self, goal_id: u64, owner: [u8; 16]) -> Result<usize, ServiceError> {
        self.plans
            .iter()
            .position(|entry| {
                entry.is_some_and(|plan| plan.goal.id == goal_id && plan.goal.owner == owner)
            })
            .ok_or(ServiceError::NotFound)
    }

    // ------------------------=
    // FUNC: fail_running
    // DESC: Cancels active execution tokens without changing completed task evidence.
    // ------------------=
    fn fail_running<E: Executor>(&mut self, index: usize, executor: &mut E) {
        for task in self.plans[index]
            .as_mut()
            .unwrap()
            .tasks
            .iter_mut()
            .flatten()
        {
            if task.state == TaskState::Running {
                executor.cancel(task.execution_token);
                task.state = TaskState::Failed;
            }
        }
    }

    // ------------------------=
    // FUNC: emit
    // DESC: Appends one bounded structured progress event.
    // ------------------=
    fn emit(&mut self, goal_id: u64, task_id: Option<u16>, kind: EventKind) {
        self.events[self.event_cursor] = Some(ProgressEvent {
            sequence: self.next_event_sequence,
            goal_id,
            task_id,
            kind,
        });
        self.next_event_sequence = self.next_event_sequence.saturating_add(1).max(1);
        self.event_cursor = (self.event_cursor + 1) % MAX_EVENTS;
        self.event_count = self.event_count.saturating_add(1).min(MAX_EVENTS);
    }
}

// ------------------------=
// FUNC: push_task
// DESC: Adds one typed task to a bounded plan while preserving stable one-based task identifiers.
// ------------------=
fn push_task(
    plan: &mut Plan,
    capability: CapabilityId,
    description: &[u8],
    inputs: &[u8],
    expected_output: Contract,
    verification: &[u8],
    dependencies: &[u16],
    failure_policy: FailurePolicy,
) -> Result<(), ServiceError> {
    if plan.task_count as usize >= plan.limits.maximum_tasks as usize
        || dependencies.len() > MAX_DEPENDENCIES
    {
        return Err(ServiceError::InvalidPlan);
    }
    let mut deps = [0; MAX_DEPENDENCIES];
    deps[..dependencies.len()].copy_from_slice(dependencies);
    let id = plan.task_count as u16 + 1;
    plan.tasks[plan.task_count as usize] = Some(Task {
        id,
        description: BoundedText::new(description).ok_or(ServiceError::InvalidPlan)?,
        capability,
        dependencies: deps,
        dependency_count: dependencies.len() as u8,
        inputs: BoundedText::new(inputs).ok_or(ServiceError::InvalidPlan)?,
        expected_output,
        verification: BoundedText::new(verification).ok_or(ServiceError::InvalidPlan)?,
        state: TaskState::Pending,
        failure_policy,
        attempts: 0,
        execution_token: 0,
        evidence: [0; 16],
    });
    plan.task_count += 1;
    Ok(())
}

// ------------------------=
// FUNC: validate_plan
// DESC: Rejects missing capabilities, forward dependencies, cycles, and execution-limit violations before execution.
// ------------------=
fn validate_plan<R: CapabilityRegistry>(plan: &Plan, registry: &R) -> Result<(), ServiceError> {
    if plan.task_count == 0 || plan.task_count > plan.limits.maximum_tasks {
        return Err(ServiceError::InvalidPlan);
    }
    for task in plan.tasks.iter().flatten() {
        if registry.describe(task.capability).is_none()
            || task.dependency_count as usize > MAX_DEPENDENCIES
        {
            return Err(ServiceError::InvalidPlan);
        }
        for dependency in &task.dependencies[..task.dependency_count as usize] {
            if *dependency == 0 || *dependency >= task.id {
                return Err(ServiceError::InvalidPlan);
            }
        }
    }
    Ok(())
}

// ------------------------=
// FUNC: refresh_ready
// DESC: Marks tasks ready only after every dependency has verified success and blocks descendants of failures.
// ------------------=
fn refresh_ready(plan: &mut Plan) {
    let snapshot = plan.tasks;
    for task in plan.tasks.iter_mut().flatten() {
        if task.state != TaskState::Pending {
            continue;
        }
        let deps = &task.dependencies[..task.dependency_count as usize];
        if deps.iter().any(|id| {
            task_by_id(&snapshot, *id).is_some_and(|value| {
                matches!(
                    value.state,
                    TaskState::Failed | TaskState::Cancelled | TaskState::Blocked
                )
            })
        }) {
            task.state = TaskState::Blocked;
        } else if deps.iter().all(|id| {
            task_by_id(&snapshot, *id).is_some_and(|value| value.state == TaskState::Succeeded)
        }) {
            task.state = TaskState::Ready;
        }
    }
}

// ------------------------=
// FUNC: task_by_id
// DESC: Finds one immutable task by stable plan-local identity.
// ------------------=
fn task_by_id(tasks: &[Option<Task>; MAX_TASKS], id: u16) -> Option<&Task> {
    tasks.iter().flatten().find(|task| task.id == id)
}

// ------------------------=
// FUNC: task_mut
// DESC: Finds one mutable task by stable plan-local identity.
// ------------------=
fn task_mut(plan: &mut Plan, id: u16) -> Option<&mut Task> {
    plan.tasks.iter_mut().flatten().find(|task| task.id == id)
}

// ------------------------=
// FUNC: summary
// DESC: Projects factual progress from task states rather than generated language.
// ------------------=
fn summary(plan: &Plan) -> PlanSummary {
    PlanSummary {
        goal_id: plan.goal.id,
        state: plan.state,
        completed: plan
            .tasks
            .iter()
            .flatten()
            .filter(|task| task.state == TaskState::Succeeded)
            .count() as u8,
        total: plan.task_count,
        failed_task: plan
            .tasks
            .iter()
            .flatten()
            .find(|task| task.state == TaskState::Failed)
            .map(|task| task.id),
    }
}

// ------------------------=
// FUNC: clauses
// DESC: Splits a bounded objective into ordered action clauses without allocation.
// ------------------=
fn clauses(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    input
        .split(|byte| matches!(byte, b',' | b';' | b'\n'))
        .flat_map(|part| part.split(|byte| *byte == b'.'))
        .flat_map(|part| split_once_case(part, b" and then "))
        .map(|part| part.trim_ascii())
        .filter(|part| !part.is_empty())
}

// ------------------------=
// FUNC: split_once_case
// DESC: Splits one clause on a case-insensitive delimiter while retaining borrowed slices.
// ------------------=
fn split_once_case<'a>(input: &'a [u8], needle: &[u8]) -> core::array::IntoIter<&'a [u8], 2> {
    let index = find_case(input, needle);
    [
        &input[..index.unwrap_or(input.len())],
        index.map(|at| &input[at + needle.len()..]).unwrap_or(&[]),
    ]
    .into_iter()
}

// ------------------------=
// FUNC: starts_case
// DESC: Compares an ASCII prefix without allocating normalized text.
// ------------------=
fn starts_case(input: &[u8], prefix: &[u8]) -> bool {
    input
        .get(..prefix.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
}

// ------------------------=
// FUNC: contains_case
// DESC: Searches bounded ASCII text case-insensitively.
// ------------------=
fn contains_case(input: &[u8], needle: &[u8]) -> bool {
    find_case(input, needle).is_some()
}

// ------------------------=
// FUNC: find_case
// DESC: Returns the first case-insensitive ASCII match offset.
// ------------------=
fn find_case(input: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    input
        .windows(needle.len())
        .position(|value| value.eq_ignore_ascii_case(needle))
}

// ------------------------=
// FUNC: origin_code
// DESC: Encodes a stable request-origin discriminator.
// ------------------=
fn origin_code(value: RequestOrigin) -> u8 {
    match value {
        RequestOrigin::Voice => 1,
        RequestOrigin::Text => 2,
        RequestOrigin::Application(_) => 3,
        RequestOrigin::NativeService => 4,
    }
}

// ------------------------=
// FUNC: decode_origin
// DESC: Decodes a stable request-origin discriminator.
// ------------------=
fn decode_origin(value: u8, application:usize) -> Result<RequestOrigin, ServiceError> {
    match value {
        1 => Ok(RequestOrigin::Voice),
        2 => Ok(RequestOrigin::Text),
        3 => Ok(RequestOrigin::Application(application)),
        4 => Ok(RequestOrigin::NativeService),
        _ => Err(ServiceError::Persistence),
    }
}

// ------------------------=
// FUNC: failure_code
// DESC: Encodes a stable recovery-policy discriminator.
// ------------------=
fn failure_code(value: FailurePolicy) -> u8 {
    match value {
        FailurePolicy::Stop => 1,
        FailurePolicy::RetryOnce => 2,
        FailurePolicy::ReplanAffected => 3,
    }
}

// ------------------------=
// FUNC: decode_failure
// DESC: Decodes a stable recovery-policy discriminator.
// ------------------=
fn decode_failure(value: u8) -> Result<FailurePolicy, ServiceError> {
    match value {
        1 => Ok(FailurePolicy::Stop),
        2 => Ok(FailurePolicy::RetryOnce),
        3 => Ok(FailurePolicy::ReplanAffected),
        _ => Err(ServiceError::Persistence),
    }
}

// ------------------------=
// FUNC: encode_capability
// DESC: Encodes one stable capability identifier without Rust layout dependence.
// ------------------=
fn encode_capability(value: CapabilityId, out: &mut [u8]) {
    let (kind, id) = match value {
        CapabilityId::AiAction(id) => (1, id),
        CapabilityId::DevKit => (2, 0),
        CapabilityId::ObjectStorage => (3, 0),
        CapabilityId::UiServices => (4, 0),
        CapabilityId::InfinityPool => (5, 0),
        CapabilityId::ApplicationInteraction => (6, 0),
    };
    out[0] = kind;
    put_u16(out, 1, id);
}

// ------------------------=
// FUNC: decode_capability
// DESC: Decodes one stable capability identifier.
// ------------------=
fn decode_capability(input: &[u8]) -> Result<CapabilityId, ServiceError> {
    match input.first().copied() {
        Some(1) => Ok(CapabilityId::AiAction(read_u16(input, 1))),
        Some(2) => Ok(CapabilityId::DevKit),
        Some(3) => Ok(CapabilityId::ObjectStorage),
        Some(4) => Ok(CapabilityId::UiServices),
        Some(5) => Ok(CapabilityId::InfinityPool),
        Some(6) => Ok(CapabilityId::ApplicationInteraction),
        _ => Err(ServiceError::Persistence),
    }
}

// ------------------------=
// FUNC: decode_task_state
// DESC: Decodes a stable task-state discriminator.
// ------------------=
fn decode_task_state(value: u8) -> Result<TaskState, ServiceError> {
    match value {
        0 => Ok(TaskState::Pending),
        1 => Ok(TaskState::WaitingApproval),
        2 => Ok(TaskState::Ready),
        3 => Ok(TaskState::Running),
        4 => Ok(TaskState::Succeeded),
        5 => Ok(TaskState::Failed),
        6 => Ok(TaskState::Cancelled),
        7 => Ok(TaskState::Blocked),
        _ => Err(ServiceError::Persistence),
    }
}

// ------------------------=
// FUNC: contract_code
// DESC: Encodes a stable typed input or output contract discriminator.
// ------------------=
fn contract_code(value: Contract) -> u8 {
    match value {
        Contract::None => 0,
        Contract::Text => 1,
        Contract::ObjectReference => 2,
        Contract::ApplicationArtifact => 3,
        Contract::ExecutionReceipt => 4,
        Contract::ComputeResult => 5,
    }
}

// ------------------------=
// FUNC: decode_contract
// DESC: Decodes a stable typed input or output contract discriminator.
// ------------------=
fn decode_contract(value: u8) -> Result<Contract, ServiceError> {
    match value {
        0 => Ok(Contract::None),
        1 => Ok(Contract::Text),
        2 => Ok(Contract::ObjectReference),
        3 => Ok(Contract::ApplicationArtifact),
        4 => Ok(Contract::ExecutionReceipt),
        5 => Ok(Contract::ComputeResult),
        _ => Err(ServiceError::Persistence),
    }
}

// ------------------------=
// FUNC: put_u16
// DESC: Writes one portable little-endian u16.
// ------------------=
fn put_u16(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: read_u16
// DESC: Reads one portable little-endian u16.
// ------------------=
fn read_u16(input: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(input[at..at + 2].try_into().unwrap())
}

// ------------------------=
// FUNC: put_u64
// DESC: Writes one portable little-endian u64.
// ------------------=
fn put_u64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

// ------------------------=
// FUNC: read_u64
// DESC: Reads one portable little-endian u64.
// ------------------=
fn read_u64(input: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(input[at..at + 8].try_into().unwrap())
}
