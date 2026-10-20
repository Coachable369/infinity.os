//! Exact launch binding and bounded native failure evidence. This grants no
//! execution authority and never requests relaunch or replays application input.
use core::fmt::Write;

pub const SOURCE_BYTES: usize = 16 * 1024;
pub const REQUEST_BYTES: usize = 1024;
pub const PATH_BYTES: usize = 95;
pub const STDERR_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub user: [u8; 16],
    pub session: [u8; 16],
    pub editor_revision: u64,
    pub goal: Option<u64>,
    pub source_object: [u8; 16],
    pub source_version: u32,
    pub workflow_source_revision: u64,
    pub context: u64,
}

#[derive(Clone, Copy)]
pub struct Current<'a> {
    pub user: [u8; 16],
    pub session: [u8; 16],
    pub editor_revision: u64,
    pub goal: Option<u64>,
    pub source_binding: Option<([u8; 16], u32)>,
    pub source: &'a [u8],
    pub request: &'a [u8],
    pub path: &'a [u8],
    pub authorized: bool,
    pub editor_alive: bool,
    pub ai_allowed: bool,
    pub idle: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Exit(i64),
    Fault(u64, u64, u64),
    Cancelled,
    Deadline,
    Budget,
    Platform,
    Release,
}

impl Failure {
    // ------------------------=
    // FUNC: observed_failure
    // DESC: Separates terminal lifecycle failure from source-repair eligibility; normal exit and explicit cancellation are not failures.
    // ------------------=
    pub const fn observed_failure(self) -> bool {
        !matches!(self, Self::Exit(0) | Self::Cancelled)
    }

    // ------------------------=
    // FUNC: repairable
    // DESC: Distinguishes observed program failures from normal exit, STOP and execution infrastructure/resource failures.
    // ------------------=
    pub const fn repairable(self) -> bool {
        matches!(self, Self::Exit(code) if code != 0) || matches!(self, Self::Fault(..))
    }
}

pub struct Snapshot {
    pub binding: Binding,
    source: [u8; SOURCE_BYTES],
    source_length: usize,
    request: [u8; REQUEST_BYTES],
    request_length: usize,
    path: [u8; PATH_BYTES],
    path_length: usize,
    stderr: [u8; STDERR_BYTES],
    stderr_length: usize,
}

impl Snapshot {
    // ------------------------=
    // FUNC: new
    // DESC: Captures exact generated inputs at compile or validation admission without synthesizing authority or context.
    // ------------------=
    pub fn new(binding: Binding, source: &[u8], request: &[u8], path: &[u8]) -> Option<Self> {
        if source.is_empty() || source.len() > SOURCE_BYTES || request.is_empty()
            || request.len() > REQUEST_BYTES || path.is_empty() || path.len() > PATH_BYTES
            || binding.source_object == [0; 16] { return None; }
        let mut snapshot = Self { binding, source: [0; SOURCE_BYTES], source_length: source.len(),
            request: [0; REQUEST_BYTES], request_length: request.len(), path: [0; PATH_BYTES],
            path_length: path.len(), stderr: [0; STDERR_BYTES], stderr_length: 0 };
        snapshot.source[..source.len()].copy_from_slice(source);
        snapshot.request[..request.len()].copy_from_slice(request);
        snapshot.path[..path.len()].copy_from_slice(path);
        Some(snapshot)
    }

    // ------------------------=
    // FUNC: request
    // DESC: Returns the immutable initiating objective, not a newer ambient editor prompt.
    // ------------------=
    pub fn request(&self) -> &[u8] { &self.request[..self.request_length] }

    // ------------------------=
    // FUNC: path
    // DESC: Returns the immutable saved source path for version checks and diagnostics.
    // ------------------=
    pub fn path(&self) -> &[u8] { &self.path[..self.path_length] }

    // ------------------------=
    // FUNC: can_repair
    // DESC: Requires a genuine code failure and unchanged exact source, initiating identity, request, storage version and editor revision.
    // ------------------=
    pub fn can_repair(&self, current: Current<'_>, failure: Failure) -> bool {
        let binding = self.binding;
        failure.repairable() && binding.context != 0 && current.authorized
            && current.editor_alive && current.ai_allowed && current.idle
            && binding.user == current.user && binding.session == current.session
            && binding.editor_revision == current.editor_revision && binding.goal == current.goal
            && current.source_binding == Some((binding.source_object, binding.source_version))
            && current.source == &self.source[..self.source_length]
            && current.request == self.request() && current.path == self.path()
    }

    // ------------------------=
    // FUNC: record_stderr
    // DESC: Retains only a bounded tail of actual application stderr, never stdout or generated success claims.
    // ------------------=
    pub fn record_stderr(&mut self, bytes: &[u8]) {
        if bytes.len() >= STDERR_BYTES {
            self.stderr.copy_from_slice(&bytes[bytes.len() - STDERR_BYTES..]);
            self.stderr_length = STDERR_BYTES;
            return;
        }
        let retain = self.stderr_length.min(STDERR_BYTES - bytes.len());
        self.stderr.copy_within(self.stderr_length - retain..self.stderr_length, 0);
        self.stderr[retain..retain + bytes.len()].copy_from_slice(bytes);
        self.stderr_length = retain + bytes.len();
    }

    // ------------------------=
    // FUNC: stderr
    // DESC: Returns the retained exact stderr tail without adding native-exit prose or changing typed Python exception evidence.
    // ------------------=
    pub fn stderr(&self) -> &[u8] { &self.stderr[..self.stderr_length] }

