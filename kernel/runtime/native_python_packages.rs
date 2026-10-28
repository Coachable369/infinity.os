//! Private, bounded package filesystem. Mutations are staged in RAM and only a
//! successful, still-authorized pip process may publish the encoded archive.
use super::native_memory::{check_user_range, copy_from_user, copy_to_user, PageTable};

#[path = "native_python_package_limits.rs"] pub mod limits;
pub const MAX_ARCHIVE: usize = limits::ARCHIVE_BYTES;
pub const MAX_FILES: usize = limits::ENTRIES;
pub const MAX_PATH: usize = 255;
const ARENA: usize = limits::STAGING_BYTES;
const MAX_FILE: usize = limits::FILE_BYTES;
const MAGIC: &[u8; 8] = b"IPKG0001";
const DATA_MAGIC: &[u8; 8] = b"IDAT0001";
type Result<T> = core::result::Result<T, i64>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipMode { ReadOnly, Install, Uninstall }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError { Command, Confirmation, Option, Input, SelfModification }

// ------------------------=
// FUNC: admit_pip
// DESC: Validates user arguments before admitting real pip into a version-fenced wheel transaction with no insecure TLS or target overrides.
// ------------------=
pub fn admit_pip(arguments: &[&[u8]]) -> core::result::Result<PipMode, PolicyError> {
    let command = arguments.first().copied().unwrap_or(b"--help");
    let mode = match command {
        b"install" => PipMode::Install, b"uninstall" => PipMode::Uninstall,
        b"list" | b"show" | b"check" | b"freeze" | b"help" | b"--help" | b"-h" | b"--version" | b"-V" => PipMode::ReadOnly,
        _ => return Err(PolicyError::Command),
    };
    if mode == PipMode::Uninstall && !arguments.contains(&b"-y".as_slice()) && !arguments.contains(&b"--yes".as_slice()) {
        return Err(PolicyError::Confirmation);
    }
    for value in arguments.iter().skip(1) {
        if value.starts_with(b"-") {
            let allowed = matches!(*value, b"--help" | b"-h")
                || mode == PipMode::Install && matches!(*value, b"--upgrade" | b"-U" | b"--force-reinstall" | b"--no-deps" | b"--ignore-installed")
                || mode == PipMode::Uninstall && matches!(*value, b"-y" | b"--yes")
                || command == b"list" && matches!(*value, b"--format=json" | b"--format=columns" | b"--format=freeze")
                || command == b"freeze" && *value == b"--all";
            if !allowed { return Err(PolicyError::Option); }
        } else if !(mode == PipMode::Install && (value.starts_with(b"/") && package_archive(value)
                || value.starts_with(b"https://") && value.split(|b| *b == b'#').next().is_some_and(package_archive)))
            && (value.is_empty() || !value.iter().all(|b| b.is_ascii_alphanumeric() || b"._-=<>!~[],*".contains(b))) {
            return Err(PolicyError::Input);
        }
        let name = value.rsplit(|b| *b == b'/').next().unwrap_or(value);
        let project = if value.starts_with(b"/") || value.starts_with(b"https://") { name.split(|b| *b == b'-').next().unwrap_or(name) }
            else { name.split(|b| b"=<>!~[".contains(b)).next().unwrap_or(name) };
        if mode != PipMode::ReadOnly && (project.eq_ignore_ascii_case(b"pip") || project.eq_ignore_ascii_case(b"tzdata")) {
            return Err(PolicyError::SelfModification);
        }
    }
    Ok(mode)
}

// ------------------------=
// FUNC: package_archive
// DESC: Recognizes supported wheel and source archive containers without granting directory or requirement-file execution.
// ------------------=
pub fn package_archive(path: &[u8]) -> bool {
    [b".whl".as_slice(), b".zip", b".tar", b".tar.gz", b".tar.bz2", b".tar.xz", b".tgz", b".txz"]
        .iter().any(|suffix| path.ends_with(suffix))
}

#[derive(Clone, Copy)]
struct Entry { path: [u8; MAX_PATH], length: usize, offset: usize, size: usize,
    capacity: usize, directory: bool, readonly: bool, used: bool, executable: bool, identity: u64 }
impl Entry { const EMPTY: Self = Self { path: [0; MAX_PATH], length: 0, offset: 0, size: 0,
    capacity: 0, directory: false, readonly: true, used: false, executable: false, identity: 0 }; }
#[derive(Clone, Copy)]
struct Handle { token: u64, entry: usize, position: usize, flags: u32 }
#[derive(Clone, Copy)]
struct Directory { token: u64, path: [u8; MAX_PATH], length: usize, next: usize }

pub struct Client {
    handles: [Option<Handle>; 128], directories: [Option<Directory>; 64],
    next_token: u64, cwd: [u8; MAX_PATH], cwd_len: usize,
}
impl Client {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a private descriptor and working-directory view with no inherited handles.
    // ------------------=
    pub const fn new() -> Self {
        Self { handles: [None; 128], directories: [None; 64], next_token: 3, cwd: [0; MAX_PATH], cwd_len: 0 }
    }
}

