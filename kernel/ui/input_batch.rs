//! Coalesces presentation, never input events, during one bounded device drain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    None,
    Cursor { x: i32, y: i32 },
    Scene,
}

pub struct PresentationBatch {
    active: bool,
    pending: bool,
    cursor: Option<(i32, i32)>,
}
impl PresentationBatch {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an idle presentation gate without an input queue.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            active: false,
            pending: false,
            cursor: None,
        }
    }
    // ------------------------=
    // FUNC: enter
    // DESC: Returns ownership only to the outer drain so nested actions cannot flush it prematurely.
    // ------------------=
    pub fn enter(&mut self) -> bool {
        let owner = !self.active;
        self.active = true;
        owner
    }
    // ------------------------=
    // FUNC: active
    // DESC: Identifies a deferred presentation so completion telemetry is published only after the outer flush.
    // ------------------=
    pub fn active(&self) -> bool {
        self.active
    }
    // ------------------------=
    // FUNC: request
    // DESC: Records deferred damage while active; outside a drain presentation remains immediate.
    // ------------------=
    pub fn request(&mut self) -> bool {
        if self.active {
            self.pending = true;
            false
        } else {
            true
        }
    }
    // ------------------------=
    // FUNC: request_cursor
    // DESC: Retains only the newest visual cursor position without discarding any pointer action or overriding a pending scene.
    // ------------------=
    pub fn request_cursor(&mut self, x: i32, y: i32) -> bool {
        if self.active {
            self.cursor = Some((x, y));
            false
        } else {
            true
        }
    }
    // ------------------------=
    // FUNC: leave
    // DESC: Flushes exactly once for the outer drain, preserving all preceding state mutations.
    // ------------------=
    pub fn leave(&mut self, owner: bool) -> Presentation {
        if !owner {
            return Presentation::None;
        }
        self.active = false;
        let pending = self.pending;
        self.pending = false;
        let cursor = self.cursor.take();
        if pending { Presentation::Scene }
        else if let Some((x, y)) = cursor { Presentation::Cursor { x, y } }
        else { Presentation::None }
    }
}

// ------------------------=
// FUNC: drain
// DESC: Dispatches every ready device sample in order under a fixed fairness budget; leaves unread samples in the device or FIFO.
// ------------------=
pub fn drain<T>(budget: usize, mut read: impl FnMut() -> Option<T>, mut dispatch: impl FnMut(T)) -> usize {
    let mut count = 0;
    while count < budget {
        let Some(event) = read() else { break; };
        dispatch(event);
        count += 1;
    }
    count
}

// ------------------------=
// FUNC: bounded_input_drain_preserves_every_action
// DESC: Verifies ten state transitions precede one presentation, nesting cannot flush, and idle input stays immediate.
// ------------------=
#[test]
fn bounded_input_drain_preserves_every_action() {
    let mut gate = PresentationBatch::new();
    let mut accepted = 0;
    assert!(gate.request());
    let owner = gate.enter();
    for _ in 0..10 {
        accepted += 1;
        assert!(!gate.request());
    }
    let nested = gate.enter();
    assert_eq!(gate.leave(nested), Presentation::None);
    assert_eq!(accepted, 10);
    assert_eq!(gate.leave(owner), Presentation::Scene);
    assert_eq!(gate.leave(true), Presentation::None);
    assert!(gate.request());
}

// ------------------------=
// FUNC: service_batch_dispatches_speech_before_one_final_frame
// DESC: Exercises model/voice/action invalidations with nested service work and proves no intermediate paint delays speech dispatch.
// ------------------=
#[test]
fn service_batch_dispatches_speech_before_one_final_frame() {
    let mut gate = PresentationBatch::new();
    let owner = gate.enter();
    let mut paints = 0;
    let mut service_steps = 0;
    for _ in 0..3 {
        service_steps += 1;
        if gate.request() { paints += 1; }
    }
    let nested = gate.enter();
    assert_eq!(gate.leave(nested), Presentation::None);
    let speech_dispatched = service_steps == 3;
    assert!(speech_dispatched);
    assert_eq!(paints, 0);
    // Animation invalidation belongs to this service frame, not a separate
    // frame rendered ahead of inference and speech dispatch.
    assert!(!gate.request());
    assert_eq!(paints, 0);
    if gate.leave(owner) == Presentation::Scene { paints += 1; }
    assert_eq!(paints, 1);
    assert_eq!(gate.leave(owner), Presentation::None);
}

// ------------------------=
// FUNC: pointer_drain_keeps_edges_and_one_latest_presentation
// DESC: Executes mixed motion, press, release, and wheel events before one visual flush, with bounded FIFO continuation.
// ------------------=
#[test]
fn pointer_drain_keeps_edges_and_one_latest_presentation() {
    let events = [(1, 0, 0), (2, 1, 0), (3, 1, 0), (4, 0, 0), (5, 0, 1), (6, 0, 0)];
    let mut input = events.into_iter();
    let mut dispatched = std::vec::Vec::new();
    let mut gate = PresentationBatch::new();
    let owner = gate.enter();
    assert_eq!(drain(4, || input.next(), |event| {
        dispatched.push(event);
        assert!(!gate.request_cursor(event.0, 10));
    }), 4);
    assert_eq!(dispatched, events[..4]);
    assert_eq!(gate.leave(owner), Presentation::Cursor { x: 4, y: 10 });
    let owner = gate.enter();
    assert_eq!(drain(4, || input.next(), |event| {
        dispatched.push(event);
        assert!(!gate.request_cursor(event.0, 10));
        if event.2 != 0 { assert!(!gate.request()); }
    }), 2);
    assert_eq!(dispatched, events);
    assert_eq!(gate.leave(owner), Presentation::Scene);
    assert_eq!(gate.leave(true), Presentation::None);
    assert!(gate.request_cursor(7, 10));
}

// ------------------------=
// FUNC: drain_budget_bounds_work_without_dropping_relative_samples
// DESC: Verifies firmware-sized bursts preserve each relative delta and button edge, present once, and leave excess input for a fair next cycle.
// ------------------=
#[test]
fn drain_budget_bounds_work_without_dropping_relative_samples() {
    let events: std::vec::Vec<_> = (0..35).map(|i| (i % 5 - 2, i % 2)).collect();
    let mut queue = events.iter().copied();
    let mut gate = PresentationBatch::new();
    let mut dispatched = std::vec::Vec::new();
    let mut positions = std::vec::Vec::new();
    let mut position = 0;
    let mut presentations = 0;
    for expected in [16, 16, 3] {
        let owner = gate.enter();
        let mut reads = 0;
        assert_eq!(drain(16, || { reads += 1; queue.next() }, |event| {
            dispatched.push(event);
            position += event.0;
            positions.push(position);
            assert!(!gate.request_cursor(position, 0));
        }), expected);
        assert_eq!(reads, if expected == 16 { 16 } else { 4 });
        assert_eq!(gate.leave(owner), Presentation::Cursor { x: position, y: 0 });
        presentations += 1;
    }
    assert_eq!(dispatched, events);
    assert_eq!(positions.len(), 35);
    assert_eq!(presentations, 3);
    assert_eq!(queue.next(), None);
    assert_eq!(drain::<()>(0, || panic!("zero budget must not read"), |_| {}), 0);
}
