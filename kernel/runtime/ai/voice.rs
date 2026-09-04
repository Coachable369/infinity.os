use super::types::AiError;
use crate::runtime::{
    capability::{CapabilityId, CapabilityManager, CapabilityType},
    execution::SecurityIdentity,
};

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
        if expires_at <= now {
            return Err(AiError::InvalidRequest);
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.session = Some(VoiceSession {
            id,
            owner,
            state: VoiceState::Listening,
            microphone_capability: capability,
            expires_at,
        });
        Ok(id)
    }

    // ------------------------=
    // FUNC: stop
    // DESC: Ends listening immediately and removes the microphone lease from the session.
    // ------------------=
    pub fn stop(&mut self, id: u32) -> Result<(), AiError> {
        if self.session.map(|s| s.id) != Some(id) {
            return Err(AiError::InvalidRequest);
        }
        self.session = None;
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
