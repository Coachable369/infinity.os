use super::generation::{
    classify, generate, ResponseKind, CREATIVE_MODEL_ID, DIALOGUE_MODEL_ID, MAX_GENERATED_BYTES,
};
use super::types::ModelId;

pub const CHAT_MESSAGE_CAPACITY: usize = 8;
pub const CHAT_TEXT_CAPACITY: usize = 192;
pub const CHAT_INPUT_CAPACITY: usize = 96;
pub const SYSTEM_ASSISTANT_MODEL_ID: ModelId = DIALOGUE_MODEL_ID;
pub const INTENT_ASSISTANT_MODEL_ID: ModelId = super::model::LOCAL_INTENT_MODEL_ID;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    bytes: [u8; CHAT_TEXT_CAPACITY],
    length: u8,
}

impl ChatMessage {
    // ------------------------=
    // FUNC: new
    // DESC: Creates one bounded chat message without heap allocation.
    // ------------------=
    pub fn new(role: ChatRole, text: &[u8]) -> Self {
        let mut message = Self {
            role,
            bytes: [0; CHAT_TEXT_CAPACITY],
            length: 0,
        };
        let length = text.len().min(CHAT_TEXT_CAPACITY);
        message.bytes[..length].copy_from_slice(&text[..length]);
        message.length = length as u8;
        message
    }

    // ------------------------=
    // FUNC: text
    // DESC: Returns the initialized message bytes for rendering and typed consumers.
    // ------------------=
    pub fn text(&self) -> &[u8] {
        &self.bytes[..self.length as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatModel {
    pub id: ModelId,
    pub name: &'static [u8],
    pub description: &'static [u8],
}

pub const CHAT_MODELS: [ChatModel; 3] = [
    ChatModel {
        id: SYSTEM_ASSISTANT_MODEL_ID,
        name: b"Infinity Dialogue v1",
        description: b"Local conversational reasoning",
    },
    ChatModel {
        id: CREATIVE_MODEL_ID,
        name: b"Infinity Creative v1",
        description: b"Local creative conversation",
    },
    ChatModel {
        id: INTENT_ASSISTANT_MODEL_ID,
        name: b"Local Intent v1",
        description: b"Typed operation classification",
    },
];

#[derive(Clone, Copy)]
pub struct ChatRuntime {
    messages: [Option<ChatMessage>; CHAT_MESSAGE_CAPACITY],
    count: usize,
    selected_model: ModelId,
    enabled: bool,
    minimized: bool,
    input: [u8; CHAT_INPUT_CAPACITY],
    input_length: usize,
    last_response_kind: Option<ResponseKind>,
}

impl ChatRuntime {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an enabled desktop chat session with a bounded local history.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            messages: [None; CHAT_MESSAGE_CAPACITY],
            count: 0,
            selected_model: SYSTEM_ASSISTANT_MODEL_ID,
            enabled: true,
            minimized: false,
            input: [0; CHAT_INPUT_CAPACITY],
            input_length: 0,
            last_response_kind: None,
        }
    }

    // ------------------------=
    // FUNC: message_count
    // DESC: Reports the number of retained conversation messages.
    // ------------------=
    pub const fn message_count(&self) -> usize {
        self.count
    }

    // ------------------------=
    // FUNC: message
    // DESC: Returns one retained conversation message by chronological index.
    // ------------------=
    pub fn message(&self, index: usize) -> Option<&ChatMessage> {
        self.messages.get(index).and_then(Option::as_ref)
    }

    // ------------------------=
    // FUNC: selected_model
    // DESC: Returns the typed identifier of the active chat model.
    // ------------------=
    pub const fn selected_model(&self) -> ModelId {
        self.selected_model
    }

    // ------------------------=
    // FUNC: last_response_kind
    // DESC: Returns the typed response policy used for the most recent conversational turn.
    // ------------------=
    pub const fn last_response_kind(&self) -> Option<ResponseKind> {
        self.last_response_kind
    }

    // ------------------------=
    // FUNC: selected_model_index
    // DESC: Returns the installed chat-model index used by persistent preferences.
    // ------------------=
    pub fn selected_model_index(&self) -> usize {
        CHAT_MODELS
            .iter()
            .position(|model| model.id == self.selected_model)
            .unwrap_or(0)
    }

    // ------------------------=
    // FUNC: selected_model_descriptor
    // DESC: Returns display metadata for the active installed chat model.
    // ------------------=
    pub fn selected_model_descriptor(&self) -> ChatModel {
        CHAT_MODELS[self.selected_model_index()]
    }

    // ------------------------=
    // FUNC: enabled
    // DESC: Reports whether the user's desktop chat panel is enabled.
    // ------------------=
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    // ------------------------=
    // FUNC: set_enabled
    // DESC: Applies the user's persistent desktop chat preference and restores the panel when enabled.
    // ------------------=
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.minimized = false;
        }
    }

    // ------------------------=
    // FUNC: select_model_index
    // DESC: Selects a real installed chat model by bounded registry index.
    // ------------------=
    pub fn select_model_index(&mut self, index: usize) -> bool {
        let Some(model) = CHAT_MODELS.get(index) else {
            return false;
        };
        self.selected_model = model.id;
        true
    }

    // ------------------------=
    // FUNC: select_next_model
    // DESC: Cycles to the next installed chat model for pointer and keyboard controls.
    // ------------------=
    pub fn select_next_model(&mut self) -> usize {
        let next = (self.selected_model_index() + 1) % CHAT_MODELS.len();
        self.selected_model = CHAT_MODELS[next].id;
        next
    }

    // ------------------------=
    // FUNC: minimized
    // DESC: Reports whether the desktop chat is collapsed for this session.
    // ------------------=
    pub const fn minimized(&self) -> bool {
        self.minimized
    }

    // ------------------------=
    // FUNC: set_minimized
    // DESC: Collapses or restores the desktop chat without altering persistent enablement.
    // ------------------=
    pub fn set_minimized(&mut self, minimized: bool) {
        self.minimized = minimized;
    }

    // ------------------------=
    // FUNC: input
    // DESC: Returns the bounded active composer input.
    // ------------------=
    pub fn input(&self) -> &[u8] {
        &self.input[..self.input_length]
    }

    // ------------------------=
    // FUNC: push_input
    // DESC: Appends one printable character to the bounded composer.
    // ------------------=
    pub fn push_input(&mut self, byte: u8) -> bool {
        if !(b' '..=b'~').contains(&byte) || self.input_length == CHAT_INPUT_CAPACITY {
            return false;
        }
        self.input[self.input_length] = byte;
        self.input_length += 1;
        true
    }

    // ------------------------=
    // FUNC: pop_input
    // DESC: Removes one character from the active composer.
    // ------------------=
    pub fn pop_input(&mut self) -> bool {
        if self.input_length == 0 {
            return false;
        }
        self.input_length -= 1;
        self.input[self.input_length] = 0;
        true
    }

    // ------------------------=
    // FUNC: submit_input
    // DESC: Submits the composer as a conversational turn and clears it after acceptance.
    // ------------------=
    pub fn submit_input(&mut self) -> bool {
        let input = self.input;
        let length = self.input_length;
        if !self.submit(&input[..length]) {
            return false;
        }
        self.input.fill(0);
        self.input_length = 0;
        true
    }

    // ------------------------=
    // FUNC: submit
    // DESC: Appends a user turn and a bounded local response produced by the selected model.
    // ------------------=
    pub fn submit(&mut self, input: &[u8]) -> bool {
        let trimmed = trim_ascii(input);
        if trimmed.is_empty() {
            return false;
        }
        self.push(ChatMessage::new(ChatRole::User, trimmed));
        if self.selected_model == INTENT_ASSISTANT_MODEL_ID {
            self.last_response_kind = None;
            self.push(ChatMessage::new(
                ChatRole::Assistant,
                intent_response(trimmed),
            ));
        } else {
            self.last_response_kind = Some(classify(trimmed));
            let mut response = [0u8; MAX_GENERATED_BYTES];
            let length = generate(self.selected_model, trimmed, &mut response);
            self.push(ChatMessage::new(ChatRole::Assistant, &response[..length]));
        }
        true
    }

    // ------------------------=
    // FUNC: state_hash
    // DESC: Produces a non-secret change token for damage-aware UI invalidation.
    // ------------------=
    pub fn state_hash(&self) -> u64 {
        let mut value = self.selected_model as u64
            ^ ((self.minimized as u64) << 63)
            ^ ((self.enabled as u64) << 62);
        value ^= (self.count as u64) << 48;
        value ^= self
            .last_response_kind
            .map(|kind| (kind as u64) << 32)
            .unwrap_or(0);
        for byte in &self.input[..self.input_length] {
            value = value.rotate_left(7) ^ u64::from(*byte);
        }
        for message in self.messages.iter().flatten() {
            value = value.rotate_left(5) ^ message.length as u64 ^ message.role as u64;
        }
        value
    }

    // ------------------------=
    // FUNC: push
    // DESC: Adds one message while evicting the oldest entry at the fixed history limit.
    // ------------------=
    fn push(&mut self, message: ChatMessage) {
        if self.count == CHAT_MESSAGE_CAPACITY {
            self.messages.copy_within(1..CHAT_MESSAGE_CAPACITY, 0);
            self.count -= 1;
        }
        self.messages[self.count] = Some(message);
        self.count += 1;
    }
}