pub struct Vfs {
    entries: [Entry; MAX_FILES], bytes: [u8; ARENA], used: usize,
    handles: [Option<Handle>; 128], directories: [Option<Directory>; 64],
    next_token: u64, next_identity: u64, writable: bool, changed: bool,
    data_granted: bool, data_changed: bool, data_failed: bool,
    cwd: [u8; MAX_PATH], cwd_len: usize,
    parked_pins: [bool; MAX_FILES], child_active: bool,
}

// ------------------------=
// FUNC: under
// DESC: Tests a path component boundary rather than accepting shared string prefixes.
// ------------------=
fn under(path: &[u8], root: &[u8]) -> bool {
    path == root || path.starts_with(root) && path.get(root.len()) == Some(&b'/')
}

// ------------------------=
// FUNC: normalize
// DESC: Canonicalizes a bounded path without allowing traversal out of a granted top-level root.
// ------------------=
fn normalize(cwd: &[u8], path: &[u8], out: &mut [u8; MAX_PATH]) -> Result<usize> {
    if path.is_empty() || path.contains(&0) || core::str::from_utf8(path).is_err() { return Err(-22); }
    let mut length = 1; out[0] = b'/';
    let base = if path[0] == b'/' { b"".as_slice() } else { cwd };
    for part in base.split(|b| *b == b'/').chain(path.split(|b| *b == b'/')) {
        if part.is_empty() || part == b"." { continue; }
        if part == b".." {
            let slash = out[..length].iter().rposition(|b| *b == b'/').unwrap_or(0);
            if slash == 0 { return Err(-13); }
            length = slash; continue;
        }
        let start = length + usize::from(length > 1);
        let end = start.checked_add(part.len()).filter(|n| *n <= MAX_PATH).ok_or(-36i64)?;
        if length > 1 { out[length] = b'/'; }
        out[start..end].copy_from_slice(part); length = end;
    }
    Ok(length)
}

