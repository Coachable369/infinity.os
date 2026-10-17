#![allow(dead_code)]

#[path = "../kernel/runtime/mod.rs"]
mod runtime;
#[path = "storage.rs"]
mod storage;
#[path = "../kernel/ui/mod.rs"]
mod ui;

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey {
        Character(u8),
        Backspace,
        Delete,
        Left,
        Right,
        Home,
        End,
        Enter,
    }
}

// ------------------------=
// FUNC: output_text
// DESC: Supplies the diagnostic sink required by the host runtime harness.
// ------------------=
fn output_text(_: &[u8]) {}

use runtime::ai::intelligence::{
    Availability, CapabilityDescriptor, CapabilityId, CapabilityRegistry, Contract,
    ExecutionLimits, Executor, InstalledCapabilities, Observation, PlanState, RequestClass,
    RequestOrigin, Risk, Service, StartError, Task, TaskState,
};

struct HarnessExecutor {
    next: u64,
    starts: u8,
    fail_first: bool,
    deny: bool,
    pending: [Option<(u64, u8)>; 16],
    cancelled: u8,
}

impl HarnessExecutor {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a deterministic native-executor substitute for behavioral state verification.
    // ------------------=
    fn new() -> Self {
        Self {
            next: 1,
            starts: 0,
            fail_first: false,
            deny: false,
            pending: [None; 16],
            cancelled: 0,
        }
    }
}

impl Executor for HarnessExecutor {
    // ------------------------=
    // FUNC: authorize
    // DESC: Models the final native capability check independently from proposal generation.
    // ------------------=
    fn authorize(&self, _: [u8; 16], _: CapabilityDescriptor, _: u64) -> bool {
        !self.deny
    }

    // ------------------------=
    // FUNC: start
    // DESC: Starts one deterministic asynchronous fixture execution.
    // ------------------=
    fn start(
        &mut self,
        _: &runtime::ai::intelligence::Goal,
        _: &Task,
        _: CapabilityDescriptor,
        _: u64,
    ) -> Result<u64, StartError> {
        let token = self.next;
        self.next += 1;
        self.starts += 1;
        let slot = self
            .pending
            .iter()
            .position(Option::is_none)
            .ok_or(StartError::Busy)?;
        self.pending[slot] = Some((token, 0));
        Ok(token)
    }

    // ------------------------=
    // FUNC: observe
    // DESC: Produces running then terminal structured evidence without trusting generated prose.
    // ------------------=
    fn observe(&mut self, token: u64, _: u64) -> Observation {
        let entry = self
            .pending
            .iter_mut()
            .flatten()
            .find(|entry| entry.0 == token)
            .unwrap();
        entry.1 += 1;
        if entry.1 == 1 {
            return Observation::Running;
        }
        if self.fail_first && self.starts == 1 {
            return Observation::Failed(7);
        }
        let mut evidence = [0; 16];
        evidence[..8].copy_from_slice(&token.to_le_bytes());
        Observation::Succeeded(evidence)
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Records cancellation of an exact execution lease.
    // ------------------=
    fn cancel(&mut self, token: u64) {
        if let Some(slot) = self
            .pending
            .iter_mut()
            .find(|entry| entry.is_some_and(|entry| entry.0 == token))
        {
            *slot = None;
            self.cancelled += 1;
        }
    }
}

struct SensitiveRegistry;

impl CapabilityRegistry for SensitiveRegistry {
    // ------------------------=
    // FUNC: describe
    // DESC: Exposes a destructive fixture capability for approval testing.
    // ------------------=
    fn describe(&self, id: CapabilityId) -> Option<CapabilityDescriptor> {
        Some(CapabilityDescriptor {
            id,
            input: Contract::Text,
            output: Contract::ExecutionReceipt,
            required_permission: 99,
            risk: Risk::Destructive,
            reversible: false,
            dependency: None,
            availability: Availability::Ready,
        })
    }

