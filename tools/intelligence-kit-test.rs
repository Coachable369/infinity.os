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
// FUNC: software_receipts
// DESC: Verifies native software tasks cannot succeed from dispatch, generated text, stale identities, or unordered callbacks.
// ------------------=
fn software_receipts() {
    use runtime::ai::intelligence::software::{Event, Execution};
    let owner=[7;16];
    let mut execution=Execution::new(5,owner);
    assert!(execution.begin(101,CapabilityId::DevKit));
    assert_eq!(execution.observe(101),Observation::Running);
    assert!(!execution.record(5,owner,Event::Compiled{artifact:[9;16]}));
    assert!(!execution.record(6,owner,Event::SourceSaved{object:[3;16],revision:4}));
    assert!(!execution.record(5,[8;16],Event::SourceSaved{object:[3;16],revision:4}));
    assert!(!execution.record(5,owner,Event::SourceSaved{object:[0;16],revision:4}));
    assert!(execution.record(5,owner,Event::SourceSaved{object:[3;16],revision:4}));
    assert_eq!(execution.observe(101),Observation::Running);
    assert!(!execution.record(5,owner,Event::Launched{context:2}));
    assert!(execution.record(5,owner,Event::Compiled{artifact:[9;16]}));
    assert_eq!(execution.observe(101),Observation::Succeeded([9;16]));
    assert!(execution.begin(102,CapabilityId::UiServices));
    assert_eq!(execution.observe(102),Observation::Running);
    assert!(execution.record(5,owner,Event::Launched{context:2}));
    assert!(matches!(execution.observe(102),Observation::Succeeded(_)));
    assert!(execution.begin(103,CapabilityId::ApplicationInteraction));
    assert_eq!(execution.observe(103),Observation::Running);
    assert!(!execution.record(5,owner,Event::InteractionReady{context:9,revision:1}));
    assert!(!execution.record(5,owner,Event::InteractionReady{context:2,revision:0}));
    assert!(execution.record(5,owner,Event::InteractionReady{context:2,revision:1}));
    assert!(matches!(execution.observe(103),Observation::Succeeded(_)));
    assert!(execution.interaction_active);
    assert_eq!(execution.applied_actions,0);
    assert!(execution.record(5,owner,Event::InteractionApplied{context:2,revision:2,token:11}));
    assert!(!execution.record(5,owner,Event::InteractionApplied{context:2,revision:2,token:11}));
    assert_eq!(execution.applied_actions,1);
    assert!(execution.record(5,owner,Event::InteractionStopped{context:2}));
    assert!(!execution.interaction_active);
    assert_eq!(execution.applied_actions,1);
    let mut cancelled=Execution::new(6,owner);
    assert!(cancelled.begin(9,CapabilityId::DevKit));
    cancelled.cancel(9);
    assert!(!cancelled.record(6,owner,Event::SourceSaved{object:[3;16],revision:4}));
}

