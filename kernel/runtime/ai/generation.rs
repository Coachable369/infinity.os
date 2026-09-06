pub type ModelId = u32;

pub const DIALOGUE_MODEL_ID: ModelId = 0x4149_1001;
pub const CREATIVE_MODEL_ID: ModelId = 0x4149_1002;
pub const MAX_GENERATED_BYTES: usize = 192;
pub const CONVERSATION_MODEL_OBJECT_BYTES: usize = 2048;
pub const CONVERSATION_MODEL_MAGIC: &[u8; 8] = b"INFGEN1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseKind {
    Greeting,
    Identity,
    Capability,
    Gratitude,
    System,
    Network,
    Storage,
    Help,
    Creative,
    Comparison,
    UnsupportedQuestion,
    Clarification,
}

const DIALOGUE_RESPONSES: [&[u8]; 10] = [
    b"Hello there! How can I help you with InfinityOS today?",
    b"I'm Infinity, your local InfinityOS assistant. I run on this device and use only explicitly granted capabilities.",
    b"I can help inspect InfinityOS, explain its storage and networking, and translate requests into typed system operations.",
    b"You're welcome. What would you like to explore next?",
    b"I can help inspect system status through typed operations. Open System Overview for live values.",
    b"I can help with interfaces, connectivity profiles, routes, resolution, and per-application network policy.",
    b"Infinity Pool organizes native objects across System, Personal, Applications, and Recovery spaces.",
    b"Try asking about system status, storage, networking, devices, or what I can do.",
    b"I don't have enough verified knowledge in the installed local model to answer that reliably yet.",
    b"I want to answer accurately. Could you add a little more detail about the outcome you want?",
];

const CREATIVE_OPENINGS: [&[u8]; 4] = [
    b"A strong starting concept for ",
    b"One imaginative direction for ",
    b"We could shape ",
    b"A vivid interpretation of ",
];

const CREATIVE_ENDINGS: [&[u8]; 4] = [
    b" is to begin with one clear idea, add a memorable contrast, and refine the details around it.",
    b" is to combine a grounded structure with one surprising element that gives it identity.",
    b" would use a clear point of view, purposeful motion, and a restrained visual or emotional motif.",
    b" can emerge through three distinct variations, followed by choosing the most coherent direction.",
];

const COMPARISON_ENDINGS: [&[u8]; 2] = [
    b". I would compare their goals, constraints, risks, and measurable outcomes before choosing.",
    b". A useful comparison separates what each option does well, where it fails, and which tradeoff matters most.",
];

// ------------------------=
// FUNC: generate
// DESC: Produces a bounded coherent response from the selected native conversational model.
// ------------------=
pub fn generate(model: ModelId, input: &[u8], output: &mut [u8; MAX_GENERATED_BYTES]) -> usize {
    output.fill(0);
    let kind = classify(input);
    let seed = prompt_hash(input) ^ model;
    let mut writer = ResponseWriter::new(output);
    match kind {
        ResponseKind::Greeting => writer.push(DIALOGUE_RESPONSES[0]),
        ResponseKind::Identity => writer.push(DIALOGUE_RESPONSES[1]),
        ResponseKind::Capability => writer.push(DIALOGUE_RESPONSES[2]),
        ResponseKind::Gratitude => writer.push(DIALOGUE_RESPONSES[3]),
        ResponseKind::System => writer.push(DIALOGUE_RESPONSES[4]),
        ResponseKind::Network => writer.push(DIALOGUE_RESPONSES[5]),
        ResponseKind::Storage => writer.push(DIALOGUE_RESPONSES[6]),
        ResponseKind::Help => writer.push(DIALOGUE_RESPONSES[7]),
        ResponseKind::Creative => {
            writer.push(CREATIVE_OPENINGS[seed as usize % CREATIVE_OPENINGS.len()]);
            writer.push_topic_or_default(topic_span(input));
            writer.push(CREATIVE_ENDINGS[seed.rotate_left(11) as usize % CREATIVE_ENDINGS.len()]);
        }
        ResponseKind::Comparison => {
            writer.push(b"To compare ");
            writer.push_topic_or_default(topic_span(input));
            writer.push(COMPARISON_ENDINGS[seed as usize % COMPARISON_ENDINGS.len()]);
        }
        ResponseKind::UnsupportedQuestion => writer.push(DIALOGUE_RESPONSES[8]),
        ResponseKind::Clarification => {
            if model == CREATIVE_MODEL_ID {
                writer.push(b"I can help develop that idea. What should the result communicate, and who is it for?");
            } else {
                writer.push(DIALOGUE_RESPONSES[9]);
            }
        }
    }
    writer.length()
}

