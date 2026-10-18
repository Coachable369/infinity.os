#[path = "../kernel/ui/workflow_progress.rs"]
mod workflow_progress;

use workflow_progress::{Progress, Step};

// ------------------------=
// FUNC: main
// DESC: Verifies progressive workflow narration is ordered, bounded, and emitted once per logical transition.
// ------------------=
fn main() {
    let mut progress = Progress::new();
    progress.begin();
    assert!(!progress.schedule(Step::CompileStarted));
    assert!(progress.schedule(Step::Linking));
    assert!(progress.schedule(Step::ExecutableReady));
    assert!(progress.schedule(Step::Launching));
    assert!(progress.schedule(Step::Running));
    assert!(!progress.schedule(Step::Running));

    for expected in [Step::CompileStarted, Step::Linking, Step::ExecutableReady,
        Step::Launching, Step::Running] {
        assert_eq!(progress.next(), Some(expected));
        assert!(progress.submitted(expected));
        assert_eq!(progress.next(), None);
        assert_eq!(progress.completed(true), Some(expected));
    }
    let (scheduled, announced, pending, active) = progress.snapshot();
    assert_eq!(scheduled, 0b1_1111);
    assert_eq!(announced, scheduled);
    assert_eq!(pending, 0);
    assert_eq!(active, None);

    progress.begin();
    assert!(progress.schedule(Step::Failed));
    assert_eq!(progress.next(), Some(Step::Failed));
    assert!(!progress.schedule(Step::Launching));
    progress.discard();
    assert_eq!(progress.snapshot(), (0, 0, 0, None));
    progress.begin();
    assert_eq!(progress.next(), Some(Step::CompileStarted));
    assert!(progress.submitted(Step::CompileStarted));
    assert_eq!(progress.completed(false), Some(Step::CompileStarted));
    assert_eq!(progress.snapshot().1, 0);

    progress.begin_generation();
    assert!(progress.generating());
    let stages = [Step::Planning, Step::PlanReady, Step::Generating,
        Step::SourceReady, Step::Saved, Step::CompileStarted, Step::Linking,
        Step::ExecutableReady, Step::Launching, Step::Running, Step::InteractionReady];
    for &step in &stages[1..] { assert!(progress.schedule(step)); }
    assert!(!progress.generating());
    for step in stages {
        assert!(!progress.schedule(step));
        assert_eq!(progress.next(), Some(step));
        assert!(progress.submitted(step));
        assert_eq!(progress.completed(true), Some(step));
    }
    assert_eq!(progress.snapshot().0, progress.snapshot().1);
    assert_eq!(progress.snapshot().2, 0);

    // Failed and cancelled generation cannot speak stale saved/launch claims.
    // An already-owned speech phrase must drain before the terminal result.
    for terminal in [Step::GenerationFailed, Step::GenerationCancelled] {
        progress.begin_generation();
        assert!(progress.submitted(Step::Planning));
        assert!(progress.schedule(Step::PlanReady));
        assert!(progress.schedule(Step::Generating));
        assert!(progress.schedule(terminal));
        assert_eq!(progress.next(), None);
        assert_eq!(progress.snapshot().2, 1);
        assert!(!progress.schedule(Step::Saved));
        assert!(!progress.schedule(Step::Launching));
        assert_eq!(progress.completed(false), Some(Step::Planning));
        assert_eq!(progress.next(), Some(terminal));
        assert!(progress.submitted(terminal));
        assert_eq!(progress.completed(true), Some(terminal));
        assert_eq!(progress.next(), None);
    }
    progress.begin_generation();
    assert!(progress.generating());
    assert_eq!(progress.next(), Some(Step::Planning));
    repair_checkpoints();
    sustained_generation_checkpoint();
    prefill_checkpoint();
    generation_ownership();
    terminal_generation_handoff();
    exact_speech_receipts();
}