// ------------------------=
// FUNC: software_runtime_failure_receipts
// DESC: Accepts post-launch failure only from the exact saved-source/context lease and never mistakes published controls for functional proof.
// ------------------=
fn software_runtime_failure_receipts() {
    use runtime::ai::intelligence::software::{Event, Execution, RuntimeFailure};
    let owner = [7; 16];
    let mut execution = Execution::new(5, owner);
    assert!(execution.record(5, owner, Event::SourceSaved { object: [3; 16], revision: 4 }));
    assert!(execution.record(5, owner, Event::Compiled { artifact: [9; 16] }));
    let failure = Event::RuntimeFailed { source: [3; 16], revision: 4, context: 2, code: 3 };
    let before_launch = execution;
    assert!(!execution.record(5, owner, failure));
    assert_eq!(execution, before_launch);
    assert!(execution.record(5, owner, Event::Launched { context: 2 }));
    assert!(execution.record(5, owner, Event::InteractionReady { context: 2, revision: 1 }));
    assert!(execution.begin(103, CapabilityId::ApplicationInteraction));
    assert!(matches!(execution.observe(103), Observation::Succeeded(_)));
    assert_eq!(execution.applied_actions, 0);
    assert_eq!(execution.runtime_failure(), None);
    let launched = execution;
    for (goal, claimant, event) in [
        (6, owner, failure),
        (5, [8; 16], failure),
        (5, owner, Event::RuntimeFailed { source: [4; 16], revision: 4, context: 2, code: 3 }),
        (5, owner, Event::RuntimeFailed { source: [3; 16], revision: 5, context: 2, code: 3 }),
        (5, owner, Event::RuntimeFailed { source: [3; 16], revision: 4, context: 1, code: 3 }),
        (5, owner, Event::RuntimeFailed { source: [3; 16], revision: 4, context: 0, code: 3 }),
    ] {
        assert!(!execution.record(goal, claimant, event));
        assert_eq!(execution, launched);
    }
    assert!(execution.record(5, owner, failure));
    assert_eq!(execution.runtime_failure(), Some(RuntimeFailure {
        source: [3; 16], revision: 4, context: 2, code: 3,
    }));
    assert!(!execution.interaction_active);
    assert_eq!(execution.source, launched.source);
    assert_eq!(execution.artifact, launched.artifact);
    assert_eq!(execution.context, launched.context);
    assert!(!execution.record(5, owner, Event::InteractionApplied { context: 2, revision: 2, token: 12 }));
    assert!(execution.begin(104, CapabilityId::UiServices));
    assert_eq!(execution.observe(104), Observation::Failed(3));
    assert!(execution.begin(105, CapabilityId::DevKit));
    assert_eq!(execution.observe(105), Observation::Failed(3));
    let mut normalized = launched;
    assert!(normalized.record(5, owner, Event::RuntimeFailed {
        source: [3; 16], revision: 4, context: 2, code: 0,
    }));
    assert_eq!(normalized.runtime_failure().unwrap().code, 1);
}

// ------------------------=
// FUNC: software_runtime_failure_plan
// DESC: Invalidates launch-dependent success without replaying a goal, losing true compilation history, or resetting its authorization and retry limits.
// ------------------=
fn software_runtime_failure_plan() {
    use runtime::ai::intelligence::ServiceError;
    let owner = [7; 16];
    let registry = InstalledCapabilities;
    for objective in [
        b"create a graphical calendar app".as_slice(),
        b"create a board game and play me".as_slice(),
    ] {
        let mut service = Service::new();
        let id = service.propose(owner, RequestOrigin::Text, objective, &registry, 0, limits()).unwrap();
        service.begin(id, owner, &registry).unwrap();
        let mut executor = HarnessExecutor::new();
        assert_eq!(finish(&mut service, id, owner, &registry, &mut executor), PlanState::Completed);
        let completed = *service.inspect(id, owner).unwrap();
        assert_eq!(service.mark_runtime_failure(id, [8; 16]), Err(ServiceError::NotFound));
        assert_eq!(*service.inspect(id, owner).unwrap(), completed);
        service.mark_runtime_failure(id, owner).unwrap();
        let failed = *service.inspect(id, owner).unwrap();
        assert_eq!(failed.state, PlanState::Failed);
        assert_eq!(failed.goal, completed.goal);
        assert_eq!(failed.limits, completed.limits);
        assert_eq!(failed.replans, completed.replans);
        assert_eq!(failed.revision, completed.revision + 1);
        assert_eq!(failed.tasks[0], completed.tasks[0]);
        for index in 1..failed.task_count as usize {
            let task = failed.tasks[index].unwrap();
            assert_eq!(task.state, if index == 1 { TaskState::Failed } else { TaskState::Blocked });
            assert_eq!(task.evidence, [0; 16]);
            assert_eq!(task.execution_token, 0);
            assert_eq!(task.attempts, completed.tasks[index].unwrap().attempts);
        }
        let starts = executor.starts;
        let events = service.event_count();
        service.mark_runtime_failure(id, owner).unwrap();
        assert_eq!(*service.inspect(id, owner).unwrap(), failed);
        assert_eq!(service.event_count(), events);
        assert_eq!(service.poll(id, owner, &registry, &mut executor, 100).unwrap().state, PlanState::Failed);
        assert_eq!(executor.starts, starts);
        assert_eq!(service.begin(id, owner, &registry), Err(ServiceError::Busy));
    }
    let mut service = Service::new();
    let id = service.propose(owner, RequestOrigin::Text, b"create a graphical notes app", &registry, 0, limits()).unwrap();
    let proposed = *service.inspect(id, owner).unwrap();
    assert_eq!(service.mark_runtime_failure(id, owner), Err(ServiceError::InvalidPlan));
    assert_eq!(*service.inspect(id, owner).unwrap(), proposed);
    service.begin(id, owner, &registry).unwrap();
    let mut executor = HarnessExecutor::new();
    service.poll(id, owner, &registry, &mut executor, 1).unwrap();
    assert_eq!(service.inspect(id, owner).unwrap().tasks[0].unwrap().state, TaskState::Running);
    // A native failure can race the plan's consumption of earlier receipts.
    service.mark_runtime_failure(id, owner).unwrap();
    assert!(service.inspect(id, owner).unwrap().tasks.iter().flatten()
        .all(|task| task.state != TaskState::Running && task.execution_token == 0));
}