// ------------------------=
// FUNC: classify
// DESC: Classifies one chat turn into a typed response policy without using rendered text as state.
// ------------------=
pub fn classify(input: &[u8]) -> ResponseKind {
    if has_any_word(input, &[b"hello", b"hi", b"hey", b"greetings"]) {
        ResponseKind::Greeting
    } else if contains_ascii(input, b"who are you") || contains_ascii(input, b"what are you") {
        ResponseKind::Identity
    } else if contains_ascii(input, b"what can you do")
        || contains_ascii(input, b"your capabilities")
    {
        ResponseKind::Capability
    } else if has_any_word(input, &[b"thanks", b"thank"]) {
        ResponseKind::Gratitude
    } else if has_any_word(input, &[b"network", b"wifi", b"wired", b"route", b"dns"]) {
        ResponseKind::Network
    } else if has_any_word(input, &[b"storage", b"disk", b"pool", b"object"]) {
        ResponseKind::Storage
    } else if has_any_word(
        input,
        &[b"system", b"memory", b"device", b"boot", b"status"],
    ) {
        ResponseKind::System
    } else if has_word(input, b"help") {
        ResponseKind::Help
    } else if has_any_word(
        input,
        &[
            b"create",
            b"write",
            b"brainstorm",
            b"imagine",
            b"story",
            b"poem",
        ],
    ) {
        ResponseKind::Creative
    } else if has_any_word(input, &[b"compare", b"versus", b"difference"]) {
        ResponseKind::Comparison
    } else if input.contains(&b'?')
        || has_any_word(
            input,
            &[b"what", b"why", b"when", b"where", b"which", b"how"],
        )
    {
        ResponseKind::UnsupportedQuestion
    } else {
        ResponseKind::Clarification
    }
}

// ------------------------=
// FUNC: model_checksum
// DESC: Computes the integrity identity of one bundled conversational response policy.
// ------------------=
pub fn model_checksum(model: ModelId) -> u32 {
    let mut value = 0x811c_9dc5u32 ^ model;
    for clause in DIALOGUE_RESPONSES
        .iter()
        .chain(CREATIVE_OPENINGS.iter())
        .chain(CREATIVE_ENDINGS.iter())
        .chain(COMPARISON_ENDINGS.iter())
    {
        for byte in *clause {
            value = (value ^ u32::from(*byte)).wrapping_mul(0x0100_0193);
        }
    }
    value
}

// ------------------------=
// FUNC: model_object_bytes
// DESC: Serializes a bundled conversational response policy into a versioned native model object.
// ------------------=
pub fn model_object_bytes(model: ModelId) -> [u8; CONVERSATION_MODEL_OBJECT_BYTES] {
    let mut output = [0u8; CONVERSATION_MODEL_OBJECT_BYTES];
    output[..8].copy_from_slice(CONVERSATION_MODEL_MAGIC);
    output[8..10].copy_from_slice(&2u16.to_le_bytes());
    output[12..16].copy_from_slice(&model.to_le_bytes());
    output[16..20].copy_from_slice(&model_checksum(model).to_le_bytes());
    output[20..24].copy_from_slice(&(DIALOGUE_RESPONSES.len() as u32).to_le_bytes());
    output
}

// ------------------------=
// FUNC: model_object_valid
// DESC: Validates a native conversational model object before registry binding.
// ------------------=
pub fn model_object_valid(model: ModelId, bytes: &[u8]) -> bool {
    bytes.len() == CONVERSATION_MODEL_OBJECT_BYTES
        && &bytes[..8] == CONVERSATION_MODEL_MAGIC
        && u16::from_le_bytes([bytes[8], bytes[9]]) == 2
        && u32::from_le_bytes(bytes[12..16].try_into().unwrap_or([0; 4])) == model
        && u32::from_le_bytes(bytes[16..20].try_into().unwrap_or([0; 4])) == model_checksum(model)
        && (model == DIALOGUE_MODEL_ID || model == CREATIVE_MODEL_ID)
}

