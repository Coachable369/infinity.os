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
        if h.deadline <= now || h.deadline - now > 5 { return Err(AudioError::Expired); }
        let route = if h.operation_type_id == OperationId::AudioTone as u32 { AudioRoute::Playback }
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
}