    // ------------------------=
    // FUNC: resolve_action
    // DESC: Resolves every nonempty fixture clause to the destructive test action.
    // ------------------=
    fn resolve_action(&self, words: &[u8]) -> Option<CapabilityId> {
        (!words.is_empty()).then_some(CapabilityId::AiAction(0))
    }
}

// ------------------------=
// FUNC: limits
// DESC: Returns bounded deterministic execution limits shared by behavior scenarios.
// ------------------=
fn limits() -> ExecutionLimits {
    ExecutionLimits {
        maximum_tasks: 8,
        maximum_parallel: 2,
        maximum_replans: 1,
        deadline: 10_000,
    }
}

// ------------------------=
// FUNC: finish
// DESC: Polls a plan through asynchronous execution until it reaches a terminal state.
// ------------------=
fn finish<R: CapabilityRegistry>(
    service: &mut Service,
    id: u64,
    owner: [u8; 16],
    registry: &R,
    executor: &mut HarnessExecutor,
) -> PlanState {
    for now in 1..64 {
        let state = service
            .poll(id, owner, registry, executor, now)
            .unwrap()
            .state;
        if matches!(state, PlanState::Completed | PlanState::Failed) {
            return state;
        }
    }
    panic!("plan did not terminate")
}

// ------------------------=
// FUNC: main
// DESC: Verifies IntelligenceKit routing, discovery, DAG execution, recovery, persistence, ingress parity, and authorization.
// ------------------=
fn main() {
    let owner = [7; 16];
    let registry = InstalledCapabilities;
    assert_eq!(
        Service::classify(b"open browser", true),
        RequestClass::SimpleAction
    );
    assert_eq!(
        Service::classify(b"hello there", false),
        RequestClass::Conversation
    );
    assert_eq!(
        Service::classify(b"create a chess game and play me", false),
        RequestClass::ComplexGoal
    );
    assert_eq!(
        Service::classify(b"build a graphical tic-tac-toe game", false),
        RequestClass::ComplexGoal
    );

    let mut interrupted = runtime::ai::AiRuntime::new();
    interrupted.bind_chat_owner(owner);
    for byte in b"open browser and then open text editor" {
        assert!(interrupted.chat.push_input(*byte));
    }
    assert!(interrupted.submit_voice_chat());
    for byte in b"open settings" {
        assert!(interrupted.chat.push_input(*byte));
    }
    assert!(interrupted.submit_voice_chat());
    assert_eq!(
        interrupted.take_control(owner, true),
        Some(runtime::ai::control::Command::Settings)
    );

    for voice_request in [false, true] {
        let mut ai = runtime::ai::AiRuntime::new();
        ai.bind_chat_owner(owner);
        for byte in b"open browser and then open text editor" {
            assert!(ai.chat.push_input(*byte));
        }
        assert!(if voice_request {
            ai.submit_voice_chat()
        } else {
            ai.submit_chat()
        });
        assert!(ai.intelligence_plan(1, owner).is_some());
        assert!(ai.poll_qwen());
        assert_eq!(ai.take_control(owner,true),Some(runtime::ai::control::Command::Browser));
        ai.finish_control(runtime::ai::control::Command::Browser,true);
        assert!(ai.poll_qwen());
        assert_eq!(ai.take_control(owner,true),Some(runtime::ai::control::Command::TextEditor));
        ai.finish_control(runtime::ai::control::Command::TextEditor,true);
        assert!(ai.poll_qwen());
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Completed);
    }

    let mut service = Service::new();
    let graph = service
        .propose(
            owner,
            RequestOrigin::Text,
            b"open browser and then open text editor",
            &registry,
            0,
            limits(),
        )
        .unwrap();
    let plan = service.inspect(graph, owner).unwrap();
    assert_eq!(plan.task_count, 2);
    assert!(plan.tasks[1].unwrap().depends_on(1));
    assert!(matches!(
        plan.tasks[0].unwrap().capability,
        CapabilityId::AiAction(_)
    ));