// ------------------------=
// FUNC: exact_speech_receipts
// DESC: Verifies current-job completion, same-session replacement, cancellation, and cleanup cannot attribute a foreign reply to a workflow milestone.
// ------------------=
fn exact_speech_receipts() {
    for observed in [None,Some(9),Some(10)] {
        for terminal in [None,Some(false),Some(true)] {
            let mut progress=Progress::new();progress.begin_generation();
            assert!(!progress.submitted_request(Step::Planning,0));
            assert_eq!(progress.next(),Some(Step::Planning));
            assert!(progress.submitted_request(Step::Planning,9));
            assert_eq!(progress.output_request(),Some(9));
            assert!(!progress.submitted_request(Step::Planning,10));
            assert!(progress.schedule(Step::GenerationCancelled));
            let finished=observed!=Some(9) || terminal.is_some();
            assert_eq!(progress.observe_output(observed,terminal),finished);
            let announced=observed==Some(9) && terminal==Some(true);
            assert_eq!(progress.snapshot().1,if announced {1<<Step::Planning as u8} else {0});
            if finished {
                assert_eq!(progress.output_request(),None);
                assert_eq!(progress.next(),Some(Step::GenerationCancelled));
                assert!(!progress.observe_output(Some(10),Some(true)));
            } else {
                assert_eq!(progress.output_request(),Some(9));
                assert_eq!(progress.next(),None);
            }
            progress.discard();
            assert_eq!(progress.output_request(),None);
            assert_eq!(progress.snapshot(),(0,0,0,None));
        }
    }
}

// ------------------------=
// FUNC: terminal_generation_handoff
// DESC: Exercises queued admission failure, edit cancellation and asynchronous failure before lease retirement while rejecting stale goals and unrelated terminal work.
// ------------------=
fn terminal_generation_handoff() {
    let request=b"create a graphical counter app";
    for terminal in [Step::GenerationFailed,Step::GenerationCancelled] {
        for phase in 0..3 {
            let mut progress=Progress::new();
            assert!(progress.begin_generation_for(7,request));
            assert!(progress.schedule(Step::PlanReady));
            if phase!=0 {
                assert!(progress.schedule(Step::Generating));
                progress.generation_started(1);
                assert!(progress.submitted(Step::Planning));
            }
            if phase==2 {
                assert!(!progress.prefill_progress(1,100,0,0,20_000_000_001));
                assert!(progress.prefill_progress(2,100,0,0,20_000_000_002));
            }
            assert!(progress.generation_terminal(Some(7),request,true,terminal));
            assert!(!progress.generation_terminal(Some(7),request,true,terminal));
            assert_eq!(progress.snapshot().2,1);
            // The already-published terminal receipt survives the normal
            // retirement of its plan; no stale progress can be added afterward.
            assert!(progress.retain_generation_terminal(Some(7),request,true));
            assert!(progress.retain_generation(Some(7),request,true,false));
            assert!(!progress.schedule(Step::Saved));
            assert!(!progress.schedule(Step::CompileStarted));
            if phase!=0 {
                assert_eq!(progress.next(),None);
                assert_eq!(progress.completed(true),Some(Step::Planning));
            }
            assert_eq!(progress.next(),Some(terminal));
            assert!(progress.submitted(terminal));
            assert_eq!(progress.completed(true),Some(terminal));
            assert_eq!(progress.snapshot().1 & (1u32<<(terminal as u8)),1u32<<(terminal as u8));
            assert_eq!(progress.snapshot().2,0);
        }
        for (goal,current_request,permitted) in [
            (Some(8),request.as_slice(),true),
            (Some(7),b"create a graphical notes app".as_slice(),true),
            (Some(7),request.as_slice(),false),
        ] {
            let mut progress=Progress::new();
            assert!(progress.begin_generation_for(7,request));
            assert!(progress.generation_terminal(Some(7),request,true,terminal));
            assert!(progress.submitted(terminal));
            assert!(!progress.retain_generation_terminal(goal,current_request,permitted));
            assert_eq!(progress.snapshot(),(0,0,0,None));
        }
        for (goal,current_request,current) in [
            (Some(8),request.as_slice(),true),
            (Some(7),b"create a graphical notes app".as_slice(),true),
            (None,request.as_slice(),true),
            (Some(7),request.as_slice(),false),
        ] {
            let mut progress=Progress::new();
            assert!(progress.begin_generation_for(7,request));
            let before=progress;
            assert!(!progress.generation_terminal(goal,current_request,current,terminal));
            assert_eq!(progress,before);
            assert!(!progress.retain_generation(goal,current_request,true,current));
            assert_eq!(progress.snapshot(),(0,0,0,None));
        }
    }
    let mut progress=Progress::new();
    assert!(progress.begin_generation_for(7,request));
    assert!(!progress.generation_terminal(Some(7),request,true,Step::Failed));
    assert!(progress.schedule(Step::CompileStarted));
    assert!(!progress.generation_terminal(Some(7),request,true,Step::GenerationFailed));
}