    // ------------------------=
    // FUNC: diagnostics
    // DESC: Formats actual typed exit/fault evidence and captured stderr without inventing compiler diagnostics.
    // ------------------=
    pub fn diagnostics(&self, failure: Failure, output: &mut [u8]) -> usize {
        if !failure.repairable() { return 0; }
        let mut writer = DiagnosticWriter { output, length: 0 };
        match failure {
            Failure::Exit(code) => { let _ = write!(writer, "Native application exited with status {code}.\n"); }
            Failure::Fault(reason, address, pc) => {
                let _ = write!(writer, "Native application fault: reason={reason:#x}, address={address:#x}, pc={pc:#x}.\n");
            }
            _ => return 0,
        }
        if self.stderr_length > 0 {
            let _ = writer.write_str("Captured application stderr (bounded tail):\n");
            let length = self.stderr_length.min(writer.output.len() - writer.length);
            writer.output[writer.length..writer.length + length]
                .copy_from_slice(&self.stderr[self.stderr_length - length..self.stderr_length]);
            writer.length += length;
        }
        writer.length
    }
}

struct DiagnosticWriter<'a> { output: &'a mut [u8], length: usize }
impl Write for DiagnosticWriter<'_> {
    // ------------------------=
    // FUNC: write_str
    // DESC: Bounds trusted diagnostic formatting to the caller's existing repair buffer.
    // ------------------=
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        let length = value.len().min(self.output.len() - self.length);
        self.output[self.length..self.length + length].copy_from_slice(&value.as_bytes()[..length]);
        self.length += length;
        if length == value.len() { Ok(()) } else { Err(core::fmt::Error) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: fixture
    // DESC: Creates an exact immutable launch fixture without runtime, storage or LLM dependencies.
    // ------------------=
    fn fixture() -> (Snapshot, Current<'static>) {
        let binding = Binding { user: [1; 16], session: [2; 16], editor_revision: 3, goal: Some(4),
            source_object: [5; 16], source_version: 6, workflow_source_revision: 7, context: 8 };
        let current = Current { user: binding.user, session: binding.session, editor_revision: 3,
            goal: Some(4), source_binding: Some(([5; 16], 6)), source: b"exact source",
            request: b"create app", path: b"/source.c", authorized: true,
            editor_alive: true, ai_allowed: true, idle: true };
        (Snapshot::new(binding, current.source, current.request, current.path).unwrap(), current)
    }

    #[test]
    // ------------------------=
    // FUNC: exact_binding_is_required
    // DESC: Rejects every stale identity, prompt, revision and source transition while preserving the original snapshot.
    // ------------------=
    fn exact_binding_is_required() {
        let (snapshot, current) = fixture();
        assert!(snapshot.can_repair(current, Failure::Exit(1)));
        for changed in [
            Current { user: [9; 16], ..current }, Current { session: [9; 16], ..current },
            Current { editor_revision: 9, ..current }, Current { goal: None, ..current },
            Current { source_binding: Some(([5; 16], 9)), ..current },
            Current { source_binding: Some(([9; 16], 6)), ..current },
            Current { source: b"user edited source", ..current },
            Current { request: b"new objective", ..current }, Current { path: b"/other.c", ..current },
            Current { authorized: false, ..current }, Current { editor_alive: false, ..current },
            Current { ai_allowed: false, ..current }, Current { idle: false, ..current },
        ] { assert!(!snapshot.can_repair(changed, Failure::Exit(1))); }
        assert!(snapshot.can_repair(current, Failure::Fault(1, 2, 3)));
    }

    #[test]
    // ------------------------=
    // FUNC: cancellation_and_infrastructure_do_not_repair
    // DESC: Distinguishes genuine nonzero/fault evidence from STOP, normal completion and resource failures.
    // ------------------=
    fn cancellation_and_infrastructure_do_not_repair() {
        let (mut snapshot, current) = fixture();
        let mut diagnostics = [0; 128];
        for failure in [Failure::Exit(0), Failure::Cancelled, Failure::Deadline,
            Failure::Budget, Failure::Platform, Failure::Release] {
            assert!(!snapshot.can_repair(current, failure));
            assert_eq!(snapshot.diagnostics(failure, &mut diagnostics), 0);
            assert_eq!(failure.observed_failure(),
                !matches!(failure, Failure::Exit(0) | Failure::Cancelled));
        }
        assert!(Failure::Exit(1).observed_failure());
        assert!(Failure::Fault(1, 2, 3).observed_failure());
        snapshot.binding.context = 0;
        assert!(!snapshot.can_repair(current, Failure::Exit(1)));
    }

    #[test]
    // ------------------------=
    // FUNC: stderr_is_bounded_and_failure_evidence_is_real
    // DESC: Preserves the exact bounded tail and emits diagnostics only from typed terminal outcomes.
    // ------------------=
    fn stderr_is_bounded_and_failure_evidence_is_real() {
        let (mut snapshot, _) = fixture();
        snapshot.record_stderr(&[1; STDERR_BYTES - 1]); snapshot.record_stderr(&[2, 3, 4]);
        assert_eq!(snapshot.stderr_length, STDERR_BYTES);
        assert_eq!(&snapshot.stderr[STDERR_BYTES - 3..], &[2, 3, 4]);
        assert_eq!(snapshot.stderr(), &snapshot.stderr[..snapshot.stderr_length]);
        snapshot.record_stderr(&[5; STDERR_BYTES + 9]);
        assert_eq!(snapshot.stderr, [5; STDERR_BYTES]);
        assert_eq!(snapshot.stderr(), &[5; STDERR_BYTES]);
        let mut diagnostics = [0; 512];
        assert_eq!(snapshot.diagnostics(Failure::Exit(-1), &mut diagnostics), diagnostics.len());
        assert!(snapshot.diagnostics(Failure::Fault(4, 5, 6), &mut diagnostics) > 0);
        assert_eq!(snapshot.diagnostics(Failure::Exit(1), &mut []), 0);
    }
}