// ------------------------=
// FUNC: native_software_plan
// DESC: Exercises the real shared voice/text router and native plan executor with actual typed lifecycle receipts for differing app goals.
// ------------------=
fn native_software_plan() {
    use runtime::ai::intelligence::software::Event;
    let owner=[7;16];
    for (objective,voice,interaction) in [
        (b"generate a chess game and play me".as_slice(),true,true),
        (b"create a graphical calendar app".as_slice(),false,false),
    ] {
        let mut ai=runtime::ai::AiRuntime::new();
        ai.bind_chat_owner(owner);
        for byte in objective {assert!(ai.chat.push_input(*byte));}
        assert!(if voice {ai.submit_voice_chat()}else{ai.submit_chat()});
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().task_count,if interaction{3}else{2});
        assert!(ai.poll_qwen());
        let command=ai.take_control(owner,true).unwrap();
        assert!(matches!(command,runtime::ai::control::Command::Context(_)));
        assert_eq!(ai.software_execution_id(),Some(1));
        ai.finish_control(command,true);
        ai.poll_qwen();
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[0].unwrap().state,TaskState::Running);
        assert!(ai.record_software_event(1,owner,Event::SourceSaved{object:[2;16],revision:12}));
        ai.poll_qwen();
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[0].unwrap().state,TaskState::Running);
        assert!(ai.record_software_event(1,owner,Event::Compiled{artifact:[3;16]}));
        ai.poll_qwen();
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[0].unwrap().state,TaskState::Succeeded);
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[1].unwrap().state,TaskState::Running);
        assert!(ai.record_software_event(1,owner,Event::Launched{context:99}));
        ai.poll_qwen();
        if interaction {
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Executing);
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[2].unwrap().state,TaskState::Running);
            assert!(ai.record_software_event(1,owner,Event::InteractionReady{context:99,revision:1}));
            ai.poll_qwen();
        }
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Completed);
        assert_eq!(ai.software_execution(owner).unwrap().interaction_active,interaction);
        assert_eq!(ai.software_execution(owner).unwrap().applied_actions,0);
        assert!(ai.software_execution([8;16]).is_none());
    }

    let mut ai=runtime::ai::AiRuntime::new();
    ai.bind_chat_owner(owner);
    for byte in b"create a graphical notes app" {assert!(ai.chat.push_input(*byte));}
    assert!(ai.submit_chat());ai.poll_qwen();
    let command=ai.take_control(owner,true).unwrap();
    ai.finish_control(command,true);
    assert!(ai.record_software_event(1,owner,Event::Failed{code:7}));
    ai.poll_qwen();
    assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Failed);
    assert!(!ai.control_pending());
    for byte in b"open browser" {assert!(ai.chat.push_input(*byte));}
    assert!(ai.submit_voice_chat());
    assert_eq!(ai.take_control(owner,true),Some(runtime::ai::control::Command::Browser));
}

