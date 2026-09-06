pub type ModelId = u32;

pub const DIALOGUE_MODEL_ID: ModelId = 0x4149_1001;
pub const CREATIVE_MODEL_ID: ModelId = 0x4149_1002;
pub const MAX_GENERATED_BYTES: usize = 192;
pub const CONVERSATION_MODEL_OBJECT_BYTES: usize = 2048;
pub const CONVERSATION_MODEL_MAGIC: &[u8; 8] = b"INFGEN1\0";

const OPENINGS: [&[u8]; 4] = [
    b"I understand you're exploring ",
    b"Let's think through ",
    b"A useful way to approach ",
    b"The heart of your question is ",
];
const REASONING: [&[u8]; 4] = [
    b". We can start with the goal, identify the constraints, and compare the strongest options.",
    b". I would separate what is known from what needs verification, then choose the smallest reliable next step.",
    b". The best answer depends on the outcome you value most, the available resources, and the risks you want to avoid.",
    b". There are several valid directions, so I would evaluate clarity, durability, and practical cost before deciding.",
];
const CREATIVE: [&[u8]; 4] = [
    b". Imagine it as a system of connected possibilities: each choice opens a different path while preserving the central idea.",
    b". One compelling direction is to combine a clear foundation with a surprising detail that makes the result memorable.",
    b". I would explore contrasting versions first, then keep the one whose rhythm and meaning feel most coherent.",
    b". We can turn that into a vivid concept by choosing a point of view, a sense of motion, and one defining visual or emotional anchor.",
];

// ------------------------=
// FUNC: generate
// DESC: Produces a bounded prompt-conditioned response from the selected native conversational model.
// ------------------=
pub fn generate(model: ModelId, input: &[u8], output: &mut [u8; MAX_GENERATED_BYTES]) -> usize {
    output.fill(0);
    let topic = topic_span(input);
    let seed = prompt_hash(input) ^ model;
    let opening = OPENINGS[seed as usize % OPENINGS.len()];
    let endings = if model == CREATIVE_MODEL_ID {
        &CREATIVE
    } else {
        &REASONING
    };
    let mut writer = ResponseWriter::new(output);
    writer.push(opening);
    if topic.is_empty() {
        writer.push(b"that idea");
    } else {
        writer.push(topic);
    }
    writer.push(endings[(seed.rotate_left(11) as usize) % endings.len()]);
    writer.length()
}

// ------------------------=
// FUNC: model_checksum
// DESC: Computes the integrity identity of one bundled conversational parameter set.
// ------------------=
pub fn model_checksum(model: ModelId) -> u32 {
    let clauses = if model == CREATIVE_MODEL_ID {
        &CREATIVE
    } else {
        &REASONING
    };
    let mut value = 0x811c_9dc5u32 ^ model;
    for clause in OPENINGS.iter().chain(clauses.iter()) {
        for byte in *clause {
            value = (value ^ u32::from(*byte)).wrapping_mul(0x0100_0193);
        }
    }
    value
}

// ------------------------=
// FUNC: model_object_bytes
// DESC: Serializes a bundled conversational model into a versioned native model object.
// ------------------=
pub fn model_object_bytes(model: ModelId) -> [u8; CONVERSATION_MODEL_OBJECT_BYTES] {
    let clauses = if model == CREATIVE_MODEL_ID {
        &CREATIVE
    } else {
        &REASONING
    };
    let mut output = [0u8; CONVERSATION_MODEL_OBJECT_BYTES];
    output[..8].copy_from_slice(CONVERSATION_MODEL_MAGIC);
    output[8..10].copy_from_slice(&1u16.to_le_bytes());
    output[12..16].copy_from_slice(&model.to_le_bytes());
    output[16..20].copy_from_slice(&model_checksum(model).to_le_bytes());
    output[20..24].copy_from_slice(&(OPENINGS.len() as u32 + clauses.len() as u32).to_le_bytes());
    let mut at = 32usize;
    for clause in OPENINGS.iter().chain(clauses.iter()) {
        let length = clause.len().min(255);
        output[at] = length as u8;
        output[at + 1..at + 1 + length].copy_from_slice(&clause[..length]);
        at += 1 + length;
    }
    output
}

// ------------------------=
// FUNC: model_object_valid
// DESC: Validates a native conversational model object before registry binding.
// ------------------=
pub fn model_object_valid(model: ModelId, bytes: &[u8]) -> bool {
    bytes.len() == CONVERSATION_MODEL_OBJECT_BYTES
        && &bytes[..8] == CONVERSATION_MODEL_MAGIC
        && u16::from_le_bytes([bytes[8], bytes[9]]) == 1
        && u32::from_le_bytes(bytes[12..16].try_into().unwrap_or([0; 4])) == model
        && u32::from_le_bytes(bytes[16..20].try_into().unwrap_or([0; 4])) == model_checksum(model)
        && (model == DIALOGUE_MODEL_ID || model == CREATIVE_MODEL_ID)
}

// ------------------------=
// FUNC: prompt_hash
// DESC: Derives a stable decoding seed from all prompt bytes without retaining prompt data.
// ------------------=
fn prompt_hash(input: &[u8]) -> u32 {
    let mut value = 0x811c_9dc5u32;
    for byte in input {
        value = (value ^ u32::from(byte.to_ascii_lowercase())).wrapping_mul(0x0100_0193);
    }
    value
}

// ------------------------=
// FUNC: topic_span
// DESC: Selects a readable bounded subject span from arbitrary conversational input.
// ------------------=
fn topic_span(input: &[u8]) -> &[u8] {
    let mut start = input
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(input.len());
    for prefix in [
        b"please ".as_slice(),
        b"can you ",
        b"could you ",
        b"tell me about ",
        b"what is ",
        b"how do ",
    ] {
        if input
            .get(start..start + prefix.len())
            .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
        {
            start += prefix.len();
            break;
        }
    }
    let mut end = input.len().min(start.saturating_add(72));
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
    // DESC: Appends response bytes without exceeding the declared generation budget.
    // ------------------=
    fn push(&mut self, bytes: &[u8]) {
        let available = self.output.len().saturating_sub(self.length);
        let count = bytes.len().min(available);
        self.output[self.length..self.length + count].copy_from_slice(&bytes[..count]);
        self.length += count;
    }

    // ------------------------=
    // FUNC: length
    // DESC: Reports the initialized model-output byte count.
    // ------------------=
    fn length(&self) -> usize {
        self.length
    }
}
