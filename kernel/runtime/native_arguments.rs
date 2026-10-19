//! Owned argument snapshots for asynchronously prepared native processes.
#[derive(Clone)]
pub struct Arguments {
    bytes: [u8; 2048],
    ends: [u16; 32],
    count: usize,
}

impl Arguments {
    // ------------------------=
    // FUNC: new
    // DESC: Copies bounded arguments so background launch never borrows an editor or terminal input buffer.
    // ------------------=
    pub fn new(arguments: &[&[u8]]) -> Option<Self> {
        if arguments.is_empty() || arguments.len() > 32 { return None; }
        let mut result = Self { bytes: [0; 2048], ends: [0; 32], count: arguments.len() };
        let mut offset = 0usize;
        for (index, argument) in arguments.iter().enumerate() {
            if argument.contains(&0) { return None; }
            let end = offset.checked_add(argument.len())?;
            if end > result.bytes.len() { return None; }
            result.bytes[offset..end].copy_from_slice(argument);
            result.ends[index] = end as u16;
            offset = end;
        }
        Some(result)
    }

    // ------------------------=
    // FUNC: count
    // DESC: Reports the validated number of launch arguments.
    // ------------------=
    pub fn count(&self) -> usize { self.count }

    // ------------------------=
    // FUNC: get
    // DESC: Returns one stable owned argument without storing self-referential pointers.
    // ------------------=
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        if index >= self.count { return None; }
        let start = if index == 0 { 0 } else { self.ends[index - 1] as usize };
        Some(&self.bytes[start..self.ends[index] as usize])
    }
}