// ------------------------=
// FUNC: runtime_failure_releases_waiting_chat
// DESC: Retires only the failed active goal's waiting chat state and preserves a newer voice command after an older completed goal fails.
// ------------------=
fn runtime_failure_releases_waiting_chat() {
    use runtime::ai::{chat::GenerationState, control::Command, intelligence::software::Event};
    let owner=[7;16];
    for newer_command in [false,true] {
        let mut ai=runtime::ai::AiRuntime::new();ai.bind_chat_owner(owner);
        for byte in b"create a graphical notes app" {assert!(ai.chat.push_input(*byte));}
        assert!(ai.submit_voice_chat());assert!(ai.poll_qwen());
        let command=ai.take_control(owner,true).unwrap();
        ai.finish_control(command,true);ai.poll_qwen();
        assert!(ai.record_software_event(1,owner,Event::SourceSaved{object:[2;16],revision:12}));
        assert!(ai.record_software_event(1,owner,Event::Compiled{artifact:[3;16]}));
        ai.poll_qwen();
        assert!(ai.record_software_event(1,owner,Event::Launched{context:99}));
        if newer_command {
            ai.poll_qwen();
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Completed);
            for byte in b"open browser" {assert!(ai.chat.push_input(*byte));}
            assert!(ai.submit_voice_chat());
            assert_eq!(ai.chat.generation_state,GenerationState::Running);
        } else {
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Executing);
            // Exercise the waiting state used by asynchronous voice ingress.
            ai.chat.generation_state=GenerationState::Running;
        }
        let turn=ai.chat.turn_id();
        let chat_state=ai.chat.state_hash();
        assert!(!ai.record_software_event(1,owner,Event::RuntimeFailed {
            source:[2;16],revision:11,context:99,code:3,
        }));
        assert_eq!(ai.chat.state_hash(),chat_state);
        assert!(ai.record_software_event(1,owner,Event::RuntimeFailed {
            source:[2;16],revision:12,context:99,code:3,
        }));
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Failed);
        assert_eq!(ai.software_execution_id(),None);
        assert_eq!(ai.app_turn_owner(),None);
        assert_eq!(ai.chat.turn_id(),turn);
        if newer_command {
            assert_eq!(ai.chat.generation_state,GenerationState::Running);
            assert_eq!(ai.chat.state_hash(),chat_state);
        } else {
            assert_eq!(ai.chat.generation_state,GenerationState::Failed);
            assert!(!ai.control_pending());
            assert!(!ai.poll_qwen());
            assert_eq!(ai.chat.generation_state,GenerationState::Failed);
            for byte in b"open browser" {assert!(ai.chat.push_input(*byte));}
            assert!(ai.submit_voice_chat());
        }
        assert_eq!(ai.take_control(owner,true),Some(Command::Browser));
        ai.finish_control(Command::Browser,true);
        assert_eq!(ai.chat.generation_state,GenerationState::Complete);
        assert!(!ai.control_pending());
    }
}

// ------------------------=
// FUNC: interrupt_software_plan
// DESC: Proves stop and new voice requests release exact plan leases while stale callbacks cannot complete a replacement goal.
// ------------------=
fn interrupt_software_plan() {
    use runtime::ai::intelligence::software::Event;
    let owner=[7;16];
    for explicit_stop in [true,false] {
        let mut ai=runtime::ai::AiRuntime::new();ai.bind_chat_owner(owner);
        for byte in b"create a graphical notes app" {assert!(ai.chat.push_input(*byte));}
        assert!(ai.submit_voice_chat());ai.poll_qwen();
        let command=ai.take_control(owner,true).unwrap();
        ai.finish_control(command,true);ai.poll_qwen();
        if explicit_stop {
            assert!(ai.cancel_chat());
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Interrupted);
            assert_eq!(ai.take_intelligence_checkpoint_request(),Some(1));
        }
        for byte in b"create a graphical calendar app" {assert!(ai.chat.push_input(*byte));}
        assert!(ai.submit_voice_chat());ai.poll_qwen();
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Interrupted);
        assert_eq!(ai.software_execution_id(),Some(2));
        assert!(!ai.record_software_event(1,owner,Event::SourceSaved{object:[2;16],revision:1}));
        assert!(!ai.record_software_event(1,owner,Event::Compiled{artifact:[3;16]}));
        assert!(!ai.record_software_event(1,owner,Event::Failed{code:9}));
        assert_eq!(ai.software_execution(owner).unwrap().source,[0;16]);
        let replacement=ai.take_control(owner,true).unwrap();
        ai.finish_control(command,true);
        assert_eq!(ai.intelligence_plan(2,owner).unwrap().tasks[0].unwrap().state,TaskState::Running);
        ai.finish_control(replacement,true);
        assert!(ai.record_software_event(2,owner,Event::SourceSaved{object:[4;16],revision:2}));
        assert!(ai.record_software_event(2,owner,Event::Compiled{artifact:[5;16]}));
        assert!(ai.record_software_event(2,owner,Event::Launched{context:3}));
        ai.poll_qwen();ai.poll_qwen();
        assert_eq!(ai.intelligence_plan(2,owner).unwrap().state,PlanState::Completed);
    }
}