// ------------------------=
// FUNC: generation_ownership
// DESC: Exercises exact request and goal fencing, shared inference ownership, stale queue revocation and post-generation narration continuity.
// ------------------=
fn generation_ownership() {
    let request=b"Create a graphical calculator";
    let other=b"Create a graphical chess game";
    let goal=Some(41);
    let mut progress=Progress::new();
    assert!(!progress.begin_generation_for(41,b""));
    assert!(!progress.begin_generation_for(41,&[1;1025]));
    assert_eq!(progress.snapshot(),(0,0,0,None));
    let mut input=*request;
    assert!(progress.begin_generation_for(41,&input));
    input[0]=b'X';
    assert!(progress.generation_matches(goal,request));
    assert!(!progress.generation_matches(goal,&input));
    assert!(progress.retain_generation(goal,request,true,true));
    assert!(progress.observes_generation(Some(2),goal,request,true,true));
    for owner in [None,Some(0),Some(1),Some(3),Some(12)] {
        assert!(!progress.observes_generation(owner,goal,request,true,true));
    }
    for (panel_goal,panel_request,creates,current) in [
        (Some(42),request.as_slice(),true,true),
        (goal,other.as_slice(),true,true),
        (None,request.as_slice(),true,true),
        (goal,request.as_slice(),false,true),
        (goal,request.as_slice(),true,false),
        (None,b"Explain this source".as_slice(),false,true),
    ] {
        assert!(progress.begin_generation_for(41,request));
        assert!(progress.schedule(Step::Generating));
        progress.generation_started(1);
        assert!(!progress.prefill_progress(1,100,0,0,2));
        assert!(progress.prefill_progress(2,100,0,0,20_000_000_001));
        assert!(progress.submitted(Step::Planning));
        assert!(!progress.observes_generation(Some(2),panel_goal,panel_request,creates,current));
        assert!(!progress.retain_generation(panel_goal,panel_request,creates,current));
        assert_eq!(progress.snapshot(),(0,0,0,None));
        assert!(!progress.prefill_progress(3,100,0,0,30_000_000_000));
        assert!(!progress.source_progress(4096,30_000_000_000));
        assert_eq!(progress.next(),None);
    }
    // A replacement plan receives its own fresh sequence, never old attempts.
    assert!(progress.begin_generation_for(42,other));
    assert!(progress.generation_matches(Some(42),other));
    assert_eq!(progress.next(),Some(Step::Planning));
    assert!(!progress.observes_generation(Some(2),goal,request,true,true));
    // Compilation owns its receipts after source completes; a cleared editor
    // panel must not suppress compiler, launch or interaction-ready narration.
    for step in [Step::Generating,Step::SourceReady,Step::Saved,Step::CompileStarted,Step::Linking,
        Step::ExecutableReady,Step::Launching,Step::Running,Step::InteractionReady] {
        assert!(progress.schedule(step));
    }
    let saved=progress.snapshot();
    assert!(progress.retain_generation(None,b"",false,false));
    assert_eq!(progress.snapshot(),saved);
    assert!(!progress.observes_generation(Some(2),Some(42),other,true,true));
    assert!(progress.schedule(Step::Repairing));
    assert!(progress.retain_generation(Some(42),other,true,true));
    assert!(!progress.retain_generation(goal,request,true,true));
    assert_eq!(progress.snapshot(),(0,0,0,None));
    // Explicit terminal failure remains eligible to announce once after the
    // generation engine releases its active goal.
    assert!(progress.begin_generation_for(41,request));
    assert!(progress.schedule(Step::GenerationCancelled));
    assert!(progress.retain_generation(None,b"",false,false));
    assert_eq!(progress.next(),Some(Step::GenerationCancelled));
}

