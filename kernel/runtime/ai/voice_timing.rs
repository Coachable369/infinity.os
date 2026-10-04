//! Allocation-free, content-free timing for one accepted conversational turn.
//! Missing milestones stay explicit: zero duration is never evidence of success.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Milestone {
    Endpoint,
    Transcript,
    Submitted,
    FirstText,
    SpeechQueued,
    FirstAudio,
    Drained,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeline {
    pub sequence: u64,
    present: u64,
    timestamps: [u64; 7],
}

impl Timeline {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an inactive timeline without treating absent events as completed stages.
    // ------------------=
    pub const fn new() -> Self {
        Self { sequence: 0, present: 0, timestamps: [0; 7] }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Starts a distinct utterance at its measured endpoint and discards stale milestones.
    // ------------------=
    pub fn begin(&mut self, endpoint_ns: u64) {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.present = 0;
        self.timestamps.fill(0);
        self.observe(Milestone::Endpoint, endpoint_ns);
    }

    // ------------------------=
    // FUNC: observe
    // DESC: Records each actual milestone once; repeated polls cannot move first-response timings.
    // ------------------=
    pub fn observe(&mut self, event: Milestone, now: u64) {
        if self.sequence == 0 { return; }
        let index = event as usize;
        let mask = 1 << index;
        if self.present & mask == 0 {
            self.timestamps[index] = now;
            self.present |= mask;
        }
    }

    // ------------------------=
    // FUNC: at
    // DESC: Exposes an observed timestamp without conflating a missing event with clock zero.
    // ------------------=
    pub fn at(&self, event: Milestone) -> Option<u64> {
        (self.present & (1 << event as usize) != 0).then_some(self.timestamps[event as usize])
    }

    // ------------------------=
    // FUNC: elapsed
    // DESC: Returns a duration only when both measured milestones exist and their clock order is valid.
    // ------------------=
    pub fn elapsed(&self, start: Milestone, end: Milestone) -> Option<u64> {
        let mask = (1 << start as usize) | (1 << end as usize);
        if self.present & mask != mask { return None; }
        self.timestamps[end as usize].checked_sub(self.timestamps[start as usize])
    }

    // ------------------------=
    // FUNC: complete
    // DESC: Distinguishes a fully drained spoken turn from queued, text-only, failed or cancelled work.
    // ------------------=
    pub fn complete(&self) -> bool {
        self.present == 0x7f && self.timestamps.windows(2).all(|pair| pair[0] <= pair[1])
    }

    // ------------------------=
    // FUNC: record
    // DESC: Exports version, sequence, presence flags and seven absolute monotonic times without user content.
    // ------------------=
    pub fn record(&self) -> [u64; 10] {
        let mut result = [0; 10];
        result[0] = 1;
        result[1] = self.sequence;
        result[2] = self.present;
        result[3..].copy_from_slice(&self.timestamps);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // ------------------------=
    // FUNC: measurements_require_real_milestones
    // DESC: Rejects absent audio as zero latency and computes actual endpoint-to-playback time.
    // ------------------=
    fn measurements_require_real_milestones() {
        let mut timing = Timeline::new();
        timing.begin(100);
        timing.observe(Milestone::Transcript, 200);
        timing.observe(Milestone::Submitted, 220);
        timing.observe(Milestone::FirstText, 300);
        timing.observe(Milestone::SpeechQueued, 350);
        assert_eq!(timing.elapsed(Milestone::Endpoint, Milestone::FirstAudio), None);
        assert!(!timing.complete());
        timing.observe(Milestone::FirstAudio, 900);
        timing.observe(Milestone::Drained, 1900);
        assert_eq!(timing.elapsed(Milestone::Endpoint, Milestone::FirstAudio), Some(800));
        assert_eq!(timing.elapsed(Milestone::SpeechQueued, Milestone::FirstAudio), Some(550));
        assert!(timing.complete());
        assert_eq!(timing.record(), [1, 1, 127, 100, 200, 220, 300, 350, 900, 1900]);
    }

    #[test]
    // ------------------------=
    // FUNC: repeated_polls_and_new_turns_are_isolated
    // DESC: Preserves first-event timing and prevents a previous turn's playback from satisfying a new one.
    // ------------------=
    fn repeated_polls_and_new_turns_are_isolated() {
        let mut timing = Timeline::new();
        timing.begin(0);
        timing.observe(Milestone::FirstAudio, 100);
        timing.observe(Milestone::FirstAudio, 200);
        assert_eq!(timing.elapsed(Milestone::Endpoint, Milestone::FirstAudio), Some(100));
        timing.begin(300);
        assert_eq!(timing.sequence, 2);
        assert_eq!(timing.elapsed(Milestone::Endpoint, Milestone::FirstAudio), None);
        assert_eq!(timing.record(), [1, 2, 1, 300, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    // ------------------------=
    // FUNC: clock_reversal_is_not_a_fast_response
    // DESC: Refuses invalid backwards timestamps rather than reporting a misleading performance pass.
    // ------------------=
    fn clock_reversal_is_not_a_fast_response() {
        let mut timing = Timeline::new();
        timing.begin(100);
        timing.observe(Milestone::FirstAudio, 90);
        assert_eq!(timing.elapsed(Milestone::Endpoint, Milestone::FirstAudio), None);
        assert!(!timing.complete());
    }
}