// ------------------------=
// FUNC: queued_goal_fencing
// DESC: Exercises queued replacement, exact-goal cancellation, stale admission and receipt rejection without requiring inference weights.
// ------------------=
fn queued_goal_fencing() {
    use runtime::ai::kit::RequestRoute;
    use runtime::ai::intelligence::software::Event;
    use ui::app_assistant::{self as assistant,Action,GenerationStatus,Panel};
    let owner=[7;16];
    for replacement in [b"create a graphical calendar app".as_slice(),b"open browser"] {
        assistant::reset();
        let mut ai=runtime::ai::AiRuntime::new();ai.bind_chat_owner(owner);
        assert_eq!(ai.route_application_request(2,b"create a graphical notes app"),Ok(RequestRoute::Goal(1)));
        ai.poll_qwen();
        let command=ai.take_control(owner,true).unwrap();ai.finish_control(command,true);ai.poll_qwen();
        let mut old=Panel::new();old.workflow_goal=Some(1);old.generation=GenerationStatus::Queued;
        old.argument[..3].copy_from_slice(b"old");old.argument_len=3;old.pending=Action::ReplaceCompile;
        let mut delivered=old;delivered.stream.delivered=2;
        assert!(delivered.cancel_workflow(1));assert_eq!(&delivered.argument[..delivered.argument_len],b"ol");
        assert_eq!(delivered.argument[2],0);
        assistant::write(2,old);
        let mut unrelated=old;unrelated.workflow_goal=Some(99);assistant::write(3,unrelated);
        assert!(ai.generation_workflow_current(Some(1)));
        assert_eq!(ai.software_goal_for_request(b"create a graphical notes app"),Some(1));
        let routed=ai.route_application_request(2,replacement).unwrap();
        let cancelled=assistant::read(2);
        assert_eq!(cancelled.generation,GenerationStatus::Cancelled);
        assert_eq!(cancelled.pending,Action::None);
        assert_eq!(cancelled.argument_len,0);assert_eq!(&cancelled.argument[..3],&[0;3]);
        assert!(assistant::read(3)==unrelated);
        assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Interrupted);
        assert!(!ai.generation_workflow_current(Some(1)));
        assert!(!old.workflow_matches(ai.software_execution_id()));
        assert!(ai.submit_app_turn(2,b"stale prompt").is_err());
        assert_eq!(ai.app_turn_owner(),None);
        if routed==RequestRoute::Goal(2) {
            ai.poll_qwen();
            assert!(ai.generation_workflow_current(Some(2)));
            assert_eq!(ai.software_goal_for_request(b"create a graphical notes app"),None);
            assert_eq!(ai.software_goal_for_request(replacement),Some(2));
            assert!(!ai.record_software_event(1,owner,Event::Failed{code:5}));
            assert!(!ai.record_software_event(1,owner,Event::SourceSaved{object:[9;16],revision:3}));
            assert_eq!(ai.software_execution(owner).unwrap().source,[0;16]);
            assert_eq!(ai.intelligence_plan(2,owner).unwrap().state,PlanState::Executing);
        } else {
            assert_eq!(routed,RequestRoute::Control(runtime::ai::control::Command::Browser));
        }
    }
    assistant::reset();
}