// ------------------------=
// FUNC: prompt_hash
// DESC: Derives a stable response-selection seed from prompt bytes without retaining prompt data.
// ------------------=
fn prompt_hash(input: &[u8]) -> u32 {
    let mut value = 0x811c_9dc5u32;
    for byte in input {
        value = (value ^ u32::from(byte.to_ascii_lowercase())).wrapping_mul(0x0100_0193);
    }
    value
}

// ------------------------=
// FUNC: contains_ascii
// DESC: Finds one ASCII phrase case-insensitively without allocating a normalized prompt.
// ------------------=
fn contains_ascii(input: &[u8], needle: &[u8]) -> bool {
    input.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

// ------------------------=
// FUNC: has_word
// DESC: Finds one complete ASCII word so short intents do not match inside unrelated words.
// ------------------=
fn has_word(input: &[u8], word: &[u8]) -> bool {
    input
        .windows(word.len())
        .enumerate()
        .any(|(index, window)| {
            window.eq_ignore_ascii_case(word)
                && (index == 0 || !input[index - 1].is_ascii_alphanumeric())
                && (index + word.len() == input.len()
                    || !input[index + word.len()].is_ascii_alphanumeric())
        })
}

// ------------------------=
// FUNC: has_any_word
// DESC: Tests a bounded word vocabulary against one conversational prompt.
// ------------------=
fn has_any_word(input: &[u8], words: &[&[u8]]) -> bool {
    words.iter().any(|word| has_word(input, word))
}

// ------------------------=
// FUNC: topic_span
// DESC: Selects a bounded subject span for creative and comparison responses.
// ------------------=
fn topic_span(input: &[u8]) -> &[u8] {
    let mut start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    for _ in 0..2 {
        let mut advanced = false;
        for prefix in [
            b"please ".as_slice(),
            b"can you ",
            b"could you ",
            b"compare ",
            b"create ",
            b"write ",
            b"brainstorm ",
            b"imagine ",
        ] {
            if input
                .get(start..start.saturating_add(prefix.len()))
                .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
            {
                start += prefix.len();
                advanced = true;
                break;
            }
        }
        if !advanced {
            break;
        }
    }
    let mut end = input.len().min(start.saturating_add(56));
    while end > start && (input[end - 1].is_ascii_whitespace() || b"?.!".contains(&input[end - 1]))
    {
        end -= 1;
    }
    &input[start..end]
}

struct ResponseWriter<'a> {
    output: &'a mut [u8; MAX_GENERATED_BYTES],
    length: usize,
}

impl<'a> ResponseWriter<'a> {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a bounded response writer over caller-owned model output memory.
    // ------------------=
    fn new(output: &'a mut [u8; MAX_GENERATED_BYTES]) -> Self {
        Self { output, length: 0 }
    }

    // ------------------------=
    // FUNC: push
    // DESC: Appends static response bytes without exceeding the declared generation budget.
    // ------------------=
    fn push(&mut self, bytes: &[u8]) {
        let available = self.output.len().saturating_sub(self.length);
        let count = bytes.len().min(available);
        self.output[self.length..self.length + count].copy_from_slice(&bytes[..count]);
        self.length += count;
    }

    // ------------------------=
    // FUNC: push_topic
    // DESC: Appends only printable prompt characters and normalizes whitespace for safe readable output.
    // ------------------=
    fn push_topic(&mut self, bytes: &[u8]) {
        let mut spaced = false;
        for byte in bytes {
            if byte.is_ascii_whitespace() {
                if !spaced && self.length > 0 {
                    self.push(b" ");
                    spaced = true;
                }
            } else if byte.is_ascii_graphic() {
                self.push(core::slice::from_ref(byte));
                spaced = false;
            }
        }
        if self.length > 0 && self.output[self.length - 1] == b' ' {
            self.length -= 1;
        }
    }

    // ------------------------=
    // FUNC: push_topic_or_default
    // DESC: Appends a sanitized topic or a grammatical fallback when the prompt has no subject.
    // ------------------=
    fn push_topic_or_default(&mut self, bytes: &[u8]) {
        let before = self.length;
        self.push_topic(bytes);
        if self.length == before {
            self.push(b"that idea");
        }
    }

    // ------------------------=
    // FUNC: length
    // DESC: Reports the initialized model-output byte count.
    // ------------------=
    fn length(&self) -> usize {
        self.length
    }
}
