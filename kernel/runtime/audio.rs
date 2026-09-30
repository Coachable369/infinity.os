//! Native audio service contract. PCM buffers never contain caller-supplied DMA addresses.
use super::capability::{CapabilityManager, CapabilityType};
use super::execution::SecurityIdentity;
use super::iop::{IopMessage, MessageType, OperationId, IOP_VERSION};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioFormat { pub sample_rate: u32, pub channels: u8, pub bits: u8 }
pub const PCM: AudioFormat = AudioFormat { sample_rate: 48000, channels: 2, bits: 16 };
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioRoute { Playback, Capture }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioError { Invalid, Denied, Expired }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureState { Idle, Recording, Complete, Cancelled, Denied, DeviceLost, Overrun }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState { Idle, Playing, Complete, Cancelled, Denied, DeviceLost, Underrun }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSpan {
    pub start_frame: u64,
    pub end_frame: u64,
    pub content_start: usize,
    pub content_end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InfinityAudioStatus {
    pub generation: u64,
    pub queued_frames: u64,
    pub played_frames: u64,
    pub content_position: usize,
    pub content_boundary: bool,
    pub sealed: bool,
    pub underruns: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureStatus {
    pub state: CaptureState,
    pub sample_rate: u32,
    pub frames: u64,
    pub peak: u16,
}
pub struct AudioBuffer<const N: usize> {
    samples: [i16; N],
    read: usize,
    length: usize,
}
pub struct InfinityAudio<const N: usize, const S: usize> {
    samples: [i16; N],
    read: usize,
    length: usize,
    generation: u64,
    total_frames: u64,
    spans: [AudioSpan; S],
    span_count: usize,
    sealed: bool,
    underruns: u64,
}
impl<const N: usize, const S: usize> InfinityAudio<N, S> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed-capacity stereo stream queue and content timeline without heap allocation.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            samples: [0; N], read: 0, length: 0, generation: 0, total_frames: 0,
            spans: [AudioSpan { start_frame: 0, end_frame: 0, content_start: 0, content_end: 0 }; S],
            span_count: 0, sealed: false, underruns: 0,
        }
    }
    // ------------------------=
    // FUNC: reset
    // DESC: Erases queued PCM and starts a new generation so stale producers cannot publish into a replacement stream.
    // ------------------=
    pub fn reset(&mut self, generation: u64) {
        self.samples.fill(0); self.read = 0; self.length = 0; self.generation = generation;
        self.total_frames = 0; self.spans.fill(AudioSpan { start_frame: 0, end_frame: 0, content_start: 0, content_end: 0 });
        self.span_count = 0; self.sealed = false; self.underruns = 0;
    }
    // ------------------------=
    // FUNC: can_append
    // DESC: Checks generation, stereo capacity and monotonic content metadata without mutating queued audio.
    // ------------------=
    pub fn can_append(&self, generation: u64, sample_count: usize, content_start: usize, content_end: usize) -> bool {
        if generation != self.generation || self.sealed || N % 2 != 0 || sample_count == 0 || sample_count % 2 != 0
            || sample_count > N.saturating_sub(self.length) || self.span_count == S || content_end < content_start {
            return false;
        }
        if self.span_count != 0 && content_end != 0 {
            let previous = self.spans[self.span_count - 1];
            if previous.content_end != 0 && content_start < previous.content_end { return false; }
        }
        true
    }
    // ------------------------=
    // FUNC: append
    // DESC: Queues complete stereo frames and records their optional content range on one monotonic stream timeline.
    // ------------------=
    pub fn append(&mut self, generation: u64, samples: &[i16], content_start: usize, content_end: usize) -> Result<AudioSpan, AudioError> {
        if !self.can_append(generation, samples.len(), content_start, content_end) { return Err(AudioError::Invalid); }
        let write = (self.read + self.length) % N;
        let first = samples.len().min(N - write);
        self.samples[write..write + first].copy_from_slice(&samples[..first]);
        self.samples[..samples.len() - first].copy_from_slice(&samples[first..]);
        self.length += samples.len();
        let frames = (samples.len() / 2) as u64;
        let span = AudioSpan { start_frame: self.total_frames, end_frame: self.total_frames + frames, content_start, content_end };
        self.spans[self.span_count] = span; self.span_count += 1; self.total_frames += frames;
        Ok(span)
    }
    // ------------------------=
    // FUNC: seal
    // DESC: Marks the current generation complete so the consumer can distinguish completion from an underrun.
    // ------------------=
    pub fn seal(&mut self, generation: u64) -> Result<(), AudioError> {
        if generation != self.generation || self.total_frames == 0 { return Err(AudioError::Invalid); }
        self.sealed = true; Ok(())
    }
    // ------------------------=
    // FUNC: read
    // DESC: Drains queued stereo samples in order and pads only the final sealed hardware period with silence.
    // ------------------=
    pub fn read(&mut self, generation: u64, output: &mut [i16]) -> Result<usize, AudioError> {
        if generation != self.generation || output.len() % 2 != 0 { return Err(AudioError::Invalid); }
        let count = output.len().min(self.length) & !1;
        if count != 0 {
            let first = count.min(N - self.read);
            output[..first].copy_from_slice(&self.samples[self.read..self.read + first]);
            output[first..count].copy_from_slice(&self.samples[..count - first]);
            self.read = (self.read + count) % N;
        }
        self.length -= count;
        output[count..].fill(0);
        if count < output.len() && !self.sealed { self.underruns = self.underruns.saturating_add(1); }
        Ok(count)
    }
    // ------------------------=
    // FUNC: status
    // DESC: Maps an authoritative hardware frame cursor onto queued content without exposing PCM storage.
    // ------------------=
    pub fn status(&self, played_frames: u64) -> InfinityAudioStatus {
        let played = played_frames.min(self.total_frames);
        let mut content = 0;
        let mut boundary = false;
        for span in &self.spans[..self.span_count] {
            if played >= span.end_frame { content = content.max(span.content_end); boundary = played == span.end_frame; continue; }
            if played <= span.start_frame { break; }
            let frames = span.end_frame - span.start_frame;
            let units = span.content_end.saturating_sub(span.content_start);
            content = span.content_start.saturating_add(units.saturating_mul((played - span.start_frame) as usize) / frames.max(1) as usize);
            break;
        }
        InfinityAudioStatus { generation: self.generation, queued_frames: self.total_frames, played_frames: played,
            content_position: content, content_boundary: boundary, sealed: self.sealed, underruns: self.underruns }
    }
    // ------------------------=
    // FUNC: generation
    // DESC: Returns the active stream generation for stale-producer rejection.
    // ------------------=
    pub fn generation(&self) -> u64 { self.generation }
    // ------------------------=
    // FUNC: remaining_samples
    // DESC: Reports prepared PCM still waiting to enter hardware periods.
    // ------------------=
    pub fn remaining_samples(&self) -> usize { self.length }
    // ------------------------=
    // FUNC: erase_pcm
    // DESC: Erases private queued sound after completion or cancellation while retaining non-audio timeline diagnostics.
    // ------------------=
    pub fn erase_pcm(&mut self) { self.samples.fill(0); self.read = 0; self.length = 0; }
    // ------------------------=
    // FUNC: reference_mono
    // DESC: Produces a bounded mono echo reference from samples the hardware timeline has actually reached.
    // ------------------=
    pub fn reference_mono(&self, played_frames: u64, stream_rate: u32, output_rate: u32, output: &mut [i16]) -> bool {
        if stream_rate == 0 || output_rate == 0 || self.total_frames == 0 || N < 2 || N % 2 != 0 {
            output.fill(0); return false;
        }
        output.fill(0);
        let end = played_frames.min(self.total_frames) as usize;
        let retained_start = self.total_frames.saturating_sub((N / 2) as u64) as usize;
        let available = end.saturating_mul(output_rate as usize) / stream_rate as usize;
        let count = available.min(output.len());
        let mut copied = false;
        for offset in 0..count {
            let output_at = output.len() - count + offset;
            let source_frame = end.saturating_sub(count.saturating_sub(offset).saturating_mul(stream_rate as usize) / output_rate as usize);
            if source_frame >= retained_start && source_frame < self.total_frames as usize {
                let at = source_frame.saturating_mul(2) % N;
                output[output_at] = ((self.samples[at] as i32 + self.samples[at + 1] as i32) / 2) as i16;
                copied = true;
            }
        }
        copied
    }
}
impl<const N: usize> AudioBuffer<N> {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates fixed stream storage with no per-poll allocation.
    // ------------------=
    pub const fn new() -> Self { Self { samples: [0; N], read: 0, length: 0 } }
    // ------------------------=
    // FUNC: push_stereo
    // DESC: Atomically appends complete stereo frames as mono; rejects overflow instead of silently overwriting speech.
    // ------------------=
    pub fn push_stereo(&mut self, samples: &[i16]) -> Result<(), AudioError> {
        if samples.len() % 2 != 0 || samples.len() / 2 > N - self.length { return Err(AudioError::Invalid); }
        for pair in samples.chunks_exact(2) {
            self.samples[(self.read + self.length) % N] = ((pair[0] as i32 + pair[1] as i32) / 2) as i16;
            self.length += 1;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: read
    // DESC: Transfers captured mono samples in order and zeroes consumed private audio.
    // ------------------=
    pub fn read(&mut self, output: &mut [i16]) -> usize {
        let count = output.len().min(self.length);
        for sample in &mut output[..count] {
            *sample = self.samples[self.read]; self.samples[self.read] = 0;
            self.read = (self.read + 1) % N;
        }
        self.length -= count; count
    }
    // ------------------------=
    // FUNC: clear
    // DESC: Erases buffered microphone data on stop, revocation or replacement.
    // ------------------=
    pub fn clear(&mut self) { self.samples.fill(0); self.read = 0; self.length = 0; }
}
#[derive(Clone, Copy)]
pub struct AudioStream {
    pub owner: SecurityIdentity,
    pub capability: u64,
    pub route: AudioRoute,
    pub deadline: u64,
}
impl AudioStream {
    // ------------------------=
    // FUNC: authorize
    // DESC: Decodes a bounded IOP audio request and checks direction-specific authority before touching hardware.
    // ------------------=
    pub fn authorize(message: &IopMessage, caller: SecurityIdentity, caps: &CapabilityManager, now: u64) -> Result<Self, AudioError> {
        let h = &message.header;
        if h.protocol_version != IOP_VERSION || h.schema_version != 1 || h.message_type != MessageType::Request
            || h.caller_identity != caller || h.payload_length != 0 { return Err(AudioError::Invalid); }
        let maximum = if h.operation_type_id == OperationId::AudioPlaybackStart as u32 { 35 } else { 5 };
        if h.deadline <= now || h.deadline - now > maximum { return Err(AudioError::Expired); }
        let route = if h.operation_type_id == OperationId::AudioTone as u32 || h.operation_type_id == OperationId::AudioPlaybackStart as u32 { AudioRoute::Playback }
            else if h.operation_type_id == OperationId::AudioCaptureStart as u32 { AudioRoute::Capture }
            else { return Err(AudioError::Invalid); };
        let stream = Self { owner: caller, capability: h.capability_ref, route, deadline: h.deadline };
        if !stream.valid(caps, now) { return Err(AudioError::Denied); }
        Ok(stream)
    }
    // ------------------------=
    // FUNC: valid
    // DESC: Revalidates revocation and expiration during DMA playback or capture.
    // ------------------=
    pub fn valid(&self, caps: &CapabilityManager, now: u64) -> bool {
        let kind = match self.route { AudioRoute::Playback => CapabilityType::AudioOutput, AudioRoute::Capture => CapabilityType::AudioInput };
        now < self.deadline && caps.validate(self.capability, self.owner, kind, 0, 1, 0, now).is_ok()
    }
    // ------------------------=
    // FUNC: renew
    // DESC: Reauthorizes an unexpired stream without changing owner or direction or resurrecting revoked authority.
    // ------------------=
    pub fn renew(&self, message: &IopMessage, caps: &CapabilityManager, now: u64) -> Result<Self, AudioError> {
        if !self.valid(caps, now) { return Err(AudioError::Denied); }
        let next = Self::authorize(message, self.owner, caps, now)?;
        if next.route != self.route { return Err(AudioError::Denied); }
        Ok(next)
    }
}
