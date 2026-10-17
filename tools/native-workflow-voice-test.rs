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
    assert_eq!(progress.next(), Some(Step::CompileStarted));
    progress.discard();
    assert_eq!(progress.snapshot(), (0, 0, 0, None));
    progress.begin();
    assert_eq!(progress.next(), Some(Step::CompileStarted));
    assert!(progress.submitted(Step::CompileStarted));
    assert_eq!(progress.completed(false), Some(Step::CompileStarted));
    assert_eq!(progress.snapshot().1, 0);
}
