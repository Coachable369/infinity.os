pub const LOCAL_INTENT_MODEL_ID: u32 = 0x4149_0001;
pub const MODEL_OBJECT_MAGIC: &[u8; 8] = b"INFMLM1\0";
pub const CLASS_COUNT: usize = 5;
pub const MODEL_OBJECT_BYTES: usize = 1152;
const FEATURE_OFFSET: usize = 64;
const FEATURE_RECORD_BYTES: usize = 40;

#[derive(Clone, Copy)]
pub struct TokenWeight {
    pub token: &'static [u8],
    pub weights: [i16; CLASS_COUNT],
}

// Quantized parameters for a compact multinomial linear classifier. The
// inference engine tokenizes arbitrary input and sums calibrated feature
// weights; it does not match whole phrases or generate canned text.
pub const INTENT_WEIGHTS: [TokenWeight; 26] = [
    weight(b"status", [110, 0, 15, 10, 10]),
    weight(b"going", [170, 0, 10, 5, 0]),
    weight(b"health", [105, 0, 0, 0, 10]),
    weight(b"doing", [130, 0, 5, 0, 0]),
    weight(b"machine", [55, 5, 45, 5, 5]),
    weight(b"system", [45, 0, 50, 35, 12]),
    weight(b"devices", [0, 120, 0, 0, 0]),
    weight(b"device", [0, 110, 0, 0, 0]),
    weight(b"hardware", [0, 105, 25, 0, 0]),
    weight(b"connected", [0, 90, 0, 0, 0]),
    weight(b"information", [5, 0, 105, 0, 0]),
    weight(b"info", [0, 0, 110, 0, 0]),
    weight(b"about", [5, 0, 45, 0, 0]),
    weight(b"architecture", [0, 0, 100, 0, 0]),
    weight(b"boot", [5, 0, 0, 120, 0]),
    weight(b"booted", [0, 0, 5, 115, 0]),
    weight(b"started", [10, 0, 0, 85, 0]),
    weight(b"startup", [0, 0, 0, 90, 0]),
    weight(b"memory", [5, 0, 0, 0, 125]),
    weight(b"ram", [0, 0, 0, 0, 120]),
    weight(b"available", [10, 10, 0, 0, 45]),
    weight(b"usage", [10, 0, 0, 0, 65]),
    weight(b"show", [5, 12, 5, 5, 5]),
    weight(b"list", [0, 45, 0, 0, 0]),
    weight(b"what", [10, 5, 10, 5, 5]),
    weight(b"how", [20, 0, 0, 15, 10]),
];

// ------------------------=
// FUNC: weight
// DESC: Constructs one immutable quantized model feature.
// ------------------=
const fn weight(token: &'static [u8], weights: [i16; CLASS_COUNT]) -> TokenWeight {
    TokenWeight { token, weights }
}

// ------------------------=
// FUNC: local_model_checksum
// DESC: Computes the integrity checksum over the model's quantized parameters.
// ------------------=
pub fn local_model_checksum() -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for feature in &INTENT_WEIGHTS {
        for byte in feature.token {
            hash = (hash ^ *byte as u32).wrapping_mul(0x0100_0193);
        }
        for value in feature.weights {
            for byte in value.to_le_bytes() {
                hash = (hash ^ byte as u32).wrapping_mul(0x0100_0193);
            }
        }
    }
    hash
}

// ------------------------=
// FUNC: model_object_bytes
// DESC: Serializes model identity, adapter, capabilities, requirements, and checksum.
// ------------------=
pub fn model_object_bytes() -> [u8; MODEL_OBJECT_BYTES] {
    let mut out = [0u8; MODEL_OBJECT_BYTES];
    out[..8].copy_from_slice(MODEL_OBJECT_MAGIC);
    out[8..10].copy_from_slice(&1u16.to_le_bytes());
    out[12..16].copy_from_slice(&LOCAL_INTENT_MODEL_ID.to_le_bytes());
    out[16..20].copy_from_slice(&((1u32 << 0) | (1u32 << 6)).to_le_bytes());
    out[20..24].copy_from_slice(&local_model_checksum().to_le_bytes());
    out[24..28].copy_from_slice(&(INTENT_WEIGHTS.len() as u32).to_le_bytes());
    out[28..32].copy_from_slice(&0u32.to_le_bytes());
    out[32..40].copy_from_slice(&(64u64 * 1024).to_le_bytes());
    out[40..44].copy_from_slice(&1u32.to_le_bytes());
    for (index, feature) in INTENT_WEIGHTS.iter().enumerate() {
        let at = FEATURE_OFFSET + index * FEATURE_RECORD_BYTES;
        out[at] = feature.token.len() as u8;
        out[at + 4..at + 4 + feature.token.len()].copy_from_slice(feature.token);
        for (class, value) in feature.weights.iter().enumerate() {
            let weight_at = at + 28 + class * 2;
            out[weight_at..weight_at + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    out
}

// ------------------------=
// FUNC: serialized_model_checksum
// DESC: Computes the parameter checksum directly from a serialized native model object.
// ------------------=
fn serialized_model_checksum(bytes: &[u8]) -> Option<u32> {
    let count = u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?) as usize;
    if count != INTENT_WEIGHTS.len() || FEATURE_OFFSET + count * FEATURE_RECORD_BYTES > bytes.len()
    {
        return None;
    }
    let mut hash = 0x811c_9dc5u32;
    for index in 0..count {
        let at = FEATURE_OFFSET + index * FEATURE_RECORD_BYTES;
        let token_length = bytes[at] as usize;
        if token_length == 0 || token_length > 24 {
            return None;
        }
        for byte in &bytes[at + 4..at + 4 + token_length] {
            hash = (hash ^ *byte as u32).wrapping_mul(0x0100_0193);
        }
        for byte in &bytes[at + 28..at + 28 + CLASS_COUNT * 2] {
            hash = (hash ^ *byte as u32).wrapping_mul(0x0100_0193);
        }
    }
    Some(hash)
}

// ------------------------=
// FUNC: model_object_valid
// DESC: Validates the native model object's identity and quantized-parameter checksum.
// ------------------=
pub fn model_object_valid(bytes: &[u8]) -> bool {
    bytes.len() == MODEL_OBJECT_BYTES
        && &bytes[..8] == MODEL_OBJECT_MAGIC
        && u16::from_le_bytes([bytes[8], bytes[9]]) == 1
        && u32::from_le_bytes(bytes[12..16].try_into().unwrap_or([0; 4])) == LOCAL_INTENT_MODEL_ID
        && u32::from_le_bytes(bytes[20..24].try_into().unwrap_or([0; 4])) == local_model_checksum()
        && serialized_model_checksum(bytes) == Some(local_model_checksum())
}
