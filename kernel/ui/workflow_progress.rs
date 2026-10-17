//! Bounded, allocation-free ordering for spoken structural workflow milestones.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Step {
    CompileStarted = 0,
    Linking = 1,
    ExecutableReady = 2,
    Launching = 3,
    Running = 4,
    Failed = 5,
    Cancelled = 6,
    LaunchFailed = 7,
}

impl Step {
    // ------------------------=
    // FUNC: phrase
    // DESC: Returns the concise spoken description for one typed workflow transition.
    // ------------------=
    pub const fn phrase(self) -> &'static [u8] {
        match self {
            Self::CompileStarted => b"Compilation started.",
            Self::Linking => b"Source compiled. Linking the application.",
            Self::ExecutableReady => b"Compilation completed. The executable is ready.",
            Self::Launching => b"Launching the application.",
            Self::Running => b"The application is running.",
            Self::Failed => b"Compilation failed. Review the compiler output.",
            Self::Cancelled => b"Compilation cancelled.",
            Self::LaunchFailed => b"The application could not be launched.",
        }
    }

    // ------------------------=
    // FUNC: bit
    // DESC: Maps a typed milestone to its stable one-time workflow mask.
    // ------------------=
    const fn bit(self) -> u8 { 1 << self as u8 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pending: [Step; 8],
    head: usize,
    length: usize,
    scheduled: u8,
    announced: u8,
    in_flight: Option<Step>,
}

impl Progress {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty milestone sequence with no retained workflow state.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: [Step::CompileStarted; 8], head: 0, length: 0,
            scheduled: 0, announced: 0, in_flight: None }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts a new workflow and schedules its initial compile announcement exactly once.
    // ------------------=
    pub fn begin(&mut self) {
        *self = Self::new();
        let _ = self.schedule(Step::CompileStarted);
    }

    // ------------------------=
    // FUNC: schedule
    // DESC: Appends a logical milestone once while preserving transition order in fixed storage.
    // ------------------=
    pub fn schedule(&mut self, step: Step) -> bool {
        if self.scheduled & step.bit() != 0 || self.length == self.pending.len() { return false; }
        let tail = (self.head + self.length) % self.pending.len();
        self.pending[tail] = step;
        self.length += 1;
        self.scheduled |= step.bit();
        true
    }

    // ------------------------=
    // FUNC: next
    // DESC: Exposes only the oldest pending milestone while a prior announcement is not active.
    // ------------------=
    pub fn next(&self) -> Option<Step> {
        if self.in_flight.is_some() || self.length == 0 { None } else { Some(self.pending[self.head]) }
    }

    // ------------------------=
    // FUNC: submitted
    // DESC: Moves the currently pending milestone into flight only after speech accepts it.
    // ------------------=
    pub fn submitted(&mut self, step: Step) -> bool {
        if self.next() != Some(step) { return false; }
        self.head = (self.head + 1) % self.pending.len();
        self.length -= 1;
        self.in_flight = Some(step);
        true
    }

    // ------------------------=
    // FUNC: completed
    // DESC: Records one terminal speech result so the next ordered milestone may proceed without replay.
    // ------------------=
    pub fn completed(&mut self, played: bool) -> Option<Step> {
        let step = self.in_flight.take()?;
        if played { self.announced |= step.bit(); }
        Some(step)
    }

    // ------------------------=
    // FUNC: discard
    // DESC: Removes queued narration when its session or user voice preference is no longer valid.
    // ------------------=
    pub fn discard(&mut self) { *self = Self::new(); }

    // ------------------------=
    // FUNC: snapshot
    // DESC: Exposes content-free ordered state for behavioral verification and installed diagnostics.
    // ------------------=
    pub const fn snapshot(&self) -> (u8, u8, usize, Option<Step>) {
        (self.scheduled, self.announced, self.length, self.in_flight)
    }
}