impl Vfs {
    // ------------------------=
    // FUNC: new
    // DESC: Creates static storage without allocating a large temporary kernel stack object.
    // ------------------=
    pub const fn new() -> Self {
        Self { entries: [Entry::EMPTY; MAX_FILES], bytes: [0; ARENA], used: 0,
            handles: [None; 128], directories: [None; 64], next_token: 3,
            next_identity: 1, writable: false, changed: false, cwd: [0; MAX_PATH], cwd_len: 0,
            data_granted: false, data_changed: false, data_failed: false,
            parked_pins: [false; MAX_FILES], child_active: false }
    }
    // ------------------------=
    // FUNC: reset
    // DESC: Erases all previous-user data and installs only the three explicit private filesystem roots.
    // ------------------=
    pub fn reset(&mut self, writable: bool) {
        self.bytes.fill(0); self.entries.fill(Entry::EMPTY); self.used = 0;
        self.handles.fill(None); self.directories.fill(None); self.next_token = 3;
        self.next_identity = 1; self.writable = writable; self.changed = false;
        self.data_granted = false; self.data_changed = false; self.data_failed = false;
        self.parked_pins.fill(false); self.child_active = false;
        self.cwd.fill(0); self.cwd[0] = b'/'; self.cwd_len = 1;
        for path in [b"/".as_slice(), b"/packages", b"/work", b"/tmp"] {
            let _ = self.insert(path, true, path == b"/" || path == b"/work", b"");
        }
    }
    // ------------------------=
    // FUNC: swap_client
    // DESC: Swaps only process-local descriptor cursors and cwd, leaving the staged transaction and its permissions unchanged.
    // ------------------=
    fn swap_client(&mut self, client: &mut Client) {
        core::mem::swap(&mut self.handles, &mut client.handles);
        core::mem::swap(&mut self.directories, &mut client.directories);
        core::mem::swap(&mut self.next_token, &mut client.next_token);
        core::mem::swap(&mut self.cwd, &mut client.cwd);
        core::mem::swap(&mut self.cwd_len, &mut client.cwd_len);
    }
    // ------------------------=
    // FUNC: enter_child
    // DESC: Parks parent handles and cwd, starts a unique child descriptor epoch and pins parent-open identities against removal or subtree rename.
    // ------------------=
    pub fn enter_child(&mut self, client: &mut Client, generation: u64) -> Result<()> {
        if self.child_active || generation == 0 || generation >= 1u64 << 31 { return Err(-16); }
        for handle in self.handles.iter().flatten() { self.parked_pins[handle.entry] = true; }
        for directory in self.directories.iter().flatten() {
            if let Some(index) = self.find(&directory.path[..directory.length]) { self.parked_pins[index] = true; }
        }
        if let Some(index) = self.find(&self.cwd[..self.cwd_len]) { self.parked_pins[index] = true; }
        client.handles.fill(None); client.directories.fill(None);
        client.next_token = (generation << 32) | 3;
        client.cwd.fill(0); client.cwd[0] = b'/'; client.cwd_len = 1;
        self.swap_client(client); self.child_active = true; Ok(())
    }
    // ------------------------=
    // FUNC: leave_child
    // DESC: Restores the parked parent without accepting any child descriptor tokens or inheriting child cwd changes.
    // ------------------=
    pub fn leave_child(&mut self, client: &mut Client) {
        if !self.child_active { return; }
        self.swap_client(client); client.handles.fill(None); client.directories.fill(None);
        client.cwd.fill(0); client.cwd_len = 0;
        self.parked_pins.fill(false); self.child_active = false;
    }
    // ------------------------=
    // FUNC: dirty
    // DESC: Reports whether persistent package content changed during this authorized transaction.
    // ------------------=
    pub fn dirty(&self) -> bool { self.changed }
    // ------------------------=
    // FUNC: grant_data
    // DESC: Mounts a distinct app-private data root only for a direct, non-package process authorized by its native launcher.
    // ------------------=
    pub fn grant_data(&mut self, archive: &[u8]) -> Result<()> {
        if self.writable || self.child_active || self.data_granted { return Err(-13); }
        self.insert(b"/data", true, false, b"")?;
        if !archive.is_empty() { self.load_root(archive, b"/data", DATA_MAGIC)?; }
        self.data_granted = true; self.data_changed = false;
        self.cwd.fill(0); self.cwd[..5].copy_from_slice(b"/data"); self.cwd_len = 5;
        Ok(())
    }
    // ------------------------=
    // FUNC: data_dirty
    // DESC: Reports source-scoped changes independently of the package transaction.
    // ------------------=
    pub fn data_dirty(&self) -> bool { self.data_granted && self.data_changed }
    // ------------------------=
    // FUNC: data_checkpointed
    // DESC: Clears dirty state only after durable storage acknowledges the exact archive revision.
    // ------------------=
    pub fn data_checkpointed(&mut self) { self.data_changed = false; }
    // ------------------------=
    // FUNC: data_checkpoint_failed
    // DESC: Stops further data mutations after a failed durable checkpoint instead of continuing with uncommittable state.
    // ------------------=
    pub fn data_checkpoint_failed(&mut self) { self.data_failed = true; }
    // ------------------------=
    // FUNC: data_barrier
    // DESC: Requires durable publication after successful sync, close and namespace operations, including SQLite journal deletion.
    // ------------------=
    pub fn data_barrier(&self, operation: u64, result: i64) -> bool {
        self.data_dirty() && !self.child_active && result == 0
            && matches!(operation, 0x120f | 0x1217 | 0x1218 | 0x1233..=0x1237)
    }
    // ------------------------=
    // FUNC: encode_data
    // DESC: Serializes app data only; installed code, packages, inputs and scratch never enter the checkpoint.
    // ------------------=
    pub fn encode_data(&self, out: &mut [u8]) -> Result<usize> {
        if !self.data_granted || self.child_active || self.data_failed { return Err(-13); }
        self.encode_root(out, b"/data", DATA_MAGIC)
    }
    // ------------------------=
    // FUNC: native_extension_nth
    // DESC: Borrows one exact staged package extension for validation and stable snapshotting, never ambient native filesystem paths.
    // ------------------=
    pub fn native_extension_nth(&self, index: usize) -> Option<(&[u8], &[u8])> {
        let entry = self.entries.iter().filter(|entry| entry.used && !entry.directory
            && entry.path[..entry.length].starts_with(b"/packages/")
            && entry.path[..entry.length].ends_with(b".so")).nth(index)?;
        Some((&entry.path[..entry.length], &self.bytes[entry.offset..entry.offset + entry.size]))
    }
    // ------------------------=
    // FUNC: canonical
    // DESC: Resolves relative paths against the process-private working directory.
    // ------------------=
    fn canonical(&self, path: &[u8]) -> Result<([u8; MAX_PATH], usize)> {
        let mut out = [0; MAX_PATH]; let length = normalize(&self.cwd[..self.cwd_len], path, &mut out)?;
        Ok((out, length))
    }
    // ------------------------=
    // FUNC: find
    // DESC: Resolves one exact staged filesystem identity.
    // ------------------=
    fn find(&self, path: &[u8]) -> Option<usize> {
        self.entries.iter().position(|e| e.used && &e.path[..e.length] == path)
    }
    // ------------------------=
    // FUNC: mutable
    // DESC: Grants temporary writes only in guest RAM and package writes only to an explicitly admitted pip job.
    // ------------------=
    fn mutable(&self, path: &[u8]) -> bool {
        if let Some(relative) = path.strip_prefix(b"/packages/".as_slice()) {
            let first = relative.split(|b| *b == b'/').next().unwrap_or(b"");
            for protected in [b"pip".as_slice(), b"tzdata"] {
                if first.eq_ignore_ascii_case(protected) || first.len() > protected.len()
                    && first[..protected.len()].eq_ignore_ascii_case(protected)
                    && first[protected.len()] == b'-' && first.ends_with(b".dist-info") { return false; }
            }
        }
        under(path, b"/tmp") || self.writable && under(path, b"/packages")
            || self.data_granted && !self.child_active && !self.data_failed && under(path, b"/data")
    }
    // ------------------------=
    // FUNC: parent
    // DESC: Requires an existing directory parent before creating an entry.
    // ------------------=
    fn parent(&self, path: &[u8]) -> Result<()> {
        let slash = path.iter().rposition(|b| *b == b'/').ok_or(-22i64)?;
        let parent = if slash == 0 { b"/".as_slice() } else { &path[..slash] };
        let index = self.find(parent).ok_or(-2i64)?;
        if !self.entries[index].directory { return Err(-20); } Ok(())
    }
    // ------------------------=
    // FUNC: compact
    // DESC: Reclaims unreachable arena allocations in-place while preserving every live handle's entry identity.
    // ------------------=
    fn compact(&mut self) {
        let mut selected = [false; MAX_FILES]; let mut next = 0;
        for _ in 0..MAX_FILES {
            let candidate = (0..MAX_FILES).filter(|i| !selected[*i] && self.entries[*i].used && self.entries[*i].capacity > 0)
                .min_by_key(|i| self.entries[*i].offset);
            let Some(index) = candidate else { break; }; selected[index] = true;
            let entry = &mut self.entries[index];
            self.bytes.copy_within(entry.offset..entry.offset + entry.size, next);
            entry.offset = next; entry.capacity = entry.size; next += entry.size;
        }
        self.bytes[next..self.used].fill(0); self.used = next;
    }
    // ------------------------=
    // FUNC: capacity
    // DESC: Reserves a bounded file extent without exposing discarded data through holes or growth.
    // ------------------=
    fn capacity(&mut self, index: usize, size: usize) -> Result<()> {
        if size > MAX_FILE { return Err(-27); }
        if size <= self.entries[index].capacity { return Ok(()); }
        let capacity = size.max(self.entries[index].capacity.saturating_mul(2)).max(256).min(MAX_FILE);
        if self.used.checked_add(capacity).is_none_or(|end| end > ARENA) { self.compact(); }
        if self.used.checked_add(capacity).is_none_or(|end| end > ARENA) { return Err(-28); }
        let entry = &mut self.entries[index];
        self.bytes.copy_within(entry.offset..entry.offset + entry.size, self.used);
        self.bytes[self.used + entry.size..self.used + capacity].fill(0);
        entry.offset = self.used; entry.capacity = capacity; self.used += capacity; Ok(())
    }
    // ------------------------=
    // FUNC: insert
    // DESC: Inserts a validated new entry using a fresh identity and caller-selected immutable ownership.
    // ------------------=
    fn insert(&mut self, path: &[u8], directory: bool, readonly: bool, bytes: &[u8]) -> Result<usize> {
        if path.len() > MAX_PATH || self.find(path).is_some() { return Err(-17); }
        if bytes.len() > MAX_FILE { return Err(-27); }
        let index = self.entries.iter().position(|e| !e.used).ok_or(-28i64)?;
        let identity = self.next_identity; self.next_identity = identity.checked_add(1).ok_or(-75i64)?;
        let mut entry = Entry::EMPTY; entry.path[..path.len()].copy_from_slice(path); entry.length = path.len();
        entry.directory = directory; entry.readonly = readonly; entry.used = true; entry.identity = identity;
        self.entries[index] = entry;
        if let Err(error) = self.capacity(index, bytes.len()) { self.entries[index] = Entry::EMPTY; return Err(error); }
        let offset = self.entries[index].offset; self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
        self.entries[index].size = bytes.len(); Ok(index)
    }
    // ------------------------=
    // FUNC: mount_readonly
    // DESC: Makes one explicitly authorized input visible without granting access to neighboring native objects.
    // ------------------=
    pub fn mount_readonly(&mut self, path: &[u8], bytes: &[u8]) -> Result<()> {
        let (name, length) = self.canonical(path)?; let path = &name[..length];
        if under(path, b"/packages") || under(path, b"/tmp") || under(path, b"/data") { return Err(-13); }
        for slash in 1..length {
            if path[slash] == b'/' && self.find(&path[..slash]).is_none() {
                self.insert(&path[..slash], true, true, b"")?;
            }
        }
        self.parent(path)?; self.insert(path, false, true, bytes)?; Ok(())
    }
    // ------------------------=
    // FUNC: mkdir
    // DESC: Creates only a staged directory inside this process's mutable roots.
    // ------------------=
    pub fn mkdir(&mut self, path: &[u8]) -> Result<()> {
        let (name, length) = self.canonical(path)?; let path = &name[..length];
        if !self.mutable(path) { return Err(-13); }
        self.parent(path)?; self.insert(path, true, false, b"")?;
        self.changed |= under(path, b"/packages"); self.data_changed |= under(path, b"/data"); Ok(())
    }
    // ------------------------=
    // FUNC: put
    // DESC: Publishes bounded bytes into staged storage, never directly into the ObjectStore.
    // ------------------=
    pub fn put(&mut self, path: &[u8], bytes: &[u8]) -> Result<()> {
        let (name, length) = self.canonical(path)?; let path = &name[..length];
        if !self.mutable(path) { return Err(-13); }
        let index = if let Some(index) = self.find(path) {
            if self.entries[index].readonly { return Err(-13); }
            if self.entries[index].directory { return Err(-21); } index
        } else { self.parent(path)?; self.insert(path, false, false, b"")? };
        self.capacity(index, bytes.len())?;
        let entry = &mut self.entries[index]; self.bytes[entry.offset..entry.offset + bytes.len()].copy_from_slice(bytes);
        entry.size = bytes.len(); self.changed |= under(path, b"/packages"); self.data_changed |= under(path, b"/data"); Ok(())
    }
    // ------------------------=
    // FUNC: read
    // DESC: Borrows the exact staged file bytes for typed validation and archive consumers.
    // ------------------=
    pub fn read(&self, path: &[u8]) -> Result<&[u8]> {
        let (name, length) = self.canonical(path)?;
        if self.child_active && under(&name[..length], b"/data") { return Err(-13); }
        let entry = &self.entries[self.find(&name[..length]).ok_or(-2i64)?];
        if entry.directory { return Err(-21); } Ok(&self.bytes[entry.offset..entry.offset + entry.size])
    }
    // ------------------------=
    // FUNC: remove
    // DESC: Removes only a mutable file or empty directory with exact POSIX node-type checks.
    // ------------------=
    pub fn remove(&mut self, path: &[u8], directory: bool) -> Result<()> {
        let (name, length) = self.canonical(path)?; let path = &name[..length];
        if !self.mutable(path) || matches!(path, b"/packages" | b"/tmp" | b"/data") { return Err(-13); }
        let index = self.find(path).ok_or(-2i64)?; let entry = self.entries[index];
        if entry.readonly { return Err(-13); }
        if entry.directory != directory { return Err(if directory { -20 } else { -21 }); }
        if directory && self.entries.iter().any(|e| e.used && e.length > length && under(&e.path[..e.length], path)) { return Err(-39); }
        if self.parked_pins[index] || self.handles.iter().flatten().any(|h| h.entry == index)
            || self.directories.iter().flatten().any(|d| &d.path[..d.length] == path)
            || &self.cwd[..self.cwd_len] == path { return Err(-16); }
        self.entries[index] = Entry::EMPTY; self.changed |= under(path, b"/packages"); self.data_changed |= under(path, b"/data"); Ok(())
    }
    // ------------------------=
    // FUNC: rename
    // DESC: Preflights a whole subtree rename before changing any path, rejecting cross-grant escapes and collisions.
    // ------------------=
    pub fn rename(&mut self, source: &[u8], target: &[u8]) -> Result<()> {
        let (from, fl) = self.canonical(source)?; let from = &from[..fl];
        let (to, tl) = self.canonical(target)?; let to = &to[..tl];
        if !self.mutable(from) || !self.mutable(to) || matches!(from, b"/packages" | b"/tmp" | b"/data")
            || matches!(to, b"/packages" | b"/tmp" | b"/data") { return Err(-13); }
        if under(from, b"/data") != under(to, b"/data") { return Err(-18); }
        let source = self.find(from).ok_or(-2i64)?;
        if from == to { return Ok(()); }
        if self.entries.iter().enumerate().any(|(index, entry)| self.parked_pins[index]
            && entry.used && (under(&entry.path[..entry.length], from) || under(&entry.path[..entry.length], to))) { return Err(-16); }
        if under(to, from) { return Err(-22); } self.parent(to)?;
        let replacement = self.find(to);
        if let Some(index) = replacement {
            if self.entries[index].readonly { return Err(-13); }
            if self.entries[index].directory != self.entries[source].directory { return Err(-21); }
            if self.entries[index].directory && self.entries.iter().any(|e| e.used && e.length > tl && under(&e.path[..e.length], to)) { return Err(-39); }
            if self.handles.iter().flatten().any(|h| h.entry == index) { return Err(-16); }
        }
        for entry in self.entries.iter().filter(|e| e.used && under(&e.path[..e.length], from)) {
            if entry.readonly || tl + entry.length - fl > MAX_PATH { return Err(-36); }
            let mut candidate = [0; MAX_PATH]; candidate[..tl].copy_from_slice(to);
            candidate[tl..tl + entry.length - fl].copy_from_slice(&entry.path[fl..entry.length]);
            if self.find(&candidate[..tl + entry.length - fl]).is_some_and(|i| Some(i) != replacement) { return Err(-17); }
        }
        if let Some(index) = replacement { self.entries[index] = Entry::EMPTY; }
        for entry in self.entries.iter_mut().filter(|e| e.used && under(&e.path[..e.length], from)) {
            entry.path.copy_within(fl..entry.length, tl); entry.path[..tl].copy_from_slice(to); entry.length = tl + entry.length - fl;
        }
        for directory in self.directories.iter_mut().flatten().filter(|d| under(&d.path[..d.length], from)) {
            directory.path.copy_within(fl..directory.length, tl); directory.path[..tl].copy_from_slice(to);
            directory.length = tl + directory.length - fl;
        }
        if under(&self.cwd[..self.cwd_len], from) {
            self.cwd.copy_within(fl..self.cwd_len, tl); self.cwd[..tl].copy_from_slice(to); self.cwd_len = tl + self.cwd_len - fl;
        }
        self.changed |= under(from, b"/packages") || under(to, b"/packages");
        self.data_changed |= under(from, b"/data"); Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Serializes only persistent package entries with explicit versioned fields; scratch and wheel inputs are omitted.
    // ------------------=
    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        self.encode_root(out, b"/packages", MAGIC)
    }
    // ------------------------=
    // FUNC: encode_root
    // DESC: Encodes one versioned granted root without including any other private filesystem mount.
    // ------------------=
    fn encode_root(&self, out: &mut [u8], root: &[u8], magic: &[u8; 8]) -> Result<usize> {
        if out.len() < 16 { return Err(-28); }
        out[..8].copy_from_slice(magic); out[8..16].fill(0);
        let mut length = 16; let mut count = 0u32;
        for entry in self.entries.iter().filter(|e| e.used && e.length > root.len() && under(&e.path[..e.length], root)) {
            let size = if entry.directory { 0 } else { entry.size };
            let end = length + 8 + entry.length + size;
            if end > out.len().min(MAX_ARCHIVE) { return Err(-28); }
            out[length..length + 2].copy_from_slice(&(entry.length as u16).to_le_bytes());
            let flags = u16::from(entry.directory) | u16::from(entry.executable) << 1;
            out[length + 2..length + 4].copy_from_slice(&flags.to_le_bytes());
            out[length + 4..length + 8].copy_from_slice(&(size as u32).to_le_bytes());
            out[length + 8..length + 8 + entry.length].copy_from_slice(&entry.path[..entry.length]);
            out[length + 8 + entry.length..end].copy_from_slice(&self.bytes[entry.offset..entry.offset + size]);
            length = end; count += 1;
        }
        out[8..12].copy_from_slice(&count.to_le_bytes()); Ok(length)
    }
    // ------------------------=
    // FUNC: load_archive
    // DESC: Rejects malformed, duplicate, out-of-root or over-capacity archive entries before admitting them to imports.
    // ------------------=
    pub fn load_archive(&mut self, input: &[u8]) -> Result<()> {
        self.load_root(input, b"/packages", MAGIC)?; self.changed = false; Ok(())
    }
    // ------------------------=
    // FUNC: load_root
    // DESC: Checks archive version, component boundaries and entry topology before admitting persisted bytes into a granted mount.
    // ------------------=
    fn load_root(&mut self, input: &[u8], root: &[u8], magic: &[u8; 8]) -> Result<()> {
        if input.len() < 16 || input.len() > MAX_ARCHIVE || &input[..8] != magic || input[12..16] != [0; 4] { return Err(-22); }
        let count = u32::from_le_bytes(input[8..12].try_into().unwrap()) as usize;
        if count > MAX_FILES - 4 { return Err(-28); }
        let mut cursor = 16;
        for _ in 0..count {
            let header = input.get(cursor..cursor + 8).ok_or(-22i64)?;
            let length = u16::from_le_bytes(header[..2].try_into().unwrap()) as usize;
            let flags = u16::from_le_bytes(header[2..4].try_into().unwrap());
            let size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
            if length <= root.len() + 1 || length > MAX_PATH || flags > 3 || flags & 1 != 0 && size != 0 { return Err(-22); }
            let path = input.get(cursor + 8..cursor + 8 + length).ok_or(-22i64)?;
            let bytes = input.get(cursor + 8 + length..cursor + 8 + length + size).ok_or(-22i64)?;
            let (canonical, cl) = self.canonical(path)?;
            if &canonical[..cl] != path || !under(path, root) { return Err(-13); }
            let index = self.insert(path, flags & 1 != 0, false, bytes)?;
            self.entries[index].executable = flags & 2 != 0; cursor += 8 + length + size;
        }
        if cursor != input.len() { return Err(-22); }
        for entry in self.entries.iter().filter(|e| e.used && e.length > root.len() && under(&e.path[..e.length], root)) {
            self.parent(&entry.path[..entry.length])?;
        }
        Ok(())
    }
    // ------------------------=
    // FUNC: user_path
    // DESC: Copies and canonicalizes a NUL-terminated guest path through checked process page tables.
    // ------------------=
    fn user_path<T: PageTable>(&self, tables: &T, address: u64) -> Result<([u8; MAX_PATH], usize)> {
        let mut path = [0; MAX_PATH + 1];
        for i in 0..path.len() {
            copy_from_user(tables, address.checked_add(i as u64).ok_or(-14i64)?, &mut path[i..i + 1]).map_err(|_| -14i64)?;
            if path[i] == 0 { return self.canonical(&path[..i]); }
        } Err(-36)
    }
    // ------------------------=
    // FUNC: metadata
    // DESC: Emits the native metadata ABI with effective private grant rights, not ambient Unix permissions.
    // ------------------=
    fn metadata<T: PageTable>(&self, tables: &mut T, index: usize, destination: u64) -> Result<()> {
        let entry = self.entries[index]; let mut bytes = [0; 40];
        bytes[..8].copy_from_slice(&entry.identity.to_le_bytes()); bytes[8..16].copy_from_slice(&(entry.size as u64).to_le_bytes());
        bytes[28..32].copy_from_slice(&(if entry.directory { 2u32 } else { 1 }).to_le_bytes());
        let access = 1u32 | if !entry.readonly && self.mutable(&entry.path[..entry.length]) { 2 } else { 0 } | if entry.executable { 4 } else { 0 };
        bytes[32..36].copy_from_slice(&access.to_le_bytes()); copy_to_user(tables, destination, &bytes).map_err(|_| -14)
    }
    // ------------------------=
    // FUNC: token
    // DESC: Allocates a non-recycled descriptor token so stale handles cannot address a reopened file.
    // ------------------=
    fn token(&mut self) -> Result<u64> {
        let token = self.next_token;
        if token as u32 == u32::MAX { return Err(-75); }
        self.next_token = token.checked_add(1).ok_or(-75i64)?; Ok(token)
    }
    // ------------------------=
    // FUNC: syscall
    // DESC: Handles only the filesystem ABI and leaves process, memory, extension and audio services to their existing dispatcher.
    // ------------------=
    pub fn syscall<T: PageTable>(&mut self, tables: &mut T, op: u64, a: [u64; 6]) -> Option<i64> {
        if !matches!(op, 0x1206..=0x1208 | 0x120b..=0x121a | 0x1233..=0x1237) { return None; }
        Some(self.dispatch(tables, op, a).map_or_else(|error| error, |_| 0))
    }
    // ------------------------=
    // FUNC: dispatch
    // DESC: Validates guest buffers before mutation and executes confined private filesystem state transitions.
    // ------------------=
    fn dispatch<T: PageTable>(&mut self, tables: &mut T, op: u64, a: [u64; 6]) -> Result<()> {
        if op == 0x1214 {
            if a[1] < self.cwd_len as u64 { return Err(-34); }
            check_user_range(tables, a[0], self.cwd_len, true).map_err(|_| -14i64)?;
            check_user_range(tables, a[2], 8, true).map_err(|_| -14i64)?;
            copy_to_user(tables, a[0], &self.cwd[..self.cwd_len]).map_err(|_| -14i64)?;
            return copy_to_user(tables, a[2], &(self.cwd_len as u64).to_le_bytes()).map_err(|_| -14);
        }
        if matches!(op, 0x1206 | 0x120b | 0x1212 | 0x1215..=0x121a | 0x1233..=0x1235) {
            let (name, length) = self.user_path(tables, a[0])?; let path = &name[..length];
            if self.child_active && under(path, b"/data") { return Err(-13); }
            if matches!(op, 0x1219 | 0x121a) { return Err(-95); }
            if op == 0x1217 { return self.mkdir(path); }
            if op == 0x1218 || op == 0x1234 { return self.remove(path, op == 0x1234); }
            if op == 0x1233 { let (target, tl) = self.user_path(tables, a[1])?; return self.rename(path, &target[..tl]); }
            if op == 0x120b {
                if a[1] > u32::MAX as u64 { return Err(-22); }
                let flags = a[1] as u32;
                // NOFOLLOW (128) is intrinsically enforced: this filesystem
                // contains only regular files and directories, never symlinks.
                if flags == 0 || flags & !(63 | 128) != 0 || flags & 3 == 0 || flags & (8 | 16) != 0 && flags & 2 == 0
                    || flags & 32 != 0 && flags & 4 == 0 { return Err(-22); }
                let existing = self.find(path);
                if flags & 32 != 0 && existing.is_some() { return Err(-17); }
                let slot = self.handles.iter().position(Option::is_none).ok_or(-24i64)?;
                check_user_range(tables, a[2], 8, true).map_err(|_| -14i64)?;
                if flags & (2 | 4 | 8 | 16) != 0 && !self.mutable(path) { return Err(-13); }
                let index = if let Some(index) = existing { index } else {
                    if flags & 4 == 0 { return Err(-2); } self.parent(path)?; self.insert(path, false, false, b"")?
                };
                if self.entries[index].directory { return Err(-21); }
                if flags & 2 != 0 && self.entries[index].readonly { return Err(-13); }
                if flags & 8 != 0 { self.entries[index].size = 0; }
                let token = self.token()?;
                self.handles[slot] = Some(Handle { token, entry: index, position: if flags & 16 != 0 { self.entries[index].size } else { 0 }, flags });
                self.changed |= (existing.is_none() || flags & 8 != 0) && under(path, b"/packages");
                self.data_changed |= (existing.is_none() || flags & 8 != 0) && under(path, b"/data");
                return copy_to_user(tables, a[2], &token.to_le_bytes()).map_err(|_| -14);
            }
            let index = self.find(path).ok_or(-2i64)?;
            if op == 0x1212 { return self.metadata(tables, index, a[1]); }
            if op == 0x1235 { return self.chmod(index, a[1]); }
            if op == 0x1215 {
                if a[2] < length as u64 { return Err(-34); }
                check_user_range(tables, a[1], length, true).map_err(|_| -14i64)?;
                check_user_range(tables, a[3], 8, true).map_err(|_| -14i64)?;
                copy_to_user(tables, a[1], path).map_err(|_| -14i64)?;
                return copy_to_user(tables, a[3], &(length as u64).to_le_bytes()).map_err(|_| -14);
            }
            if !self.entries[index].directory { return Err(-20); }
            if op == 0x1216 { self.cwd = name; self.cwd_len = length; return Ok(()); }
            let slot = self.directories.iter().position(Option::is_none).ok_or(-24i64)?;
            check_user_range(tables, a[1], 8, true).map_err(|_| -14i64)?;
            let token = self.token()?;
            self.directories[slot] = Some(Directory { token, path: name, length, next: 0 });
            return copy_to_user(tables, a[1], &token.to_le_bytes()).map_err(|_| -14);
        }
        if op == 0x1207 || op == 0x1208 {
            let index = self.directories.iter().position(|d| d.is_some_and(|d| d.token == a[0])).ok_or(-9i64)?;
            if op == 0x1208 { self.directories[index] = None; return Ok(()); }
            let directory = self.directories[index].unwrap(); let mut next = directory.next;
            let mut name = [0; MAX_PATH + 1]; let mut length = 0;
            while next < MAX_FILES {
                let entry = &self.entries[next]; next += 1;
                if !entry.used { continue; }
                if self.child_active && under(&entry.path[..entry.length], b"/data") { continue; }
                let parent = &directory.path[..directory.length];
                let rest = if parent == b"/" { entry.path[..entry.length].strip_prefix(b"/".as_slice()) }
                    else { entry.path[..entry.length].strip_prefix(parent).and_then(|rest| rest.strip_prefix(b"/".as_slice())) };
                if let Some(rest) = rest.filter(|rest| !rest.is_empty() && !rest.contains(&b'/')) { length = rest.len(); name[..length].copy_from_slice(rest); break; }
            }
            if a[2] < (length + 1) as u64 { return Err(-34); }
            copy_to_user(tables, a[1], &name[..length + 1]).map_err(|_| -14i64)?;
            self.directories[index].as_mut().unwrap().next = next; return Ok(());
        }
        let slot = self.handles.iter().position(|h| h.is_some_and(|h| h.token == a[0])).ok_or(-9i64)?;
        let handle = self.handles[slot].unwrap(); let index = handle.entry;
        if op == 0x120f { self.handles[slot] = None; return Ok(()); }
        if op == 0x1211 { return self.metadata(tables, index, a[1]); }
        if op == 0x1236 { return Ok(()); } // The trusted app-data adapter completes its durable barrier before returning to userspace; packages retain their transaction boundary.
        if op == 0x1237 { return self.chmod(index, a[1]); }
        if op == 0x120e {
            let base = match a[2] { 0 => 0, 1 => handle.position, 2 => self.entries[index].size, _ => return Err(-22) };
            let position = (base as i128 + a[1] as i64 as i128).try_into().ok().filter(|p: &usize| *p <= MAX_FILE).ok_or(-22i64)?;
            copy_to_user(tables, a[3], &(position as u64).to_le_bytes()).map_err(|_| -14i64)?;
            self.handles[slot].as_mut().unwrap().position = position; return Ok(());
        }
        if op == 0x1213 || op == 0x120d {
            if handle.flags & 2 == 0 || self.entries[index].readonly || !self.mutable(&self.entries[index].path[..self.entries[index].length]) { return Err(-13); }
            let position = if handle.flags & 16 != 0 { self.entries[index].size } else { handle.position };
            let count = a[2].min(4096) as usize;
            let size = if op == 0x1213 { usize::try_from(a[1]).map_err(|_| -27i64)? } else { position.checked_add(count).ok_or(-27i64)? };
            let mut bytes = [0; 4096];
            if op == 0x120d {
                check_user_range(tables, a[3], 8, true).map_err(|_| -14i64)?;
                if count == 0 { return copy_to_user(tables, a[3], &0u64.to_le_bytes()).map_err(|_| -14); }
                copy_from_user(tables, a[1], &mut bytes[..count]).map_err(|_| -14i64)?;
            }
            self.capacity(index, size)?;
            let entry = &mut self.entries[index];
            if size > entry.size { self.bytes[entry.offset + entry.size..entry.offset + size].fill(0); }
            if op == 0x1213 { entry.size = size; } else {
                self.bytes[entry.offset + position..entry.offset + position + count].copy_from_slice(&bytes[..count]);
                entry.size = entry.size.max(size); self.handles[slot].as_mut().unwrap().position = size;
                copy_to_user(tables, a[3], &(count as u64).to_le_bytes()).map_err(|_| -14i64)?;
            }
            self.changed |= under(&entry.path[..entry.length], b"/packages");
            self.data_changed |= under(&entry.path[..entry.length], b"/data"); return Ok(());
        }
        if op == 0x120c || op == 0x1210 {
            if handle.flags & 1 == 0 { return Err(-9); }
            let positional = op == 0x1210;
            let position = if positional { usize::try_from(a[1]).map_err(|_| -22i64)? } else { handle.position };
            let destination = a[if positional { 2 } else { 1 }]; let result = a[if positional { 4 } else { 3 }];
            let count = (a[if positional { 3 } else { 2 }].min(4096) as usize).min(self.entries[index].size.saturating_sub(position));
            check_user_range(tables, result, 8, true).map_err(|_| -14i64)?;
            if count > 0 { let offset = self.entries[index].offset + position;
                copy_to_user(tables, destination, &self.bytes[offset..offset + count]).map_err(|_| -14i64)?; }
            copy_to_user(tables, result, &(count as u64).to_le_bytes()).map_err(|_| -14i64)?;
            if !positional { self.handles[slot].as_mut().unwrap().position += count; } return Ok(());
        }
        Err(-95)
    }
    // ------------------------=
    // FUNC: chmod
    // DESC: Retains only the executable hint while keeping effective access bound to the owning private grant.
    // ------------------=
    fn chmod(&mut self, index: usize, mode: u64) -> Result<()> {
        let entry = &self.entries[index];
        if mode & !0o777 != 0 || entry.readonly || !self.mutable(&entry.path[..entry.length]) { return Err(-13); }
        self.changed |= under(&entry.path[..entry.length], b"/packages");
        self.data_changed |= under(&entry.path[..entry.length], b"/data");
        self.entries[index].executable = mode & 0o100 != 0; Ok(())
    }
}
