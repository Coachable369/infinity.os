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
    pub directory: bool,
}
impl Row {
    pub const EMPTY: Self = Self {
        name: [0; 96],
        len: 0,
        bytes: 0,
        first: 0,
        id: [0; 16],
        container: false,
        directory: false,
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
    pub pool_capacity: u64,
    pub partition: Row,
    pub rows: [Row; LIMIT],
    pub path: [u8; 96],
    pub path_len: usize,
    pub partitions: [Row; LIMIT],
    pub partition_count: usize,
    pub total: u64,
    pub omitted: u64,
    pub incomplete: bool,
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
            pool_capacity: 0,
            partition: Row::EMPTY,
            rows: [Row::EMPTY; LIMIT],
            path: [0; 96],
            path_len: 0,
            partitions: [Row::EMPTY; LIMIT],
            partition_count: 0,
            total: 0,
            omitted: 0,
            incomplete: false,
            inspect: false,
            confirm: false,
            message: b"",
        }
    }
    // ------------------------=
    // FUNC: partition_at
    // DESC: Maps a physical disk byte to its GPT extent, retaining unpartitioned gaps.
    // ------------------=
    pub fn partition_at(&self, byte: u128) -> Option<usize> {
        let rows = if self.depth == 0 { &self.rows[..self.count] } else { &self.partitions[..self.partition_count] };
        rows.iter().position(|r| {
            let first=u128::from(r.first)*512;
            byte>=first && byte<first+u128::from(r.bytes)
        })
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
        row.directory = false;
    }
}

impl DiskView {
    // ------------------------=
    // FUNC: begin_listing
    // DESC: Clears navigation-local totals without discarding the partition map or current directory.
    // ------------------=
    pub fn begin_listing(&mut self) {
        self.count = 0;
        self.total = 0;
        self.omitted = 0;
        self.incomplete = false;
    }
    // ------------------------=
    // FUNC: add_path
    // DESC: Aggregates accessible descendants into immediate directory children with saturating totals.
    // ------------------=
    pub fn add_path(&mut self, path: &[u8], bytes: u64, id: [u8; 16], directory: bool) {
        if path.len() > 96 { self.incomplete = true; return; }
        if directory && !path.ends_with(b"/") {
            if path.len() == 96 { self.incomplete = true; return; }
            let mut normalized = [0;96]; normalized[..path.len()].copy_from_slice(path);
            normalized[path.len()] = b'/';
            self.add_path(&normalized[..path.len()+1], bytes, id, true);
            return;
        }
        let prefix = if self.path_len == 0 { b"/".as_slice() } else { &self.path[..self.path_len] };
        if !path.starts_with(prefix) || path.len() <= prefix.len() { return; }
        let rest = &path[prefix.len()..];
        let slash = rest.iter().position(|&b| b == b'/');
        let end = slash.map_or(path.len(), |n| prefix.len()+n+1);
        let is_dir = slash.is_some() || directory;
        self.total = self.total.saturating_add(bytes);
        if let Some(i) = self.rows[..self.count].iter().position(|r| r.label() == &path[..end]) {
            self.rows[i].bytes = self.rows[i].bytes.saturating_add(bytes);
            return;
        }
        if self.count == LIMIT {
            self.omitted = self.omitted.saturating_add(bytes);
            self.incomplete = true;
            return;
        }
        self.push(&path[..end], bytes, 0, if is_dir { [0;16] } else { id }, false);
        self.rows[self.count-1].directory = is_dir;
    }
    // ------------------------=
    // FUNC: enter
    // DESC: Descends into a partition or child directory; files open a non-destructive inspector.
    // ------------------=
    pub fn enter(&mut self) -> bool {
        if self.selected >= self.count { return false; }
        let row = self.rows[self.selected];
        if self.depth == 0 {
            self.partition_count = self.count.min(self.partitions.len());
            self.partitions[..self.partition_count].copy_from_slice(&self.rows[..self.partition_count]);
            self.partition = row;
            self.depth = 1;
            self.path[0] = b'/'; self.path_len = 1;
        } else if row.directory {
            self.path[..row.len].copy_from_slice(row.label()); self.path_len = row.len;
            if self.path[self.path_len-1] != b'/' {
                if self.path_len == self.path.len() { return false; }
                self.path[self.path_len] = b'/'; self.path_len += 1;
            }
        } else { self.inspect = true; return false; }
        self.selected = 0; self.message = b"";
        true
    }
    // ------------------------=
    // FUNC: back
    // DESC: Returns from inspection, a nested directory or the partition root without losing identity.
    // ------------------=
    pub fn back(&mut self) -> bool {
        if self.inspect { self.inspect = false; return false; }
        if self.depth == 0 { self.active = false; return false; }
        if self.path_len > 1 {
            self.path_len = self.path[..self.path_len-1].iter().rposition(|&b| b == b'/').unwrap_or(0)+1;
        } else {
            self.depth = 0;
        }
        self.selected = 0; self.message = b"";
        true
    }
    // ------------------------=
    // FUNC: sector
    // DESC: Resolves a clockwise 16-bit wheel angle to a real partition or accessible logical-size row.
    // ------------------=
    pub fn sector(&self, angle: u32) -> Option<usize> {
        if self.depth == 0 {
            return self.partition_at(u128::from(self.capacity)*u128::from(angle.min(65535))/65536);
        }
        let at = u128::from(self.total)*u128::from(angle.min(65535))/65536;
        let mut sum = 0u128;
        for (i, row) in self.rows[..self.count].iter().enumerate() {
            sum += u128::from(row.bytes);
            if at < sum { return Some(i); }
        }
        None
    }
}
