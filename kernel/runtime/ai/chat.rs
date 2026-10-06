use super::generation::{
    classify, generate, ResponseKind, CREATIVE_MODEL_ID, DIALOGUE_MODEL_ID, MAX_GENERATED_BYTES,
};
use super::memory::{AiMemory, MemoryResponseKind};
use super::types::ModelId;

pub const CHAT_MESSAGE_CAPACITY: usize = 8;
pub const CHAT_TEXT_CAPACITY: usize = 16384;
pub const CHAT_INPUT_CAPACITY: usize = 4096;
pub const SYSTEM_ASSISTANT_MODEL_ID: ModelId = DIALOGUE_MODEL_ID;
pub const INTENT_ASSISTANT_MODEL_ID: ModelId = super::model::LOCAL_INTENT_MODEL_ID;
pub const MINISTRAL_MODEL_ID: ModelId = 0x4149_1004;
pub const HERMES_MODEL_ID: ModelId = 0x4149_1005;

// ------------------------=
// FUNC: response_line_height
// DESC: Matches scale-one chat glyph height with a compact baseline gap, independent of desktop geometry scaling.
// ------------------=
pub const fn response_line_height(cell_height: usize, font_pixels: usize, atlas_pixels: usize) -> usize {
    cell_height * font_pixels / if atlas_pixels == 0 { 1 } else { atlas_pixels } + 2
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationState {
    Ready,
    Running,
    Complete,
    Cancelled,
    Failed,
    ContextFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    bytes: [u8; CHAT_TEXT_CAPACITY],
    length: u16,
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
        message.length = length as u16;
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

pub const CHAT_MODELS: [ChatModel; 5] = [
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
    ChatModel {
        id: HERMES_MODEL_ID,
        name: b"Hermes 3 Llama 3.2 3B",
        description: b"Primary local intent/tools - Q4_K_M - 4K context",
    },
    ChatModel {
        id: MINISTRAL_MODEL_ID,
        name: b"Ministral 3 3B",
        description: b"Lightweight local CPU - Q4_K_M - 4K context",
    },
];

#[derive(Clone, Copy)]
pub struct ChatRuntime {
    messages: [Option<ChatMessage>; CHAT_MESSAGE_CAPACITY],
    count: usize,
    turn_id: u64,
    selected_model: ModelId,
    ministral_ready: bool,
    hermes_ready: bool,
    pub generation_state: GenerationState,
    enabled: bool,
    minimized: bool,
    input: [u8; CHAT_INPUT_CAPACITY],
    input_length: usize,
    input_cursor: usize,
    last_response_kind: Option<ResponseKind>,
    memory: AiMemory,
    memory_dirty: bool,
    last_memory_response: Option<MemoryResponseKind>,
    timeline_scroll_offset: usize,
    timeline_maximum_scroll: usize,
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
            turn_id: 0,
            selected_model: HERMES_MODEL_ID,
            ministral_ready: false,
            hermes_ready: false,
            generation_state: GenerationState::Ready,
            enabled: true,
            minimized: false,
            input: [0; CHAT_INPUT_CAPACITY],
            input_length: 0,
            input_cursor: 0,
            last_response_kind: None,
            memory: AiMemory::new(),
            memory_dirty: false,
            last_memory_response: None,
            timeline_scroll_offset: 0,
            timeline_maximum_scroll: 0,
        }
    }

    // ------------------------=
    // FUNC: timeline_scroll_offset
    // DESC: Returns the bounded pixel offset used by the desktop conversation viewport.
    // ------------------=
    pub const fn timeline_scroll_offset(&self) -> usize {
        self.timeline_scroll_offset
    }

    // ------------------------=
    // FUNC: timeline_maximum_scroll
    // DESC: Returns the most recently measured scroll extent for the rendered conversation.
    // ------------------=
    pub const fn timeline_maximum_scroll(&self) -> usize {
        self.timeline_maximum_scroll
    }

    // ------------------------=
    // FUNC: set_timeline_scroll_metrics
    // DESC: Publishes a measured extent, preserving bottom-following while respecting deliberate history review.
    // ------------------=
    pub fn set_timeline_scroll_metrics(&mut self, maximum: usize) -> usize {
        let followed_end = self.timeline_scroll_offset >= self.timeline_maximum_scroll;
        self.timeline_maximum_scroll = maximum;
        self.timeline_scroll_offset = if followed_end {
            maximum
        } else {
            self.timeline_scroll_offset.min(maximum)
        };
        self.timeline_scroll_offset
    }

    // ------------------------=
    // FUNC: scroll_timeline
    // DESC: Moves the desktop conversation viewport within its last measured content extent.
    // ------------------=
    pub fn scroll_timeline(&mut self, delta: isize) -> bool {
        let previous = self.timeline_scroll_offset;
        self.timeline_scroll_offset = if delta < 0 {
            self.timeline_scroll_offset
                .saturating_sub(delta.unsigned_abs())
        } else {
            self.timeline_scroll_offset
                .saturating_add(delta as usize)
                .min(self.timeline_maximum_scroll)
        };
        self.timeline_scroll_offset != previous
    }

    // ------------------------=
    // FUNC: scroll_timeline_to_end
    // DESC: Restores automatic following when the user submits a new turn.
    // ------------------=
    pub fn scroll_timeline_to_end(&mut self) {
        self.timeline_scroll_offset = self.timeline_maximum_scroll;
    }

    // ------------------------=
    // FUNC: message_count
    // DESC: Reports the number of retained conversation messages.
    // ------------------=
    pub const fn message_count(&self) -> usize {
        self.count
    }

    // ------------------------=
    // FUNC: turn_id
    // DESC: Identifies an accepted turn independently of bounded history rollover.
    // ------------------=
    pub const fn turn_id(&self) -> u64 {
        self.turn_id
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
    // FUNC: selected_model_ready
    // DESC: Keeps catalog selection distinct from actual local inference availability.
    // ------------------=
    pub const fn selected_model_ready(&self) -> bool {
        match self.selected_model {
            MINISTRAL_MODEL_ID => self.ministral_ready,
            HERMES_MODEL_ID => self.hermes_ready,
            _ => true,
        }
    }

    // ------------------------=
    // FUNC: set_ministral_ready
    // DESC: Makes the second model ready only after verified native initialization.
    // ------------------=
    pub fn set_ministral_ready(&mut self, ready: bool) {
        self.ministral_ready = ready;
    }
    // ------------------------=
    // FUNC: set_hermes_ready
    // DESC: Exposes Hermes availability only after pinned native initialization.
    // ------------------=
    pub fn set_hermes_ready(&mut self, ready: bool) {
        self.hermes_ready = ready;
    }

    // ------------------------=
    // FUNC: begin_native_turn
    // DESC: Records an accepted asynchronous turn without a canned response.
    // ------------------=
    pub fn begin_native_turn(&mut self) {
        self.turn_id = self.turn_id.wrapping_add(1);
        let input = self.input;
        self.push(ChatMessage::new(
            ChatRole::User,
            &input[..self.input_length],
        ));
        self.input.fill(0);
        self.input_length = 0;
        self.input_cursor = 0;
        self.scroll_timeline_to_end();
    }
    // ------------------------=
    // FUNC: update_native_response
    // DESC: Publishes only decoded model output in the active assistant turn.
    // ------------------=
    pub fn update_native_response(&mut self, bytes: &[u8]) {
        if self.count != 0
            && self.message(self.count - 1).is_some_and(|m| m.role == ChatRole::Assistant)
        {
            self.messages[self.count - 1] = Some(ChatMessage::new(ChatRole::Assistant, bytes));
        } else {
            self.push(ChatMessage::new(ChatRole::Assistant, bytes));
        }
        // Request bottom-following before the renderer measures the new text.
        // Manual history scrolling remains available between response updates.
        self.scroll_timeline_to_end();
    }

    // ------------------------=
    // FUNC: publish_native_completion
    // DESC: Publishes cumulative model output as it arrives and marks the turn complete only at its terminal token.
    // ------------------=
    pub fn publish_native_completion(&mut self, bytes: &[u8], complete: bool) -> bool {
        if (!complete && self.generation_state == GenerationState::Complete)
            || (bytes.is_empty() && !complete)
        {
            return false;
        }
        if !bytes.is_empty() {
            self.update_native_response(bytes);
        }
        if complete {
            self.generation_state = GenerationState::Complete;
        }
        true
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
    // FUNC: paste_input
    // DESC: Atomically inserts shared clipboard text without submitting a chat request.
    // ------------------=
    pub fn paste_input(&mut self,text:&[u8])->bool {
        crate::ui::text_input::paste_ascii(&mut self.input,&mut self.input_length,&mut self.input_cursor,text)
    }
    // ------------------------=
    // FUNC: clear_input
    // DESC: Erases composer bytes after a successful explicit cut.
    // ------------------=
    pub fn clear_input(&mut self) {self.input.fill(0);self.input_length=0;self.input_cursor=0;}

    // ------------------------=
    // FUNC: input_cursor
    // DESC: Returns the current bounded composer insertion position.
    // ------------------=
    pub const fn input_cursor(&self) -> usize {
        self.input_cursor
    }

    // ------------------------=
    // FUNC: set_input_cursor
    // DESC: Places the composer caret at one bounded pointer-derived insertion index.
    // ------------------=
    pub fn set_input_cursor(&mut self, index: usize) {
        self.input_cursor = index.min(self.input_length);
    }

    // ------------------------=
    // FUNC: edit_input
    // DESC: Applies standard printable insertion, deletion, and caret navigation to the composer.
    // ------------------=
    pub fn edit_input(&mut self, key: crate::ui::text_input::TextEditKey) -> bool {
        match key {
            crate::ui::text_input::TextEditKey::Character(value) => {
                crate::ui::text_input::insert_ascii(
                    &mut self.input,
                    &mut self.input_length,
                    &mut self.input_cursor,
                    value,
                )
            }
            crate::ui::text_input::TextEditKey::Backspace => crate::ui::text_input::backspace(
                &mut self.input,
                &mut self.input_length,
                &mut self.input_cursor,
            ),
            crate::ui::text_input::TextEditKey::Delete => crate::ui::text_input::delete(
                &mut self.input,
                &mut self.input_length,
                &mut self.input_cursor,
            ),
            crate::ui::text_input::TextEditKey::Left => {
                crate::ui::text_input::move_caret(&mut self.input_cursor, self.input_length, -1)
            }
            crate::ui::text_input::TextEditKey::Right => {
                crate::ui::text_input::move_caret(&mut self.input_cursor, self.input_length, 1)
            }
            crate::ui::text_input::TextEditKey::Home => {
                crate::ui::text_input::move_caret(&mut self.input_cursor, self.input_length, -2)
            }
            crate::ui::text_input::TextEditKey::End => {
                crate::ui::text_input::move_caret(&mut self.input_cursor, self.input_length, 2)
            }
        }
    }

    // ------------------------=
    // FUNC: set_memory
    // DESC: Restores the authenticated user's durable semantic memory into the chat session.
    // ------------------=
    pub fn set_memory(&mut self, memory: AiMemory) {
        self.memory = memory;
        self.memory_dirty = false;
    }

    // ------------------------=
    // FUNC: memory
    // DESC: Returns the active user's bounded semantic memory.
    // ------------------=
    pub const fn memory(&self) -> AiMemory {
        self.memory
    }

    // ------------------------=
    // FUNC: last_memory_response
    // DESC: Returns the typed semantic-memory behavior used for the latest turn.
    // ------------------=
    pub const fn last_memory_response(&self) -> Option<MemoryResponseKind> {
        self.last_memory_response
    }

    // ------------------------=
    // FUNC: take_memory_update
    // DESC: Returns a changed memory snapshot exactly once for durable checkpointing.
    // ------------------=
    pub fn take_memory_update(&mut self) -> Option<AiMemory> {
        if !self.memory_dirty {
            return None;
        }
        self.memory_dirty = false;
        Some(self.memory)
    }

    // ------------------------=
    // FUNC: push_input
    // DESC: Appends one printable character to the bounded composer.
    // ------------------=
    pub fn push_input(&mut self, byte: u8) -> bool {
        crate::ui::text_input::insert_ascii(
            &mut self.input,
            &mut self.input_length,
            &mut self.input_cursor,
            byte,
        )
    }

    // ------------------------=
    // FUNC: pop_input
    // DESC: Removes one character from the active composer.
    // ------------------=
    pub fn pop_input(&mut self) -> bool {
        crate::ui::text_input::backspace(
            &mut self.input,
            &mut self.input_length,
            &mut self.input_cursor,
        )
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
        self.input_cursor = 0;
        true
    }

    // ------------------------=
    // FUNC: submit
    // DESC: Appends a user turn and a bounded local response produced by the selected model.
    // ------------------=
    pub fn submit(&mut self, input: &[u8]) -> bool {
        if !self.selected_model_ready()
            || matches!(self.selected_model, MINISTRAL_MODEL_ID | HERMES_MODEL_ID)
        {
            return false;
        }
        let trimmed = trim_ascii(input);
        if trimmed.is_empty() {
            return false;
        }
        self.turn_id = self.turn_id.wrapping_add(1);
        self.scroll_timeline_to_end();
        self.push(ChatMessage::new(ChatRole::User, trimmed));
        let mut response = [0u8; MAX_GENERATED_BYTES];
        self.last_memory_response = None;
        if let Some((length, changed, kind)) = self.memory.respond(trimmed, &mut response) {
            self.last_response_kind = None;
            self.last_memory_response = Some(kind);
            self.memory_dirty |= changed;
            self.push(ChatMessage::new(ChatRole::Assistant, &response[..length]));
        } else if self.selected_model == INTENT_ASSISTANT_MODEL_ID {
            self.last_response_kind = None;
            self.push(ChatMessage::new(
                ChatRole::Assistant,
                intent_response(trimmed),
            ));
        } else {
            self.last_response_kind = Some(classify(trimmed));
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
        value ^= (self.timeline_scroll_offset as u64).rotate_left(19);
        value ^= self
            .last_response_kind
            .map(|kind| (kind as u64) << 32)
            .unwrap_or(0);
        for byte in &self.input[..self.input_length] {
            value = value.rotate_left(7) ^ u64::from(*byte);
        }
        for message in self.messages.iter().flatten() {
            value = value.rotate_left(5) ^ message.length as u64 ^ message.role as u64;
            for byte in message.text() {
                value = value.rotate_left(7) ^ u64::from(*byte);
            }
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
