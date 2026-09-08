//! Bounded raw device queue; no UI dispatch, allocation, or event coalescing.
pub const CAPACITY: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Byte { pub status: u8, pub value: u8 }
pub struct Buffer { bytes: [Byte; CAPACITY], head: usize, len: usize }
impl Buffer {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a fixed-size FIFO preserving keyboard and mouse byte ordering.
    // ------------------=
    pub const fn new() -> Self { Self { bytes: [Byte { status: 0, value: 0 }; CAPACITY], head: 0, len: 0 } }
    // ------------------------=
    // FUNC: full
    // DESC: Prevents consuming a device byte when no software queue slot is available.
    // ------------------=
    pub fn full(&self) -> bool { self.len == CAPACITY }
    // ------------------------=
    // FUNC: push
    // DESC: Appends without overwriting any queued make, break, prefix, or pointer byte.
    // ------------------=
    pub fn push(&mut self, value: Byte) -> bool {
        if self.full() { return false; }
        self.bytes[(self.head + self.len) % CAPACITY] = value; self.len += 1; true
    }
    // ------------------------=
    // FUNC: peek
    // DESC: Inspects the next source tag without consuming a different device's byte.
    // ------------------=
    pub fn peek(&self) -> Option<Byte> { if self.len == 0 { None } else { Some(self.bytes[self.head]) } }
    // ------------------------=
    // FUNC: pop
    // DESC: Removes exactly one oldest byte with bounded ring wraparound.
    // ------------------=
    pub fn pop(&mut self) -> Option<Byte> {
        let result = self.peek()?; self.head = (self.head + 1) % CAPACITY; self.len -= 1; Some(result)
    }
}

// ------------------------=
// FUNC: capture_fifo_preserves_edges_and_applies_backpressure
// DESC: Exercises mixed-source ordering, full-buffer rejection, wraparound, and repeated draining without lost releases.
// ------------------=
#[test]
fn capture_fifo_preserves_edges_and_applies_backpressure() {
    let mut queue = Buffer::new();
    for cycle in 0..8 {
        for value in 0..CAPACITY { assert!(queue.push(Byte { status: (value % 2) as u8 * 32 + 1, value: (value + cycle) as u8 })); }
        assert!(queue.full()); assert!(!queue.push(Byte { status: 1, value: 0 }));
        for value in 0..CAPACITY { assert_eq!(queue.pop(), Some(Byte { status: (value % 2) as u8 * 32 + 1, value: (value + cycle) as u8 })); }
        assert_eq!(queue.pop(), None);
    }
}
