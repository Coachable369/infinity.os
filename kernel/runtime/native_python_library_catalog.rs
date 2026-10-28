//! Bounded import-root discovery from committed package bytes, never package execution.
#[path = "native_python_package_limits.rs"] mod limits;
pub const MAX_MODULES: usize = 48;
pub const MAX_NAME: usize = 63;
pub const CONTEXT_BYTES: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Catalog { names: [[u8; MAX_NAME]; MAX_MODULES], lengths: [u8; MAX_MODULES], pub count: usize, pub omitted: bool }
impl Catalog {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded import directory without allocating archive-sized storage.
    // ------------------=
    pub const fn new() -> Self { Self { names: [[0; MAX_NAME]; MAX_MODULES], lengths: [0; MAX_MODULES], count: 0, omitted: false } }
    // ------------------------=
    // FUNC: module
    // DESC: Returns one sanitized import identifier rather than untrusted package metadata prose.
    // ------------------=
    pub fn module(&self, index: usize) -> Option<&[u8]> { (index < self.count).then(|| &self.names[index][..self.lengths[index] as usize]) }
    // ------------------------=
    // FUNC: add_path
    // DESC: Recognizes Python source and native extension roots, excluding metadata, resources and path traversal.
    // ------------------=
    fn add_path(&mut self, path: &[u8]) {
        let Some(relative) = path.strip_prefix(b"/packages/") else { return; };
        if !relative.ends_with(b".py") && !relative.ends_with(b".so") { return; }
        let root = relative.split(|b| *b == b'/').next().unwrap_or(b"");
        let name = if relative.contains(&b'/') { root } else { root.split(|b| *b == b'.').next().unwrap_or(b"") };
        if name.is_empty() || name.len() > MAX_NAME || !(name[0].is_ascii_alphabetic() || name[0] == b'_')
            || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') || name == b"__pycache__"
            || [b"False".as_slice(), b"None", b"True", b"and", b"as", b"assert", b"async", b"await", b"break", b"class", b"continue", b"def", b"del", b"elif", b"else", b"except", b"finally", b"for", b"from", b"global", b"if", b"import", b"in", b"is", b"lambda", b"nonlocal", b"not", b"or", b"pass", b"raise", b"return", b"try", b"while", b"with", b"yield"].contains(&name) { return; }
        if (0..self.count).any(|index| self.module(index) == Some(name)) { return; }
        if self.count == MAX_MODULES { self.omitted = true; return; }
        self.names[self.count][..name.len()].copy_from_slice(name); self.lengths[self.count] = name.len() as u8; self.count += 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormattedContext {
    pub length: usize,
    pub shown: usize,
    pub omitted: bool,
    pub directory_available: bool,
    pub names_start: usize,
    pub names_length: usize,
}

// ------------------------=
// FUNC: format_context
// DESC: Formats the same bounded trusted library context for production and probes; unavailable authority never publishes cached or assumed package names.
// ------------------=
pub fn format_context(catalog: Option<&Catalog>, output: &mut [u8]) -> Option<FormattedContext> {
    let prefix = b"\nINSTALLED THIRD-PARTY ROOTS:\n";
    let suffix = b"\nUse compatible native stdlib as needed; suggestions are not exhaustive. Third-party imports require listed roots; native API compatibility still applies. Never assume packages, install or download them.\n";
    let empty = b"EMPTY: no additional packages installed.";
    let unavailable = b"UNAVAILABLE: use verified defaults only.";
    let truncated = b" (bounded listing)";
    let capacity = output.len().min(CONTEXT_BYTES);
    let status_bytes = empty.len().max(unavailable.len()).max(truncated.len());
    if capacity < prefix.len() + suffix.len() + status_bytes { return None; }
    let limit = capacity - suffix.len() - status_bytes;
    let mut n = prefix.len(); output[..n].copy_from_slice(prefix);
    let mut result = FormattedContext { length: 0, shown: 0, omitted: false,
        directory_available: catalog.is_some(), names_start: n, names_length: 0 };
    let status = if let Some(catalog) = catalog {
        while let Some(name) = catalog.module(result.shown) {
            let separator = usize::from(result.shown > 0);
            if n + separator + name.len() > limit { break; }
            if separator != 0 { output[n] = b','; n += 1; }
            output[n..n + name.len()].copy_from_slice(name); n += name.len(); result.shown += 1;
        }
        result.names_length = n - result.names_start;
        result.omitted = result.shown < catalog.count || catalog.omitted;
        if catalog.count == 0 { empty.as_slice() } else if result.omitted { truncated.as_slice() } else { b"".as_slice() }
    } else { unavailable.as_slice() };
    output[n..n + status.len()].copy_from_slice(status); n += status.len();
    output[n..n + suffix.len()].copy_from_slice(suffix); result.length = n + suffix.len();
    Some(result)
}

pub struct Reader { catalog: Catalog, buffer: [u8; 255], have: usize, need: usize,
    stage: u8, entries: usize, seen: usize, size: usize, directory: bool, skip: usize, total: usize }
impl Reader {
    // ------------------------=
    // FUNC: new
    // DESC: Starts an incremental IPKG reader with a fixed 255-byte metadata buffer.
    // ------------------=
    pub const fn new() -> Self { Self { catalog: Catalog::new(), buffer: [0; 255], have: 0, need: 16,
        stage: 0, entries: 0, seen: 0, size: 0, directory: false, skip: 0, total: 0 } }
    // ------------------------=
    // FUNC: feed
    // DESC: Validates archive framing across arbitrary chunks while discarding file contents and retaining only import roots.
    // ------------------=
    pub fn feed(&mut self, mut bytes: &[u8]) -> Result<(), ()> {
        self.total = self.total.checked_add(bytes.len()).filter(|n| *n <= limits::ARCHIVE_BYTES).ok_or(())?;
        while !bytes.is_empty() {
            if self.skip > 0 { let n = bytes.len().min(self.skip); self.skip -= n; bytes = &bytes[n..]; continue; }
            if self.stage == 1 && self.seen == self.entries { return Err(()); }
            let n = bytes.len().min(self.need - self.have);
            self.buffer[self.have..self.have + n].copy_from_slice(&bytes[..n]); self.have += n; bytes = &bytes[n..];
            if self.have != self.need { continue; }
            if self.stage == 0 {
                if &self.buffer[..8] != b"IPKG0001" || self.buffer[12..16] != [0; 4] { return Err(()); }
                self.entries = u32::from_le_bytes(self.buffer[8..12].try_into().unwrap()) as usize;
                if self.entries > limits::ENTRIES - 4 { return Err(()); }
                self.stage = 1; self.need = 8;
            } else if self.stage == 1 {
                let length = u16::from_le_bytes(self.buffer[..2].try_into().unwrap()) as usize;
                let flags = u16::from_le_bytes(self.buffer[2..4].try_into().unwrap());
                self.size = u32::from_le_bytes(self.buffer[4..8].try_into().unwrap()) as usize;
                if !(11..=255).contains(&length) || flags > 3 || self.size > limits::FILE_BYTES || flags & 1 != 0 && self.size != 0 { return Err(()); }
                self.directory = flags & 1 != 0; self.stage = 2; self.need = length;
            } else {
                let path = &self.buffer[..self.need];
                if !path.starts_with(b"/packages/") || core::str::from_utf8(path).is_err() || path.contains(&0)
                    || path[10..].split(|b| *b == b'/').any(|part| part.is_empty() || part == b"." || part == b"..") { return Err(()); }
                if !self.directory { self.catalog.add_path(path); }
                self.skip = self.size; self.seen += 1; self.stage = 1; self.need = 8;
            }
            self.have = 0;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: finish
    // DESC: Publishes names only after the complete declared archive has been consumed without trailing or truncated entries.
    // ------------------=
    pub fn finish(self) -> Result<Catalog, ()> {
        if self.stage != 1 || self.have != 0 || self.skip != 0 || self.seen != self.entries { Err(()) } else { Ok(self.catalog) }
    }
}
