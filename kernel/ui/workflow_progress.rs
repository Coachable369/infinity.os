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
    Planning = 8,
    PlanReady = 9,
    Generating = 10,
    SourceReady = 11,
    Saved = 12,
    InteractionReady = 13,
    GenerationFailed = 14,
    GenerationCancelled = 15,
    Repairing = 16,
    SourceStreaming = 17,
    ContextPreparing = 18,
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
            Self::Planning => b"Planning the application workflow.",
            Self::PlanReady => b"The application plan is ready.",
            Self::Generating => b"Writing the application in the text editor.",
            Self::SourceReady => b"Source generation completed.",
            Self::Saved => b"The source file is saved.",
            Self::InteractionReady => b"Application controls are connected. I am ready to interact.",
            Self::GenerationFailed => b"Application generation failed. Your editor content is preserved.",
            Self::GenerationCancelled => b"Application generation cancelled.",
            Self::Repairing => b"The compiler found issues. Repairing the source using its diagnostics.",
            Self::SourceStreaming => b"Code is arriving in the editor. I am continuing the implementation.",
            Self::ContextPreparing => b"The local model is preparing the coding context. Source will appear as generation begins.",
        }
    }

    // ------------------------=
    // FUNC: bit
    // DESC: Maps a typed milestone to its stable one-time workflow mask.
    // ------------------=
    const fn bit(self) -> u32 { 1 << self as u8 }

    // ------------------------=
    // FUNC: terminal_failure
    // DESC: Identifies unsuccessful workflow outcomes that supersede queued progress announcements.
    // ------------------=
    const fn terminal_failure(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled | Self::LaunchFailed
            | Self::GenerationFailed | Self::GenerationCancelled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GenerationLease {
    goal: u64,
    request: [u8; 1024],
    length: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pending: [Step; 19],
    head: usize,
    length: usize,
    scheduled: u32,
    announced: u32,
    cycle: u32,
    in_flight: Option<Step>,
    output_request: Option<u64>,
    terminated: bool,
    generation_started_ns: Option<u64>,
    prefill_tokens: Option<usize>,
    generation_lease: Option<GenerationLease>,
}

impl Progress {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty milestone sequence with no retained workflow state.
    // ------------------=
    pub const fn new() -> Self {
        Self { pending: [Step::CompileStarted; 19], head: 0, length: 0,
            scheduled: 0, announced: 0, cycle: 0, in_flight: None, output_request: None,
            terminated: false, generation_started_ns: None, prefill_tokens: None,
            generation_lease: None }
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
    // FUNC: begin_generation
    // DESC: Starts one generic application workflow before any source or executable exists.
    // ------------------=
    pub fn begin_generation(&mut self) {
        *self = Self::new();
        let _ = self.schedule(Step::Planning);
    }

    // ------------------------=
    // FUNC: begin_generation_for
    // DESC: Binds narration to the already assigned goal and exact bounded request, not a later ambient editor turn.
    // ------------------=
    pub fn begin_generation_for(&mut self, goal:u64, request:&[u8]) -> bool {
        if request.is_empty() || request.len()>1024 {return false;}
        let mut lease=GenerationLease {goal,request:[0;1024],length:request.len()};
        lease.request[..request.len()].copy_from_slice(request);
        self.begin_generation();
        self.generation_lease=Some(lease);
        true
    }

    // ------------------------=
    // FUNC: generation_matches
    // DESC: Requires both immutable goal identity and exact request bytes before attributing editor progress to this narration.
    // ------------------=
    pub fn generation_matches(&self, goal:Option<u64>, request:&[u8]) -> bool {
        self.generation_lease.as_ref().is_some_and(|lease|
            goal==Some(lease.goal) && request==&lease.request[..lease.length])
    }

    // ------------------------=
    // FUNC: generation_terminal
    // DESC: Queues a terminal source result only for the exact still-authorized generation before its lease is retired.
    // ------------------=
    pub fn generation_terminal(&mut self, goal:Option<u64>, request:&[u8], current:bool, step:Step) -> bool {
        current && self.generating() && self.generation_matches(goal,request)
            && matches!(step,Step::GenerationFailed|Step::GenerationCancelled)
            && self.schedule(step)
    }

    // ------------------------=
    // FUNC: retain_generation_terminal
    // DESC: Retains a published source-failure receipt after its plan fails, but discards interrupted or replaced narration without authorizing more source work.
    // ------------------=
    pub fn retain_generation_terminal(&mut self, goal:Option<u64>, request:&[u8], permitted:bool) -> bool {
        if !self.terminated || self.cycle & (Step::GenerationFailed.bit()|Step::GenerationCancelled.bit())==0 {
            return true;
        }
        if permitted && self.generation_matches(goal,request) {return true;}
        self.discard();false
    }

    // ------------------------=
    // FUNC: retain_generation
    // DESC: Discards stale generation speech after replacement or cancellation without revoking subsequent compiler and launch receipts.
    // ------------------=
    pub fn retain_generation(&mut self, goal:Option<u64>, request:&[u8], creates_application:bool, current:bool) -> bool {
        if !self.generating() {return true;}
        if creates_application && current && self.generation_matches(goal,request) {return true;}
        self.discard();
        false
    }

    // ------------------------=
    // FUNC: observes_generation
    // DESC: Accepts shared inference samples only while the exact planned editor source turn owns the model.
    // ------------------=
    pub fn observes_generation(&self, app_owner:Option<usize>, goal:Option<u64>, request:&[u8], creates_application:bool, current:bool) -> bool {
        app_owner==Some(2) && self.generating() && creates_application && current
            && self.generation_matches(goal,request)
    }

    // ------------------------=
    // FUNC: generating
    // DESC: Identifies a live generation workflow whose compilation should retain earlier narration.
    // ------------------=
    pub const fn generating(&self) -> bool {
        !self.terminated && self.cycle & (Step::Planning.bit() | Step::Repairing.bit()) != 0
            && self.cycle & Step::CompileStarted.bit() == 0
    }

    // ------------------------=
    // FUNC: generation_started
    // DESC: Records the accepted generation phase once so narration cannot reset its elapsed-time gate on every poll.
    // ------------------=
    pub fn generation_started(&mut self, now: u64) {
        if self.cycle & Step::Generating.bit() != 0 && self.generation_started_ns.is_none() {
            self.generation_started_ns = Some(now);
        }
    }

    // ------------------------=
    // FUNC: prefill_progress
    // DESC: Queues one factual delayed-prefill checkpoint only on observed token advancement, removing it when source work has begun.
    // ------------------=
    pub fn prefill_progress(&mut self, processed:usize, prompt:usize, generated:usize, delivered:usize, now:u64) -> bool {
        let previous=self.prefill_tokens.replace(processed);
        if generated!=0 || delivered!=0 || (prompt!=0 && processed>=prompt) {
            return self.discard_pending(Step::ContextPreparing);
        }
        let Some(started)=self.generation_started_ns else {return false;};
        if self.cycle & (Step::SourceReady.bit() | Step::CompileStarted.bit()) != 0
            || prompt==0 || processed==0 || previous.is_none_or(|tokens|processed<=tokens)
            || now.saturating_sub(started)<20_000_000_000 {return false;}
        self.schedule(Step::ContextPreparing)
    }

    // ------------------------=
    // FUNC: discard_pending
    // DESC: Removes only an unspoken obsolete checkpoint while preserving ordered work, in-flight speech and once-per-attempt history.
    // ------------------=
    fn discard_pending(&mut self, step:Step) -> bool {
        let mut pending=[Step::CompileStarted;19];
        let mut length=0;
        for index in 0..self.length {
            let current=self.pending[(self.head+index)%self.pending.len()];
            if current!=step {pending[length]=current;length+=1;}
        }
        if length==self.length {return false;}
        self.pending=pending;self.head=0;self.length=length;true
    }

    // ------------------------=
    // FUNC: prefill_finished
    // DESC: Retracts an unspoken prefill checkpoint when its generation lease is no longer running.
    // ------------------=
    pub fn prefill_finished(&mut self) -> bool {self.discard_pending(Step::ContextPreparing)}

    // ------------------------=
    // FUNC: source_progress
    // DESC: Announces one sustained-generation checkpoint only after measured time and source actually delivered to the editor.
    // ------------------=
    pub fn source_progress(&mut self, delivered: usize, now: u64) -> bool {
        let discarded=delivered!=0 && self.discard_pending(Step::ContextPreparing);
        let Some(started) = self.generation_started_ns else { return false; };
        if self.cycle & (Step::SourceReady.bit() | Step::CompileStarted.bit()) != 0
            || delivered < 1024 || now.saturating_sub(started) < 15_000_000_000 {
            return discarded;
        }
        self.schedule(Step::SourceStreaming) || discarded
    }

    // ------------------------=
    // FUNC: schedule
    // DESC: Appends each milestone once per accepted attempt, preserving history while superseding stale narration on bounded repair.
    // ------------------=
    pub fn schedule(&mut self, step: Step) -> bool {
        if self.terminated { return false; }
        if matches!(step,Step::SourceStreaming|Step::SourceReady|Step::CompileStarted) {
            self.discard_pending(Step::ContextPreparing);
        }
        if step == Step::Repairing {
            if self.cycle & Step::CompileStarted.bit() == 0 { return false; }
            self.head = 0;
            self.length = 0;
            self.generation_started_ns = None;
            self.prefill_tokens = None;
            self.cycle &= !(Step::Generating.bit() | Step::SourceStreaming.bit() | Step::ContextPreparing.bit()
                | Step::SourceReady.bit() | Step::Saved.bit() | Step::CompileStarted.bit()
                | Step::Linking.bit() | Step::Repairing.bit());
        }
        if self.cycle & step.bit() != 0 { return false; }
        if step.terminal_failure() {
            self.head = 0;
            self.length = 0;
            self.terminated = true;
        }
        if self.length == self.pending.len() { return false; }
        let tail = (self.head + self.length) % self.pending.len();
        self.pending[tail] = step;
        self.length += 1;
        self.scheduled |= step.bit();
        self.cycle |= step.bit();
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
    // FUNC: submitted_request
    // DESC: Binds an accepted milestone to its exact native speech request, not other output from the same session.
    // ------------------=
    pub fn submitted_request(&mut self, step:Step, request:u64) -> bool {
        if request==0 || !self.submitted(step) {return false;}
        self.output_request=Some(request);true
    }

    // ------------------------=
    // FUNC: output_request
    // DESC: Returns the active speech receipt so cleanup can cancel only this workflow's output.
    // ------------------=
    pub const fn output_request(&self) -> Option<u64> {self.output_request}

    // ------------------------=
    // FUNC: observe_output
    // DESC: Completes only the matching speech job; replacement drops the old milestone without replay or a false announcement.
    // ------------------=
    pub fn observe_output(&mut self, request:Option<u64>, terminal:Option<bool>) -> bool {
        if self.in_flight.is_none() {return false;}
        let played=if self.output_request.is_none() || self.output_request!=request {false}
            else if let Some(played)=terminal {played} else {return false;};
        self.completed(played).is_some()
    }

    // ------------------------=
    // FUNC: completed
    // DESC: Records one terminal speech result so the next ordered milestone may proceed without replay.
    // ------------------=
    pub fn completed(&mut self, played: bool) -> Option<Step> {
        let step = self.in_flight.take()?;
        self.output_request=None;
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
    pub const fn snapshot(&self) -> (u32, u32, usize, Option<Step>) {
        (self.scheduled, self.announced, self.length, self.in_flight)
    }
}