// ------------------------=
// FUNC: repair_checkpoints
// DESC: Preserves completed history across two accepted compiler repairs without announcing terminal failure or replaying the original plan.
// ------------------=
fn repair_checkpoints() {
    let mut progress=Progress::new();
    progress.begin_generation();
    assert!(progress.submitted(Step::Planning));
    assert!(progress.schedule(Step::PlanReady));
    assert!(progress.schedule(Step::Generating));
    assert!(progress.schedule(Step::SourceReady));
    assert!(progress.schedule(Step::Saved));
    assert!(progress.schedule(Step::CompileStarted));
    let initial=progress.snapshot().0;
    for attempt in 0..2 {
        assert!(progress.schedule(Step::Repairing));
        assert_eq!(progress.snapshot().0 & initial,initial);
        assert_eq!(progress.snapshot().0 & (1u32 << Step::Failed as u8),0);
        assert!(progress.generating());
        assert!(!progress.schedule(Step::Repairing));
        if attempt==0 {
            assert_eq!(progress.next(),None);
            assert_eq!(progress.completed(true),Some(Step::Planning));
        }
        assert_eq!(progress.next(),Some(Step::Repairing));
        assert!(progress.submitted(Step::Repairing));
        assert_eq!(progress.completed(true),Some(Step::Repairing));
        assert!(!progress.schedule(Step::Planning));
        assert!(!progress.schedule(Step::PlanReady));
        for step in [Step::Generating,Step::SourceReady,Step::Saved,Step::CompileStarted] {
            assert!(progress.schedule(step));
            assert_eq!(progress.next(),Some(step));
            assert!(progress.submitted(step));
            assert_eq!(progress.completed(true),Some(step));
        }
        assert!(!progress.generating());
    }
    assert!(progress.schedule(Step::Linking));
    assert!(progress.schedule(Step::ExecutableReady));
    assert!(progress.schedule(Step::Launching));
    assert!(progress.schedule(Step::Running));
    assert!(progress.schedule(Step::InteractionReady));
    for step in [Step::Linking,Step::ExecutableReady,Step::Launching,Step::Running,Step::InteractionReady] {
        assert_eq!(progress.next(),Some(step));
        assert!(progress.submitted(step));
        assert_eq!(progress.completed(true),Some(step));
    }
    assert_eq!(progress.next(),None);

    progress.begin();
    assert!(progress.schedule(Step::Repairing));
    assert!(progress.schedule(Step::Generating));
    assert!(progress.schedule(Step::CompileStarted));
    assert!(progress.schedule(Step::Failed));
    assert_eq!(progress.next(),Some(Step::Failed));
    assert!(!progress.schedule(Step::Repairing));
    assert!(!progress.schedule(Step::ExecutableReady));
}

// ------------------------=
// FUNC: sustained_generation_checkpoint
// DESC: Requires both elapsed time and real editor delivery, emits once per attempt, and stops checkpoint admission after source completion.
// ------------------=
fn sustained_generation_checkpoint() {
    let mut progress=Progress::new();
    progress.begin_generation();
    assert!(!progress.source_progress(4096,60_000_000_000));
    assert!(progress.schedule(Step::Generating));
    progress.generation_started(10_000_000_000);
    progress.generation_started(20_000_000_000);
    assert!(!progress.source_progress(4096,9_000_000_000));
    assert!(!progress.source_progress(4096,24_999_999_999));
    assert!(!progress.source_progress(1023,25_000_000_000));
    assert!(progress.source_progress(1024,25_000_000_000));
    for now in [25_000_000_000,60_000_000_000,600_000_000_000] {
        assert!(!progress.source_progress(8192,now));
    }
    assert!(progress.schedule(Step::SourceReady));
    assert!(!progress.source_progress(8192,600_000_000_000));
    assert!(progress.schedule(Step::CompileStarted));
    assert!(progress.schedule(Step::Repairing));
    assert!(!progress.source_progress(8192,600_000_000_000));
    assert!(progress.schedule(Step::Generating));
    progress.generation_started(600_000_000_000);
    assert!(!progress.source_progress(8192,614_999_999_999));
    assert!(progress.source_progress(8192,615_000_000_000));
    assert!(progress.schedule(Step::GenerationCancelled));
    assert!(!progress.source_progress(8192,900_000_000_000));
}

// ------------------------=
// FUNC: prefill_fixture
// DESC: Starts a real progress sequence with its initial announcements drained and a measured generation start.
// ------------------=
fn prefill_fixture() -> Progress {
    let mut progress=Progress::new();
    progress.begin_generation();
    assert!(progress.schedule(Step::PlanReady));
    assert!(progress.schedule(Step::Generating));
    progress.generation_started(10_000_000_000);
    for step in [Step::Planning,Step::PlanReady,Step::Generating] {
        assert!(progress.submitted(step));
        assert_eq!(progress.completed(true),Some(step));
    }
    progress
}

