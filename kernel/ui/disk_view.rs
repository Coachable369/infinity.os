//! Session-owned snapshot for the Command Window disk explorer.
pub const LIMIT: usize = 128;
#[derive(Clone, Copy)]
pub struct Row {
    pub name: [u8; 96],
    pub len: usize,
    pub bytes: u64,
    pub first: u64,
    pub id: [u8; 16],
    pub container: bool,
}
impl Row {
    pub const EMPTY: Self = Self {
        name: [0; 96],
        len: 0,
        bytes: 0,
        first: 0,
        id: [0; 16],
        container: false,
    };
    // ------------------------=
    // FUNC: label
    // DESC: Returns the initialized path or partition label.
    // ------------------=
    pub fn label(&self) -> &[u8] {
        &self.name[..self.len]
    }
}
pub struct DiskView {
    pub active: bool,
    pub depth: u8,
    pub selected: usize,
    pub count: usize,
    pub capacity: u64,
    pub used: u64,
    pub partition: Row,
    pub rows: [Row; LIMIT],
    pub inspect: bool,
    pub confirm: bool,
    pub message: &'static [u8],
}
impl DiskView {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty session-local disk view.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            active: false,
            depth: 0,
            selected: 0,
            count: 0,
            capacity: 0,
            used: 0,
            partition: Row::EMPTY,
            rows: [Row::EMPTY; LIMIT],
            inspect: false,
            confirm: false,
            message: b"",
        }
    }
    // ------------------------=
    // FUNC: push
    // DESC: Appends one bounded entry, preserving its complete path for guarded operations.
    // ------------------=
    pub fn push(&mut self, label: &[u8], bytes: u64, first: u64, id: [u8; 16], container: bool) {
        if label.len() > 96 {
            return;
        }
        let slot = if self.count == LIMIT {
            if self.depth == 0 {
                return;
            }
            let Some((index, smallest)) = self.rows.iter().enumerate().min_by_key(|(_, r)| r.bytes)
            else {
                return;
            };
            if bytes <= smallest.bytes {
                return;
            }
            index
        } else {
            let index = self.count;
            self.count += 1;
            index
        };
        let row = &mut self.rows[slot];
        row.name[..label.len()].copy_from_slice(label);
        row.len = label.len();
        row.bytes = bytes;
        row.first = first;
        row.id = id;
        row.container = container;
    }
}