    let generated = service
        .propose(
            owner,
            RequestOrigin::Voice,
            b"build a simple native calculator app",
            &registry,
            0,
            limits(),
        )
        .unwrap();
    let generated_plan = service.inspect(generated, owner).unwrap();
    assert_eq!(
        generated_plan.tasks[0].unwrap().capability,
        CapabilityId::DevKit
    );
    assert_eq!(
        generated_plan.tasks[1].unwrap().capability,
        CapabilityId::UiServices
    );

    service.begin(graph, owner, &registry).unwrap();
    let mut recovery = HarnessExecutor::new();
    recovery.fail_first = true;
    assert_eq!(
        finish(&mut service, graph, owner, &registry, &mut recovery),
        PlanState::Completed
    );
    assert!(recovery.starts >= 3);
    assert!(service.inspect(graph, owner).unwrap().replans <= 1);

    service.begin(generated, owner, &registry).unwrap();
    let mut interrupted = HarnessExecutor::new();
    service
        .poll(generated, owner, &registry, &mut interrupted, 1)
        .unwrap();
    service
        .poll(generated, owner, &registry, &mut interrupted, 2)
        .unwrap();
    service
        .poll(generated, owner, &registry, &mut interrupted, 3)
        .unwrap();
    service
        .interrupt(generated, owner, &mut interrupted)
        .unwrap();
    assert!(interrupted.cancelled > 0);
    let mut checkpoint = [0; runtime::ai::intelligence::MAX_STATE_BYTES];
    let length = service.encode(generated, owner, &mut checkpoint).unwrap();
    let completed_before = service
        .inspect(generated, owner)
        .unwrap()
        .tasks
        .iter()
        .flatten()
        .filter(|task| task.state == TaskState::Succeeded)
        .count();
    let mut resumed = Service::new();
    resumed
        .restore(&checkpoint[..length], owner, &registry)
        .unwrap();
    let restored = resumed.inspect(generated, owner).unwrap();
    assert_eq!(
        restored
            .tasks
            .iter()
            .flatten()
            .filter(|task| task.state == TaskState::Succeeded)
            .count(),
        completed_before
    );
    assert!(restored
        .tasks
        .iter()
        .flatten()
        .filter(|task| task.state != TaskState::Succeeded)
        .all(|task| task.state == TaskState::Pending));

    let mut ingress = Service::new();
    let voice = ingress
        .propose(
            owner,
            RequestOrigin::Voice,
            b"build a simple native notes app",
            &registry,
            0,
            limits(),
        )
        .unwrap();
    let text = ingress
        .propose(
            owner,
            RequestOrigin::Text,
            b"build a simple native notes app",
            &registry,
            0,
            limits(),
        )
        .unwrap();
    assert_eq!(
        ingress.inspect(voice, owner).unwrap().task_count,
        ingress.inspect(text, owner).unwrap().task_count
    );

    let mut approval = Service::new();
    let sensitive = SensitiveRegistry;
    let sensitive_id = approval
        .propose(
            owner,
            RequestOrigin::Text,
            b"my goal is remove records",
            &sensitive,
            0,
            limits(),
        )
        .unwrap();
    approval.begin(sensitive_id, owner, &sensitive).unwrap();
    assert_eq!(
        approval.inspect(sensitive_id, owner).unwrap().state,
        PlanState::AwaitingApproval
    );
    let mut executor = HarnessExecutor::new();
    approval
        .poll(sensitive_id, owner, &sensitive, &mut executor, 1)
        .unwrap();
    assert_eq!(executor.starts, 0);
    approval.approve(sensitive_id, 1, owner).unwrap();
    executor.deny = true;
    assert_eq!(
        finish(
            &mut approval,
            sensitive_id,
            owner,
            &sensitive,
            &mut executor
        ),
        PlanState::Failed
    );
    assert_eq!(executor.starts, 0);

    println!("IntelligenceKit goal planning and execution behavior: PASS");
}
