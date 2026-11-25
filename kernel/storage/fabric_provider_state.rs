//! Shared durable Fabric provider validation, independent of the running executor.
pub const BYTES: usize = 9536;
pub const PATH: &[u8] = b"/system/fabric-provider";

// ------------------------=
// FUNC: validate
// DESC: Rejects malformed provider limits and replay windows before any storage or runtime mutation.
// ------------------=
pub fn validate(state: &[u8]) -> bool {
    if state.len() != BYTES || &state[..8] != b"INFPRV01" { return false; }
    let checksum = state[..BYTES-4].iter().fold(0x811c9dc5u32,
        |value, byte| value.wrapping_mul(16777619) ^ *byte as u32);
    if number(state, BYTES-4, 4) != checksum as u64 || state[8] > 1
        || state[9..12].iter().chain(state[32..40].iter()).chain(state[9512..BYTES-4].iter()).any(|v| *v != 0) {
        return false;
    }
    let cpu = number(state, 12, 2);
    let tasks = number(state, 14, 2);
    let memory = number(state, 16, 8);
    if number(state, 24, 8) == 0 || cpu > 128 || tasks > 8 || memory > 64 * 1024 * 1024
        || (state[8] == 1 && (cpu == 0 || tasks == 0 || memory == 0)) { return false; }
    for index in 0..32 {
        let start = 40 + index * 296;
        let peer = &state[start..start+32];
        if peer.iter().all(|v| *v == 0) {
            if state[start..start+296].iter().any(|v| *v != 0) { return false; }
            continue;
        }
        for previous in 0..index {
            let offset = 40 + previous * 296;
            if &state[offset..offset+32] == peer { return false; }
        }
        let highest = number(state, start+32, 8);
        if highest == 0 || number(state, start+40+(highest%64) as usize*4, 4) == 0 { return false; }
    }
    true
}

// ------------------------=
// FUNC: number
// DESC: Reads a validated fixed-width little-endian checkpoint field without alignment assumptions.
// ------------------=
fn number(state: &[u8], offset: usize, width: usize) -> u64 {
    state[offset..offset+width].iter().enumerate().fold(0, |value,(index,byte)| value | ((*byte as u64) << (index*8)))
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: seal
    // DESC: Produces a valid checksum for structurally corrupted fixtures so validation must inspect the fields.
    // ------------------=
    fn seal(state: &mut [u8; BYTES]) {
        let checksum = state[..BYTES-4].iter().fold(0x811c9dc5u32,
            |value,byte| value.wrapping_mul(16777619) ^ *byte as u32);
        state[BYTES-4..].copy_from_slice(&checksum.to_le_bytes());
    }
    // ------------------------=
    // FUNC: valid
    // DESC: Creates a bounded enabled provider checkpoint with an authenticated peer replay window.
    // ------------------=
    fn valid() -> [u8; BYTES] {
        let mut state = [0; BYTES];
        state[..8].copy_from_slice(b"INFPRV01");
        state[8] = 1; state[12] = 4; state[14] = 2;
        state[16..24].copy_from_slice(&1048576u64.to_le_bytes());
        state[24] = 1;
        state[40..72].fill(7); state[72] = 1; state[84] = 1;
        seal(&mut state);
        state
    }
    // ------------------------=
    // FUNC: validates_limits_padding_and_replay_before_persistence
    // DESC: Exercises valid and malformed binary checkpoints without depending on output strings or executor availability.
    // ------------------=
    #[test]
    fn validates_limits_padding_and_replay_before_persistence() {
        let baseline = valid();
        assert!(validate(&baseline));
        assert!(!validate(&baseline[..BYTES-1]));
        for (offset,value) in [(8,2),(9,1),(12,129),(14,9),(24,0),(32,1),(72,0),(84,0),(9512,1)] {
            let mut state = baseline;
            state[offset] = value; seal(&mut state);
            assert!(!validate(&state), "field {offset}");
        }
        let mut state = baseline;
        state[16..24].copy_from_slice(&(64*1024*1024+1u64).to_le_bytes()); seal(&mut state);
        assert!(!validate(&state));
        let mut state = baseline;
        let peer = state[40..336].to_vec();
        state[336..632].copy_from_slice(&peer); seal(&mut state);
        assert!(!validate(&state));
        let mut state = baseline;
        state[40..72].fill(0); seal(&mut state);
        assert!(!validate(&state));
        let mut state = baseline;
        state[84] ^= 1;
        assert!(!validate(&state));
        let mut state = baseline;
        state[8..24].fill(0); seal(&mut state);
        assert!(validate(&state));
    }
}