// ------------------------=
// FUNC: prefill_checkpoint
// DESC: Verifies measured prefill activity, one checkpoint per attempt, stale-queue removal and unchanged in-flight speech without text-based oracles.
// ------------------=
fn prefill_checkpoint() {
    assert_eq!(Step::SourceStreaming as u8,17);
    assert_eq!(Step::ContextPreparing as u8,18);
    let mut progress=Progress::new();
    assert!(!progress.prefill_progress(1,100,0,0,30_000_000_000));
    assert!(!progress.prefill_progress(2,100,0,0,31_000_000_000));
    assert_eq!(progress.snapshot(),(0,0,0,None));
    let mut progress=prefill_fixture();
    assert!(!progress.prefill_progress(10,100,0,0,29_000_000_000));
    assert!(!progress.prefill_progress(11,100,0,0,29_999_999_999));
    assert!(!progress.prefill_progress(11,100,0,0,30_000_000_000));
    assert!(!progress.prefill_progress(11,100,0,0,90_000_000_000));
    assert!(progress.prefill_progress(12,100,0,0,91_000_000_000));
    assert_eq!(progress.next(),Some(Step::ContextPreparing));
    assert!(!progress.prefill_progress(13,100,0,0,92_000_000_000));
    assert_eq!(progress.snapshot().2,1);
    assert!(progress.submitted(Step::ContextPreparing));
    assert_eq!(progress.completed(true),Some(Step::ContextPreparing));
    assert_eq!(progress.snapshot().1 & (1u32<<18),1u32<<18);
    assert!(!progress.prefill_progress(14,100,0,0,93_000_000_000));
    assert!(progress.schedule(Step::SourceReady));
    assert!(progress.schedule(Step::CompileStarted));
    assert!(progress.schedule(Step::Repairing));
    assert!(progress.schedule(Step::Generating));
    progress.generation_started(100_000_000_000);
    assert!(!progress.prefill_progress(1,100,0,0,120_000_000_000));
    assert!(progress.prefill_progress(2,100,0,0,120_000_000_001));
    for step in [Step::Repairing,Step::Generating,Step::ContextPreparing] {
        assert!(progress.submitted(step));assert_eq!(progress.completed(true),Some(step));
    }
    assert_eq!(progress.next(),None);

    for (processed,prompt,generated,delivered) in [(0,100,0,0),(2,0,0,0),(100,100,0,0),(101,100,0,0),(2,100,1,0),(2,100,0,1)] {
        let mut progress=prefill_fixture();
        assert!(!progress.prefill_progress(1,100,0,0,10_000_000_001));
        assert!(!progress.prefill_progress(processed,prompt,generated,delivered,30_000_000_000));
        assert_eq!(progress.next(),None);
    }
    for reason in 0..7 {
        let mut progress=prefill_fixture();
        assert!(!progress.prefill_progress(1,100,0,0,10_000_000_001));
        assert!(progress.prefill_progress(2,100,0,0,30_000_000_000));
        assert!(progress.schedule(Step::Saved));
        assert!(match reason {
            0=>progress.prefill_progress(3,100,1,0,31_000_000_000),
            1=>progress.source_progress(1,31_000_000_000),
            2=>progress.prefill_progress(100,100,0,0,31_000_000_000),
            3=>progress.prefill_finished(),
            4=>progress.schedule(Step::SourceReady),
            5=>progress.schedule(Step::GenerationFailed),
            _=>progress.schedule(Step::GenerationCancelled),
        });
        let expected=if reason==5 {Step::GenerationFailed} else if reason==6 {Step::GenerationCancelled} else {Step::Saved};
        assert_eq!(progress.next(),Some(expected));
        assert_eq!(progress.snapshot().0 & (1u32<<18),1u32<<18);
        assert_eq!(progress.snapshot().1 & (1u32<<18),0);
        assert!(!progress.prefill_progress(4,100,0,0,32_000_000_000));
    }
    let mut progress=prefill_fixture();
    assert!(!progress.prefill_progress(1,100,0,0,10_000_000_001));
    assert!(progress.prefill_progress(2,100,0,0,30_000_000_000));
    assert!(progress.submitted(Step::ContextPreparing));
    assert!(!progress.prefill_finished());
    assert!(progress.schedule(Step::SourceReady));
    assert_eq!(progress.next(),None);
    assert_eq!(progress.completed(true),Some(Step::ContextPreparing));
    assert_eq!(progress.next(),Some(Step::SourceReady));
}
