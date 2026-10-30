//! Allocation-free 16 kHz mono utterance segmentation. Energy VAD is not STT.
pub const RATE: usize = 16_000;
pub const FRAME: usize = 320;
pub const MAX_SAMPLES: usize = RATE * 10;
// Preserve a softly spoken wake prefix before the louder command crosses the
// onset gate. This is retained audio, not additional endpoint wait time.
const PRE_ROLL: usize = RATE * 3 / 5;
const ONSET_FRAMES: usize = 3;
const AMBIENT_FRAMES: usize = 9;
// A normal hesitation between a product name and the rest of a command must
// not submit that prefix as a finished instruction. This remains sample-based.
pub const END_SILENCE: usize = RATE * 4 / 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadState {
    Waiting,
    Speech,
    Complete,
    NoSpeech,
    Limit,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
}
pub struct Detector {
    state: VadState,
    samples: usize,
    frame_samples: usize,
    energy: u64,
    consecutive: usize,
    last_voice: usize,
    segment: Segment,
    threshold: u16,
    continuation_threshold: u16,
    waiting_energies: [u32; AMBIENT_FRAMES],
    waiting_frames: usize,
    limit_silence: usize,
}
impl Detector {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed 20-ms RMS gate with three-frame onset and bounded silence timeout.
    // ------------------=
    pub const fn new(threshold: u16) -> Self {
        Self {
            state: VadState::Waiting,
            samples: 0,
            frame_samples: 0,
            energy: 0,
            consecutive: 0,
            last_voice: 0,
            segment: Segment { start: 0, end: 0 },
            threshold,
            continuation_threshold: threshold,
            waiting_energies: [0; AMBIENT_FRAMES],
            waiting_frames: 0,
            limit_silence: 0,
        }
    }
    // ------------------------=
    // FUNC: state
    // DESC: Returns the authoritative segmentation state without consuming samples.
    // ------------------=
    pub const fn state(&self) -> VadState {
        self.state
    }
    // ------------------------=
    // FUNC: samples
    // DESC: Returns consumed mono samples including bounded preroll and trailing silence.
    // ------------------=
    pub const fn samples(&self) -> usize {
        self.samples
    }
    // ------------------------=
    // FUNC: segment
    // DESC: Exposes completed speech with its captured endpoint tail, preserving quiet final consonants.
    // ------------------=
    pub fn segment(&self) -> Option<Segment> {
        if self.state == VadState::Complete {
            Some(self.segment)
        } else {
            None
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Consumes any chunk size without allocations; terminal states consume no further audio.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        let mut consumed = 0;
        for &sample in pcm {
            if !matches!(self.state, VadState::Waiting | VadState::Speech) {
                break;
            }
            self.samples += 1;
            self.frame_samples += 1;
            consumed += 1;
            let value = i64::from(sample);
            self.energy += (value * value) as u64;
            if self.frame_samples == FRAME {
                let threshold = if self.state == VadState::Speech {
                    self.continuation_threshold
                } else { self.threshold };
                let voiced = self.energy >= u64::from(threshold.max(1)).pow(2) * FRAME as u64;
                if self.state == VadState::Waiting {
                    if voiced { self.waiting_frames = 0; }
                    else { self.observe_waiting_ambient(); }
                }
                if voiced {
                    self.consecutive += 1;
                    // Apply the same sustained-voice requirement at the tail
                    // as at onset. One noisy frame must not extend a phrase
                    // indefinitely or keep the retry gate latched.
                    if self.consecutive >= ONSET_FRAMES {
                        self.last_voice = self.samples;
                    }
                    if self.state == VadState::Waiting && self.consecutive >= ONSET_FRAMES {
                        self.segment.start =
                            self.samples.saturating_sub(ONSET_FRAMES * FRAME + PRE_ROLL);
                        self.state = VadState::Speech;
                    }
                } else {
                    self.consecutive = 0;
                }
                self.frame_samples = 0;
                self.energy = 0;
                if self.state == VadState::Speech
                    && self.consecutive == 0
                    && self.samples - self.last_voice >= END_SILENCE
                {
                    self.segment.end = self.samples;
                    self.state = VadState::Complete;
                }
            }
            if self.samples >= MAX_SAMPLES && self.state != VadState::Complete {
                if self.state == VadState::Speech {
                    self.segment = Segment { start: 0, end: 0 };
                    self.state = VadState::Limit;
                    self.frame_samples = 0;
                    self.energy = 0;
                    self.limit_silence = 0;
                } else {
                    self.state = VadState::NoSpeech;
                }
            }
        }
        consumed
    }
    // ------------------------=
    // FUNC: observe_waiting_ambient
    // DESC: Raises only the calibrated continuation floor from a short median of pre-speech ambient frames; never learns an active phrase.
    // ------------------=
    fn observe_waiting_ambient(&mut self) {
        if self.continuation_threshold >= self.threshold { return; }
        self.waiting_energies[self.waiting_frames] = (self.energy / FRAME as u64) as u32;
        self.waiting_frames += 1;
        if self.waiting_frames != AMBIENT_FRAMES { return; }
        self.waiting_frames = 0;
        let mut energies = self.waiting_energies;
        energies.sort_unstable();
        let median = u64::from(energies[AMBIENT_FRAMES / 2]);
        let mut low = 0u64;
        let mut high = 32769u64;
        while high - low > 1 {
            let middle = (low + high) / 2;
            if middle * middle <= median { low = middle; }
            else { high = middle; }
        }
        let raised = (low * 3 / 2).min(u64::from(self.threshold)) as u16;
        self.continuation_threshold = self.continuation_threshold.max(raised);
    }
    // ------------------------=
    // FUNC: drain_limit
    // DESC: Discards an overlong utterance until a pause below the new-turn gate, without retaining or submitting its truncated tail.
    // ------------------=
    fn drain_limit(&mut self, pcm: &[i16]) -> usize {
        let mut consumed = 0;
        for &sample in pcm {
            if self.state != VadState::Limit {
                break;
            }
            consumed += 1;
            self.frame_samples += 1;
            let value = i64::from(sample);
            self.energy += (value * value) as u64;
            if self.frame_samples == FRAME {
                // This utterance has already been rejected and erased. The
                // quieter continuation gate exists to preserve its soft words,
                // not to hold the next-turn gate shut on ordinary room noise.
                // Require the full endpoint pause below the calibrated onset
                // gate before allowing a fresh, separately wake-authorized turn.
                let voiced = self.energy >= u64::from(self.threshold.max(1)).pow(2) * FRAME as u64;
                self.consecutive = if voiced { self.consecutive + 1 } else { 0 };
                self.limit_silence = if self.consecutive >= ONSET_FRAMES {
                    0
                } else {
                    self.limit_silence + FRAME
                };
                self.frame_samples = 0;
                self.energy = 0;
                if self.consecutive == 0 && self.limit_silence >= END_SILENCE {
                    self.state = VadState::NoSpeech;
                }
            }
        }
        consumed
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Invalidates utterance bounds and removes accumulated energy on interruption.
    // ------------------=
    pub fn cancel(&mut self) {
        self.state = VadState::Cancelled;
        self.segment = Segment { start: 0, end: 0 };
        self.energy = 0;
        self.frame_samples = 0;
        self.consecutive = 0;
        self.last_voice = 0;
        self.waiting_energies.fill(0);
        self.waiting_frames = 0;
    }
}

pub struct Utterance {
    pcm: [i16; MAX_SAMPLES],
    detector: Detector,
}
impl Utterance {
    // ------------------------=
    // FUNC: new
    // DESC: Reserves a fixed 320,000-byte maximum utterance with no frame-time allocation.
    // ------------------=
    pub const fn new(threshold: u16) -> Self {
        Self {
            pcm: [0; MAX_SAMPLES],
            detector: Detector::new(threshold),
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Stores exactly the samples accepted by VAD, never overflowing utterance storage.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        if self.detector.state() == VadState::Limit {
            return self.detector.drain_limit(pcm);
        }
        let offset = self.detector.samples();
        let count = self.detector.push(pcm);
        self.pcm[offset..offset + count].copy_from_slice(&pcm[..count]);
        if self.detector.state() == VadState::Limit {
            self.pcm.fill(0);
            return count + self.detector.drain_limit(&pcm[count..]);
        }
        // Waiting time must not consume the speech budget. Preserve onset and
        // partial-frame history, but discard old silence before the next chunk.
        if self.detector.state == VadState::Waiting && self.detector.samples >= RATE {
            // Onset has not been confirmed yet: preserve the frames contributing
            // to that decision as well as the complete soft-consonant preroll.
            let keep = PRE_ROLL + ONSET_FRAMES * FRAME + self.detector.frame_samples;
            let end = self.detector.samples;
            self.pcm.copy_within(end - keep..end, 0);
            self.pcm[keep..end].fill(0);
            self.detector.samples = keep;
            self.detector.last_voice = self.detector.last_voice.saturating_sub(end - keep);
        }
        count
    }
    // ------------------------=
    // FUNC: state
    // DESC: Exposes speech onset, completion and bounded failures for service diagnostics.
    // ------------------=
    pub const fn state(&self) -> VadState {
        self.detector.state()
    }
    // ------------------------=
    // FUNC: active_speech_samples
    // DESC: Reports confirmed near-end speech duration so duplex playback ignores brief echo residuals before barge-in.
    // ------------------=
    pub const fn active_speech_samples(&self) -> usize {
        if matches!(self.detector.state(), VadState::Speech | VadState::Complete) {
            self.detector
                .samples()
                .saturating_sub(self.detector.segment.start)
        } else {
            0
        }
    }
    // ------------------------=
    // FUNC: speech
    // DESC: Borrows completed speech with its full endpoint tail; partial speech and failure never reach STT.
    // ------------------=
    pub fn speech(&self) -> Option<&[i16]> {
        self.detector.segment().map(|s| &self.pcm[s.start..s.end])
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases private microphone samples before reuse or cancellation.
    // ------------------=
    pub fn clear(&mut self, threshold: u16) {
        self.pcm.fill(0);
        self.detector = Detector::new(threshold);
    }
    // ------------------------=
    // FUNC: clear_with_continuation
    // DESC: Resets private PCM with calibrated onset and quieter continuation gates; unconfirmed noise cannot start a turn.
    // ------------------=
    pub fn clear_with_continuation(&mut self, threshold: u16, continuation: u16) {
        self.clear(threshold);
        self.detector.continuation_threshold = continuation.max(1).min(threshold.max(1));
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Stops segmentation and erases all retained audio, including preroll.
    // ------------------=
    pub fn cancel(&mut self) {
        self.pcm.fill(0);
        self.detector.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: waiting_rollover_preserves_soft_onset
    // DESC: Compares actual utterance PCM with the unrolled detector when a soft consonant precedes onset across the waiting-buffer boundary.
    // ------------------=
    fn waiting_rollover_preserves_soft_onset() {
        let mut input = std::vec![0; RATE * 76 / 100];
        input.extend_from_slice(&[120; PRE_ROLL]);
        input.extend_from_slice(&[1000; FRAME * 5]);
        input.extend_from_slice(&[0; RATE]);
        let mut reference = Detector::new(300);
        reference.push(&input);
        let bounds = reference.segment().unwrap();
        let expected = &input[bounds.start..bounds.end];
        assert_eq!(expected.iter().filter(|&&v| v == 120).count(), PRE_ROLL);
        for chunk in [1, 17, 319, 320, 701, 4096] {
            let mut utterance = Utterance::new(300);
            for samples in input.chunks(chunk) {
                utterance.push(samples);
            }
            assert_eq!(utterance.speech().unwrap(), expected, "chunk size {chunk}");
        }
    }
    #[test]
    // ------------------------=
    // FUNC: erases_retained_pcm
    // DESC: Inspects private storage after cancellation and reset, including samples outside speech bounds.
    // ------------------=
    fn erases_retained_pcm() {
        let mut value = Utterance::new(300);
        value.push(&[1234; FRAME * 4]);
        assert!(value.pcm.iter().any(|s| *s != 0));
        value.cancel();
        assert!(value.pcm.iter().all(|s| *s == 0));
        value.clear(300);
        value.push(&[5678; FRAME]);
        value.clear(300);
        assert!(value.pcm.iter().all(|s| *s == 0));
        assert_eq!(value.detector.samples(), 0);
    }
    #[test]
    // ------------------------=
    // FUNC: reports_only_confirmed_speech_duration
    // DESC: Distinguishes short onset residuals from sustained near-end speech used for duplex interruption.
    // ------------------=
    fn reports_only_confirmed_speech_duration() {
        let mut value = Utterance::new(300);
        value.push(&[1000; FRAME * 2]);
        assert_eq!(value.active_speech_samples(), 0);
        value.push(&[1000; FRAME * 8]);
        assert!(value.active_speech_samples() >= RATE / 5);
        value.clear(300);
        assert_eq!(value.active_speech_samples(), 0);
    }

    #[test]
    // ------------------------=
    // FUNC: completes_after_conversational_end_pause
    // DESC: Proves a finished utterance advances after 800 ms of silence without treating a shorter thinking pause as completion.
    // ------------------=
    fn completes_after_conversational_end_pause() {
        let mut value = Utterance::new(300);
        value.push(&[1000; FRAME * 5]);
        value.push(&[0; END_SILENCE - FRAME]);
        assert_eq!(value.state(), VadState::Speech);
        value.push(&[0; FRAME]);
        assert_eq!(value.state(), VadState::Complete);
        assert!(value.speech().is_some());
    }

    // ------------------------=
    // FUNC: short_pause_retains_entire_command
    // DESC: Preserves both spoken portions across normal half-second hesitations for arbitrary driver chunk boundaries.
    // ------------------=
    #[test]
    fn short_pause_retains_entire_command() {
        for pause in [RATE / 2, RATE * 7 / 10] {
            let mut input = std::vec![0; PRE_ROLL];
            input.extend_from_slice(&[1000; RATE]);
            input.extend(std::iter::repeat_n(0, pause));
            input.extend_from_slice(&[1300; RATE / 2]);
            input.extend_from_slice(&[0; END_SILENCE]);
            for chunk in [1, 17, 319, 320, 701, 4096] {
                let mut value = Utterance::new(300);
                for block in input.chunks(chunk) { assert_eq!(value.push(block), block.len()); }
                assert_eq!(value.state(), VadState::Complete);
                assert_eq!(value.speech().unwrap(), input);
            }
        }
    }

    // ------------------------=
    // FUNC: calibrated_gate_retains_quiet_long_continuation
    // DESC: Keeps a quieter 1.5-second tail after a thinking pause without permitting that level to start its own utterance.
    // ------------------=
    #[test]
    fn calibrated_gate_retains_quiet_long_continuation() {
        let mut input = std::vec![100; PRE_ROLL];
        input.extend_from_slice(&[1000; RATE / 5]);
        input.extend_from_slice(&[100; RATE / 2]);
        input.extend_from_slice(&[220; RATE * 3 / 2]);
        input.extend_from_slice(&[100; END_SILENCE]);
        for chunk in [1, 17, 319, 320, 701, 4096] {
            let mut value = Utterance::new(300);
            value.clear_with_continuation(300, 150);
            for block in input.chunks(chunk) { assert_eq!(value.push(block), block.len()); }
            assert_eq!(value.state(), VadState::Complete);
            assert_eq!(value.speech().unwrap(), input);
            value.clear_with_continuation(300, 150);
            value.push(&[220; RATE * 2]);
            assert_eq!(value.state(), VadState::Waiting);
            assert_eq!(value.speech(), None);
        }
    }

    // ------------------------=
    // FUNC: endpoint_tail_preserves_soft_final_phonemes
    // DESC: Retains captured below-gate trailing audio instead of clipping everything after the first 100 milliseconds.
    // ------------------=
    #[test]
    fn endpoint_tail_preserves_soft_final_phonemes() {
        let mut input = std::vec![1000; FRAME * 5];
        input.extend_from_slice(&[120; RATE / 2]);
        input.extend_from_slice(&[0; END_SILENCE]);
        for chunk in [1, 17, 319, 320, 701, 4096] {
            let mut value = Utterance::new(300);
            value.clear_with_continuation(300, 150);
            for block in input.chunks(chunk) { value.push(block); }
            assert_eq!(value.state(), VadState::Complete);
            assert_eq!(value.speech().unwrap(), &input[..FRAME * 5 + END_SILENCE]);
            assert_eq!(value.speech().unwrap().iter().filter(|&&sample| sample == 120).count(), RATE / 2);
        }
    }

    // ------------------------=
    // FUNC: calibrated_noise_and_quiet_overflow_fail_closed
    // DESC: Rejects measured ambient and isolated clicks while keeping the ten-second privacy bound for quiet continuous speech.
    // ------------------=
    #[test]
    fn calibrated_noise_and_quiet_overflow_fail_closed() {
        for overlong in [false, true] {
            let mut value = Utterance::new(800);
            value.clear_with_continuation(800, 600);
            value.push(&[1600; FRAME * ONSET_FRAMES]);
            if overlong {
                value.push(&[700; MAX_SAMPLES]);
                assert_eq!(value.state(), VadState::Limit);
                assert!(value.speech().is_none());
                assert!(value.pcm.iter().all(|&sample| sample == 0));
            }
            for frame in 0..END_SILENCE / FRAME {
                value.push(&[if frame % 5 == 2 { 900 } else { 400 }; FRAME]);
            }
            assert_eq!(value.state(), if overlong { VadState::NoSpeech } else { VadState::Complete });
            assert_eq!(value.speech().is_none(), overlong);
        }
    }

    // ------------------------=
    // FUNC: waiting_gain_rise_does_not_hold_the_endpoint_open
    // DESC: Reproduces calibrated RMS217 rising to ambient380 before speech, then proves bounded release and reset to the measured floor.
    // ------------------=
    #[test]
    fn waiting_gain_rise_does_not_hold_the_endpoint_open() {
        for chunk in [1, 17, 319, 320, 701, 4096] {
            let mut value = Utterance::new(434);
            value.clear_with_continuation(434, 325);
            let mut input = std::vec![380; PRE_ROLL];
            input.extend_from_slice(&[1200; RATE / 5]);
            input.extend_from_slice(&[380; END_SILENCE]);
            for block in input.chunks(chunk) { assert_eq!(value.push(block), block.len()); }
            assert_eq!(value.state(), VadState::Complete);
            assert_eq!(value.speech().unwrap(), input);
            assert_eq!(value.detector.continuation_threshold, 434);
            value.clear_with_continuation(434, 325);
            assert_eq!(value.detector.continuation_threshold, 325);
            assert_eq!(value.detector.waiting_frames, 0);
            assert!(value.detector.waiting_energies.iter().all(|&energy| energy == 0));
        }
    }

    // ------------------------=
    // FUNC: waiting_ambient_median_rejects_isolated_blips
    // DESC: Leaves the calibrated quiet-tail gate unchanged after an isolated sub-onset noise transient.
    // ------------------=
    #[test]
    fn waiting_ambient_median_rejects_isolated_blips() {
        let mut value = Utterance::new(434);
        value.clear_with_continuation(434, 325);
        value.push(&[430; FRAME]);
        value.push(&[217; FRAME * (AMBIENT_FRAMES - 1)]);
        assert_eq!(value.state(), VadState::Waiting);
        assert_eq!(value.detector.continuation_threshold, 325);
        value.push(&[1200; FRAME * ONSET_FRAMES]);
        value.push(&[350; RATE]);
        assert_eq!(value.state(), VadState::Speech);
        assert_eq!(value.detector.continuation_threshold, 325);
    }

    #[test]
    // ------------------------=
    // FUNC: overlong_speech_is_rejected_until_real_pause
    // DESC: Ensures continuous speech beyond the bounded buffer is never truncated into a command, retains no PCM, and rearms only after silence.
    // ------------------=
    fn overlong_speech_is_rejected_until_real_pause() {
        let mut value = Utterance::new(300);
        for _ in 0..MAX_SAMPLES / FRAME {
            value.push(&[1000; FRAME]);
        }
        assert_eq!(value.state(), VadState::Limit);
        assert!(value.speech().is_none());
        assert!(value.pcm.iter().all(|&sample| sample == 0));
        for _ in 0..1000 {
            value.push(&[1000; FRAME]);
        }
        assert_eq!(value.state(), VadState::Limit);
        value.push(&[0; END_SILENCE - FRAME]);
        assert_eq!(value.state(), VadState::Limit);
        value.push(&[1000; FRAME * ONSET_FRAMES]);
        value.push(&[0; END_SILENCE - FRAME]);
        assert_eq!(value.state(), VadState::Limit);
        value.push(&[0; FRAME]);
        assert_eq!(value.state(), VadState::NoSpeech);
        assert!(value.speech().is_none());
        value.clear(300);
        value.push(&[1000; FRAME * 5]);
        value.push(&[0; END_SILENCE]);
        assert_eq!(value.state(), VadState::Complete);
    }

    // ------------------------=
    // FUNC: rejected_turn_rearms_at_onset_floor_without_executing_its_tail
    // DESC: Bounds retry to the measured 800-ms pause when room noise lies between continuation and onset gates; new speech still requires a fresh segment.
    // ------------------=
    #[test]
    fn rejected_turn_rearms_at_onset_floor_without_executing_its_tail() {
        for chunk in [1, 17, 320, 701, 4096] {
            let mut value = Utterance::new(310);
            value.clear_with_continuation(310, 232);
            value.push(&[1200; MAX_SAMPLES]);
            assert_eq!(value.state(), VadState::Limit);
            let mut consumed = 0;
            for pcm in [260; END_SILENCE - 1].chunks(chunk) {
                consumed += value.push(pcm);
                assert_eq!(value.state(), VadState::Limit);
                assert!(value.speech().is_none());
            }
            consumed += value.push(&[260]);
            assert_eq!(consumed, END_SILENCE);
            assert_eq!(consumed as u64 * 1_000_000_000 / RATE as u64, 800_000_000);
            assert_eq!(value.state(), VadState::NoSpeech);
            assert!(value.speech().is_none());
            assert!(value.pcm.iter().all(|&sample| sample == 0));
            value.clear_with_continuation(310, 232);
            value.push(&[260; RATE]);
            assert_eq!(value.state(), VadState::Waiting);
            value.push(&[1200; FRAME * ONSET_FRAMES]);
            value.push(&[260; END_SILENCE]);
            assert_eq!(value.state(), VadState::Complete);
        }
    }

    // ------------------------=
    // FUNC: intermittent_background_does_not_latch_speech_or_retry
    // DESC: Exercises measured low-level background with isolated threshold crossings after speech and after an overlong segment.
    // ------------------=
    #[test]
    fn intermittent_background_does_not_latch_speech_or_retry() {
        for overlong in [false, true] {
            let mut value = Utterance::new(434);
            for _ in 0..if overlong { MAX_SAMPLES / FRAME } else { 10 } {
                value.push(&[1200; FRAME]);
            }
            for frame in 0..END_SILENCE / FRAME {
                value.push(&[if frame % 5 == 2 { 570 } else { 380 }; FRAME]);
            }
            assert_eq!(
                value.state(),
                if overlong {
                    VadState::NoSpeech
                } else {
                    VadState::Complete
                }
            );
            assert_eq!(value.speech().is_none(), overlong);
        }
    }
}