// ------------------------=
// FUNC: completed_source_edit_fence
// DESC: Proves editing after model completion cancels pending drain and compile, preserves user edits and delivered source, and erases undelivered bytes.
// ------------------=
fn completed_source_edit_fence() {
    use ui::app_assistant::{Action,GenerationStatus,Panel};
    let source=b"#include <stdio.h>\nint main(void){return 0;}";
    for delivered in [0,source.len()] {
        let mut panel=Panel::new();panel.workflow_goal=Some(1);panel.document_revision=17;
        let request=b"create a graphical notes app";
        panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
        assert!(panel.accept_generated(source,true,true));
        panel.generation=GenerationStatus::Complete;panel.stream.delivered=delivered;
        assert!(panel.apply_generated_compile_requested());
        assert!(!panel.cancel_source_on_revision(17));
        let mut document=ui::text_editor::TextDocument::new();
        for byte in b"user edits" {assert!(document.insert(*byte));}
        assert!(panel.cancel_source_on_revision(18));
        assert_eq!(panel.generation,GenerationStatus::Cancelled);
        assert_eq!(panel.pending,Action::None);
        assert!(!panel.apply_generated_compile_requested());
        assert_eq!(&panel.argument[..panel.argument_len],&source[..delivered]);
        assert!(panel.argument[delivered..source.len()].iter().all(|byte|*byte==0));
        assert_eq!(document.bytes(),b"user edits");
    }
}

// ------------------------=
// FUNC: main
// DESC: Verifies IntelligenceKit routing, discovery, DAG execution, recovery, persistence, ingress parity, and authorization.
// ------------------=
fn main() {
    language_and_update_plans();
    software_receipts();
    software_runtime_failure_receipts();
    software_runtime_failure_plan();
    native_software_plan();
    runtime_failure_releases_waiting_chat();
    interrupt_software_plan();
    queued_goal_fencing();
    completed_source_edit_fence();
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
    resumed.begin(generated,owner,&registry).unwrap();
    assert_eq!(resumed.inspect(generated,owner).unwrap().tasks.iter().flatten()
        .filter(|task|task.state==TaskState::Succeeded).count(),completed_before);

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

// ------------------------=
// FUNC: language_and_update_plans
// DESC: Verifies shared voice/text routing, native validation evidence and explicit update-launch dependency authority.
// ------------------=
fn language_and_update_plans() {
    use runtime::ai::intelligence::software::Event;
    let owner=[7;16];
    for voice in [false,true] {
        for (request,launch) in [
            (b"Create Hello world in Python".as_slice(),true),
            (b"update the calculator app and add numeric buttons and traditional mathematic calculator operators",false),
            (b"update the calendar app and add weekly navigation and launch",true),
        ] {
            let mut ai=runtime::ai::AiRuntime::new();ai.bind_chat_owner(owner);
            for byte in request {assert!(ai.chat.push_input(*byte));}
            assert!(if voice {ai.submit_voice_chat()} else {ai.submit_chat()});ai.poll_qwen();
            let command=ai.take_control(owner,true).unwrap();
            assert!(matches!(command,runtime::ai::control::Command::Context(_)));
            assert_eq!(ai.software_goal_for_request(request),Some(1));
            ai.finish_control(command,true);ai.poll_qwen();
            let plan=ai.intelligence_plan(1,owner).unwrap();assert_eq!(plan.task_count,if launch {2}else{1});
            assert_eq!(plan.tasks[0].unwrap().capability,CapabilityId::DevKit);
            assert_eq!(plan.tasks[0].unwrap().state,TaskState::Running);
            assert!(!ai.record_software_event(1,[8;16],Event::SourceSaved{object:[2;16],revision:1}));
            assert!(!ai.record_software_event(1,owner,Event::Compiled{artifact:[2;16]}));
            assert!(ai.record_software_event(1,owner,Event::SourceSaved{object:[2;16],revision:1}));ai.poll_qwen();
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Executing);
            assert!(ai.record_software_event(1,owner,Event::Compiled{artifact:[2;16]}));ai.poll_qwen();
            if launch {
                assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Executing);
                assert_eq!(ai.intelligence_plan(1,owner).unwrap().tasks[1].unwrap().capability,CapabilityId::UiServices);
                assert!(ai.record_software_event(1,owner,Event::Launched{context:55}));ai.poll_qwen();
            }
            assert_eq!(ai.intelligence_plan(1,owner).unwrap().state,PlanState::Completed);
            assert_eq!(ai.software_execution(owner).unwrap().context,if launch {55}else{0});
        }
    }
}
