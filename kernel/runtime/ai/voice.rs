use super::types::AiError;
use super::voice_vad::{Detector, Segment, VadState, RATE};
use crate::runtime::{
    capability::{CapabilityId, CapabilityManager, CapabilityType},
    execution::SecurityIdentity,
};

pub const RECOGNITION_DEADLINE_SECONDS: u64 = 45;

// ------------------------=
// FUNC: authorize_recognition
// DESC: Validates the typed speech operation and its bounded PCM contract before a provider receives audio.
// ------------------=
pub fn authorize_recognition(
    message: &crate::runtime::iop::IopMessage,
    owner: SecurityIdentity,
    samples: usize,
    capabilities: &CapabilityManager,
    now: u64,
) -> Result<(), AiError> {
    use crate::runtime::iop::{MessageType, OperationId, IOP_VERSION};
    let h = &message.header;
    if h.protocol_version != IOP_VERSION || h.schema_version != 1
        || h.message_type != MessageType::Request
        || h.operation_type_id != OperationId::SpeechRecognize as u32
        || h.caller_identity != owner || h.payload_length != 0
        || samples == 0 || samples > 160000
        || h.deadline <= now
        || h.deadline.saturating_sub(now) > RECOGNITION_DEADLINE_SECONDS
    {
        return Err(AiError::InvalidRequest);
    }
    capabilities.validate(h.capability_ref, owner, CapabilityType::AudioInput, 0, 1, 0, now)
        .map_err(|_| AiError::AccessDenied)?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceState {
    Idle,
    Listening,
    Recognizing,
    Speaking,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoiceSession {
    pub id: u32,
    pub owner: SecurityIdentity,
    pub state: VoiceState,
    pub microphone_capability: CapabilityId,
    pub expires_at: u64,
}

pub struct VoiceService {
    session: Option<VoiceSession>,
    next_id: u32,
    detector: Detector,
}

impl VoiceService {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an explicit-activation voice service with listening disabled.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            session: None,
            next_id: 1,
            detector: Detector::new(300),
        }
    }

    // ------------------------=
    // FUNC: start_push_to_talk
    // DESC: Starts a leased session only after validating microphone authority.
    // ------------------=
    pub fn start_push_to_talk(
        &mut self,
        owner: SecurityIdentity,
        capability: CapabilityId,
        expires_at: u64,
        now: u64,
        capabilities: &CapabilityManager,
    ) -> Result<u32, AiError> {
        capabilities
            .validate(capability, owner, CapabilityType::AudioInput, 0, 1, 0, now)
            .map_err(|_| AiError::AccessDenied)?;
        if expires_at <= now || expires_at.saturating_sub(now) > 10 {
            return Err(AiError::InvalidRequest);
        }
        self.expire(now);
        if self.session.is_some() { return Err(AiError::QueueFull); }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.session = Some(VoiceSession {
            id,
            owner,
            state: VoiceState::Listening,
            microphone_capability: capability,
            expires_at,
        });
        self.detector = Detector::new(300);
        Ok(id)
    }

    // ------------------------=
    // FUNC: push_pcm
    // DESC: Segments bounded native 16-kHz mono chunks only for the current authorized session owner.
    // ------------------=
    pub fn push_pcm(&mut self, id: u32, owner: SecurityIdentity, pcm: &[i16], rate: u32,
        now: u64, capabilities: &CapabilityManager) -> Result<VadState, AiError> {
        let session = self.session.ok_or(AiError::InvalidRequest)?;
        if session.id != id { return Err(AiError::InvalidRequest); }
        if session.owner != owner { return Err(AiError::AccessDenied); }
        if session.state != VoiceState::Listening || rate != RATE as u32 || pcm.len() > RATE / 10 {
            return Err(AiError::InvalidRequest);
        }
        if !self.refresh_authority(now, capabilities) { return Err(AiError::AccessDenied); }
        self.detector.push(pcm);
        let state = self.detector.state();
        if let Some(session) = self.session.as_mut() {
            if state == VadState::Complete { session.state = VoiceState::Recognizing; }
            else if matches!(state, VadState::Limit | VadState::NoSpeech) { session.state = VoiceState::Failed; }
        }
        Ok(state)
    }

    // ------------------------=
    // FUNC: speech_segment
    // DESC: Returns trimmed sample bounds only to the owning session, never a fabricated transcript.
    // ------------------=
    pub fn speech_segment(&self, id: u32, owner: SecurityIdentity) -> Result<Option<Segment>, AiError> {
        let session = self.session.ok_or(AiError::InvalidRequest)?;
        if session.id != id { return Err(AiError::InvalidRequest); }
        if session.owner != owner { return Err(AiError::AccessDenied); }
        Ok(self.detector.segment())
    }

    // ------------------------=
    // FUNC: stop
    // DESC: Ends listening immediately and removes the microphone lease from the session.
    // ------------------=
    pub fn stop(&mut self, id: u32, owner: SecurityIdentity) -> Result<(), AiError> {
        let session = self.session.ok_or(AiError::InvalidRequest)?;
        if session.id != id { return Err(AiError::InvalidRequest); }
        if session.owner != owner { return Err(AiError::AccessDenied); }
        self.session = None;
        self.detector.cancel();
        Ok(())
    }

    // ------------------------=
    // FUNC: state
    // DESC: Exposes observable microphone and voice-session state.
    // ------------------=
    pub const fn state(&self) -> VoiceState {
        match self.session {
            Some(session) => session.state,
            None => VoiceState::Idle,
        }
    }

    // ------------------------=
    // FUNC: expire
    // DESC: Ends an expired voice lease without relying on caller cleanup.
    // ------------------=
    pub fn expire(&mut self, now: u64) {
        if self.session.map(|s| now >= s.expires_at).unwrap_or(false) {
            self.session = None;
            self.detector.cancel();
        }
    }

    // ------------------------=
    // FUNC: refresh_authority
    // DESC: Revalidates microphone authority during capture so revocation takes effect immediately.
    // ------------------=
    pub fn refresh_authority(&mut self, now: u64, capabilities: &CapabilityManager) -> bool {
        let Some(session) = self.session else {
            return false;
        };
        if now >= session.expires_at
            || capabilities
                .validate(
                    session.microphone_capability,
                    session.owner,
                    CapabilityType::AudioInput,
                    0,
                    1,
                    0,
                    now,
                )
                .is_err()
        {
            self.session = None;
            self.detector.cancel();
            return false;
        }
        true
    }
}

pub trait SpeechRecognitionProvider {
    // ------------------------=
    // FUNC: recognize_pcm
    // DESC: Recognizes injected or captured PCM through a provider-neutral typed interface.
    // ------------------=
    fn recognize_pcm(
        &mut self,
        pcm: &[i16],
        sample_rate: u32,
        out: &mut [u8],
    ) -> Result<usize, AiError>;
}

pub struct UnavailableLocalSpeechProvider;

impl SpeechRecognitionProvider for UnavailableLocalSpeechProvider {
    // ------------------------=
    // FUNC: recognize_pcm
    // DESC: Reports the current absence of a verified offline speech model without fabricating a transcript.
    // ------------------=
    fn recognize_pcm(
        &mut self,
        _pcm: &[i16],
        _sample_rate: u32,
        _out: &mut [u8],
    ) -> Result<usize, AiError> {
        Err(AiError::ProviderUnavailable)
    }
}

pub trait SpeechSynthesisProvider {
    // ------------------------=
    // FUNC: synthesize
    // DESC: Produces provider-neutral PCM for system and accessibility speech.
    // ------------------=
    fn synthesize(&mut self, text: &[u8], out: &mut [i16]) -> Result<usize, AiError>;
}
