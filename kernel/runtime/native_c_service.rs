//! Native compiler object service. Only the trusted launch dispatcher constructs
//! sessions; requests contain no authority, identity, or host filesystem paths.
use super::{capability::{CapabilityId, CapabilityManager, CapabilityType},
    execution::SecurityIdentity, native_c_io::{ObjectIo, IoError, READ, WRITE}};
use crate::storage::{BlockDevice, object::ObjectStore};

pub enum Request<'a> {
    Open { path: &'a [u8], flags: u32 },
    Read(i32),
    ReadAt { handle: i32, offset: usize },
    Write { handle: i32, bytes: &'a [u8] },
    Seek { handle: i32, offset: i64, origin: u32 },
    Truncate { handle: i32, length: usize },
    Flush(i32),
    Close(i32),
    CurrentDirectory,
    Resolve(&'a [u8]),
    ChangeDirectory(&'a [u8]),
    MakeDirectory(&'a [u8]),
    Remove(&'a [u8]),
    Link { source: &'a [u8], destination: &'a [u8], symbolic: bool },
}

pub struct Session<'a, const N: usize> {
    io: ObjectIo<'a, N>,
    owner: SecurityIdentity,
    scope: u64,
    read: CapabilityId,
    write: CapabilityId,
    cancelled: bool,
}

impl<'a, const N: usize> Session<'a, N> {
    // ------------------------=
    // FUNC: new
    // DESC: Binds a trusted namespace grant set and live personal-space capabilities to one process identity.
    // ------------------=
    pub fn new(io: ObjectIo<'a, N>, owner: SecurityIdentity, scope: u64,
        read: CapabilityId, write: CapabilityId) -> Self {
        Self { io, owner, scope, read, write, cancelled: false }
    }

    // ------------------------=
    // FUNC: cancel
    // DESC: Permanently cancels this session and discards unpublished output without relying on application cleanup.
    // ------------------=
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.io.discard_all();
    }

    // ------------------------=
    // FUNC: authorize
    // DESC: Checks expiry, revocation, holder, launch scope and rights on every operation, including existing handles.
    // ------------------=
    fn authorize(&self, caps: &CapabilityManager, mode: u32, now: u64) -> Result<(), IoError> {
        for (mask, id, kind) in [(READ, self.read, CapabilityType::PersonalSpaceRead),
            (WRITE, self.write, CapabilityType::PersonalSpaceWrite)] {
            if mode & mask != 0 {
                caps.validate(id, self.owner, kind, self.scope, 1, 0, now)
                    .map_err(|_| IoError::Denied)?;
            }
        }
        Ok(())
    }

    // ------------------------=
    // FUNC: execute
    // DESC: Performs bounded object operations after live authorization; close after revocation erases rather than commits output.
    // ------------------=
    pub fn execute<D: BlockDevice>(&mut self, caller: SecurityIdentity, caps: &CapabilityManager,
        store: &mut ObjectStore<D>, request: Request<'_>, output: &mut [u8], now: u64)
        -> Result<usize, IoError> {
        if caller != self.owner || self.cancelled { return Err(IoError::Denied); }
        let mode = match &request {
            Request::Open { flags, .. } => flags & (READ | WRITE),
            Request::Write { .. } | Request::Truncate { .. } | Request::Flush(_) |
                Request::MakeDirectory(_) | Request::Remove(_) => WRITE,
            Request::Link { .. } => READ | WRITE,
            Request::Close(handle) | Request::Seek { handle, .. } => self.io.access_mode(*handle)?,
            _ => READ,
        };
        if let Err(error) = self.authorize(caps, mode, now) {
            if let Request::Close(handle) = request { self.io.discard(handle)?; }
            return Err(error);
        }
        match request {
            Request::Open { path, flags } => self.io.open(store, path, flags).map(|h| h as usize),
            Request::Read(handle) => self.io.read(handle, output),
            Request::ReadAt { handle, offset } => self.io.read_at(handle, offset, output),
            Request::Write { handle, bytes } => self.io.write(handle, bytes),
            Request::Seek { handle, offset, origin } => self.io.seek(handle, offset, origin),
            Request::Truncate { handle, length } => self.io.truncate(handle, length).map(|_| 0),
            Request::Flush(handle) => self.io.flush(store, handle).map(|_| 0),
            Request::Close(handle) => self.io.close(store, handle).map(|_| 0),
            Request::CurrentDirectory => self.io.current_directory(output),
            Request::Resolve(path) => self.io.canonical_path(store, path, output),
            Request::ChangeDirectory(path) => self.io.change_directory(store, path).map(|_| 0),
            Request::MakeDirectory(path) => self.io.make_directory(store, path).map(|_| 0),
            Request::Remove(path) => self.io.unlink(store, path).map(|_| 0),
            Request::Link { source, destination, symbolic } => self.io.link(store, source, destination, symbolic).map(|_| 0),
        }
    }
}
