//! Allocation-free 16 kHz mono utterance segmentation. Energy VAD is not STT.
pub const RATE: usize = 16_000;
pub const FRAME: usize = 320;
pub const MAX_SAMPLES: usize = RATE * 10;
const PRE_ROLL: usize = RATE / 5;
const TAIL: usize = RATE / 10;
const END_SILENCE: usize = RATE * 3 / 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadState { Waiting, Speech, Complete, NoSpeech, Limit, Cancelled }
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
}
impl Detector {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed 20-ms RMS gate with three-frame onset and bounded silence timeout.
    // ------------------=
    pub const fn new(threshold: u16) -> Self {
        Self { state: VadState::Waiting, samples: 0, frame_samples: 0, energy: 0,
            consecutive: 0, last_voice: 0, segment: Segment { start: 0, end: 0 }, threshold }
    }
    // ------------------------=
    // FUNC: state
    // DESC: Returns the authoritative segmentation state without consuming samples.
    // ------------------=
    pub const fn state(&self) -> VadState { self.state }
    // ------------------------=
    // FUNC: samples
    // DESC: Returns consumed mono samples including bounded preroll and trailing silence.
    // ------------------=
    pub const fn samples(&self) -> usize { self.samples }
    // ------------------------=
    // FUNC: segment
    // DESC: Exposes only completed speech bounds, excluding most trailing silence.
    // ------------------=
    pub fn segment(&self) -> Option<Segment> {
        if self.state == VadState::Complete { Some(self.segment) } else { None }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Consumes any chunk size without allocations; terminal states consume no further audio.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        let mut consumed = 0;
        for &sample in pcm {
            if !matches!(self.state, VadState::Waiting | VadState::Speech) { break; }
            self.samples += 1; self.frame_samples += 1; consumed += 1;
            let value = i64::from(sample);
            self.energy += (value * value) as u64;
            if self.frame_samples == FRAME {
                let voiced = self.energy >= u64::from(self.threshold.max(1)).pow(2) * FRAME as u64;
                if voiced {
                    self.consecutive += 1;
                    self.last_voice = self.samples;
                    if self.state == VadState::Waiting && self.consecutive >= 3 {
                        self.segment.start = self.samples.saturating_sub(3 * FRAME + PRE_ROLL);
                        self.state = VadState::Speech;
                    }
                } else { self.consecutive = 0; }
                self.frame_samples = 0; self.energy = 0;
                if self.state == VadState::Speech && self.samples - self.last_voice >= END_SILENCE {
                    self.segment.end = (self.last_voice + TAIL).min(self.samples);
                    self.state = VadState::Complete;
                }
            }
            if self.samples >= MAX_SAMPLES && self.state != VadState::Complete {
                if self.state == VadState::Speech {
                    self.segment.end = self.samples;
                    self.state = VadState::Complete;
                } else {
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
        self.state = VadState::Cancelled; self.segment = Segment { start: 0, end: 0 };
        self.energy = 0; self.frame_samples = 0; self.consecutive = 0; self.last_voice = 0;
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
    pub const fn new(threshold: u16) -> Self { Self { pcm: [0; MAX_SAMPLES], detector: Detector::new(threshold) } }
    // ------------------------=
    // FUNC: push
    // DESC: Stores exactly the samples accepted by VAD, never overflowing utterance storage.
    // ------------------=
    pub fn push(&mut self, pcm: &[i16]) -> usize {
        let offset = self.detector.samples();
        let count = self.detector.push(pcm);
        self.pcm[offset..offset + count].copy_from_slice(&pcm[..count]);
        // Waiting time must not consume the speech budget. Preserve onset and
        // partial-frame history, but discard old silence before the next chunk.
        if self.detector.state == VadState::Waiting && self.detector.samples >= RATE {
            let keep = PRE_ROLL + self.detector.frame_samples;
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
    pub const fn state(&self) -> VadState { self.detector.state() }
    // ------------------------=
    // FUNC: speech
    // DESC: Borrows a trimmed completed utterance; partial speech and failure never reach STT.
    // ------------------=
    pub fn speech(&self) -> Option<&[i16]> {
        self.detector.segment().map(|s| &self.pcm[s.start..s.end])
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases private microphone samples before reuse or cancellation.
    // ------------------=
    pub fn clear(&mut self, threshold: u16) {
        self.pcm.fill(0); self.detector = Detector::new(threshold);
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Stops segmentation and erases all retained audio, including preroll.
    // ------------------=
    pub fn cancel(&mut self) { self.pcm.fill(0); self.detector.cancel(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: erases_retained_pcm
    // DESC: Inspects private storage after cancellation and reset, including samples outside speech bounds.
    // ------------------=
    fn erases_retained_pcm() {
        let mut value = Utterance::new(300);
        value.push(&[1234; FRAME * 4]);
        assert!(value.pcm.iter().any(|s| *s != 0));
        value.cancel(); assert!(value.pcm.iter().all(|s| *s == 0));
        value.clear(300); value.push(&[5678; FRAME]);
        value.clear(300); assert!(value.pcm.iter().all(|s| *s == 0));
        assert_eq!(value.detector.samples(), 0);
    }
}
