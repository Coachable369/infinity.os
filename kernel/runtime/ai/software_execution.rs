//! Trusted native software-workflow receipts, separate from generated proposals.
//! This is an observer of the existing DevKit and application lifecycle, not an executor.

use super::{CapabilityId, Observation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    SourceSaved { object: [u8; 16], revision: u64 },
    Compiled { artifact: [u8; 16] },
    Launched { context: u64 },
    InteractionReady { context: u64, revision: u64 },
    InteractionApplied { context: u64, revision: u64, token: u64 },
    InteractionStopped { context: u64 },
    Failed { code: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Execution {
    pub goal: u64,
    pub owner: [u8; 16],
    pub source: [u8; 16],
    pub source_revision: u64,
    pub artifact: [u8; 16],
    pub context: u64,
    pub interaction_revision: u64,
    pub interaction_active: bool,
    pub applied_actions: u32,
    pub last_action: u64,
    failure: Option<u16>,
    active: Option<(u64, CapabilityId)>,
}

impl Execution {
    // ------------------------=
    // FUNC: new
    // DESC: Binds empty receipts to an exact goal owner without granting execution authority.
    // ------------------=
    pub const fn new(goal: u64, owner: [u8; 16]) -> Self {
        Self { goal, owner, source: [0; 16], source_revision: 0, artifact: [0; 16],
            context: 0, interaction_revision: 0, interaction_active: false,
            applied_actions: 0, last_action: 0, failure: None, active: None }
    }

    // ------------------------=
    // FUNC: begin
    // DESC: Associates a plan task with receipts while refusing overlapping or unsupported tasks.
    // ------------------=
    pub fn begin(&mut self, token: u64, capability: CapabilityId) -> bool {
        if token == 0 || self.active.is_some() || !matches!(capability,
            CapabilityId::DevKit | CapabilityId::UiServices | CapabilityId::ApplicationInteraction) {
            return false;
        }
        self.active = Some((token, capability));
        true
    }

    // ------------------------=
    // FUNC: owns
    // DESC: Identifies the exact task whose native receipts this observer may consume.
    // ------------------=
    pub fn owns(&self, token: u64) -> bool {
        self.active.is_some_and(|active| active.0 == token)
    }

    // ------------------------=
    // FUNC: record
    // DESC: Accepts ordered owner-bound native evidence, rejecting stale or invented empty receipts.
    // ------------------=
    pub fn record(&mut self, goal: u64, owner: [u8; 16], event: Event) -> bool {
        if self.goal != goal || self.owner != owner || self.failure.is_some() { return false; }
        match event {
            Event::SourceSaved { object, revision } if object != [0; 16]
                && self.artifact == [0; 16] && revision >= self.source_revision => {
                self.source = object;
                self.source_revision = revision;
            }
            Event::Compiled { artifact } if self.source != [0; 16] && artifact != [0; 16]
                && (self.artifact == [0; 16] || self.artifact == artifact) => self.artifact = artifact,
            Event::Launched { context } if self.artifact != [0; 16] && context != 0
                && (self.context == 0 || self.context == context) => self.context = context,
            Event::InteractionReady { context, revision } if self.context != 0
                && self.context == context && revision > 0
                && (!self.interaction_active || self.interaction_revision == 0) => {
                if self.interaction_revision == 0 { self.interaction_revision = revision; }
                self.interaction_active = true;
            }
            Event::InteractionApplied { context, revision, token } if self.context == context
                && self.interaction_active && revision > self.interaction_revision
                && token != 0 && token != self.last_action => {
                self.interaction_revision = revision;
                self.last_action = token;
                self.applied_actions = self.applied_actions.saturating_add(1);
            }
            Event::InteractionStopped { context } if self.context != 0 && self.context == context => {
                self.interaction_active = false;
            }
            Event::Failed { code } => self.failure = Some(code.max(1)),
            _ => return false,
        }
        true
    }

    // ------------------------=
    // FUNC: observe
    // DESC: Completes tasks only with persisted artifacts, a real process instance, or a granted published interaction contract.
    // ------------------=
    pub fn observe(&mut self, token: u64) -> Observation {
        let Some((active, capability)) = self.active else { return Observation::Running; };
        if active != token { return Observation::Running; }
        let result = if let Some(code) = self.failure {
            Observation::Failed(code)
        } else {
            match capability {
                CapabilityId::DevKit if self.artifact != [0; 16] => Observation::Succeeded(self.artifact),
                CapabilityId::UiServices if self.context != 0 => {
                    let mut receipt = [0; 16];
                    receipt[..8].copy_from_slice(&self.context.to_le_bytes());
                    receipt[8..].copy_from_slice(&self.goal.to_le_bytes());
                    Observation::Succeeded(receipt)
                }
                CapabilityId::ApplicationInteraction if self.interaction_active && self.interaction_revision > 0 => {
                    let mut receipt = [0; 16];
                    receipt[..8].copy_from_slice(&self.context.to_le_bytes());
                    receipt[8..].copy_from_slice(&self.interaction_revision.to_le_bytes());
                    Observation::Succeeded(receipt)
                }
                _ => Observation::Running,
            }
        };
        if !matches!(result, Observation::Running) { self.active = None; }
        result
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Invalidates the exact interrupted lease so later callbacks cannot falsely complete it.
    // ------------------=
    pub fn cancel(&mut self, token: u64) {
        if self.owns(token) {
            self.active = None;
            self.failure = Some(2);
        }
    }
}