// ------------------------=
// FUNC: trim_ascii
// DESC: Removes surrounding ASCII whitespace from one bounded chat turn.
// ------------------=
fn trim_ascii(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map(|index| index + 1)
        .unwrap_or(start);
    &input[start..end]
}

// ------------------------=
// FUNC: contains_ascii_case_insensitive
// DESC: Finds one ASCII phrase without allocating a normalized copy.
// ------------------=
fn contains_ascii_case_insensitive(input: &[u8], needle: &[u8]) -> bool {
    input.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

// ------------------------=
// FUNC: intent_response
// DESC: Projects the bundled intent model's supported operation classes into a conversational result.
// ------------------=
fn intent_response(input: &[u8]) -> &'static [u8] {
    if contains_ascii_case_insensitive(input, b"device") {
        b"Typed intent: Device.List. Open Devices to inspect the discovered capability set."
    } else if contains_ascii_case_insensitive(input, b"memory") {
        b"Typed intent: System.MemoryStatus. The request is ready for capability validation."
    } else if contains_ascii_case_insensitive(input, b"boot") {
        b"Typed intent: System.BootStatus. The request is ready for capability validation."
    } else if contains_ascii_case_insensitive(input, b"status") {
        b"Typed intent: System.Status. The request is ready for capability validation."
    } else {
        b"No confident typed operation was found. Rephrase with a system domain and action."
    }
}
