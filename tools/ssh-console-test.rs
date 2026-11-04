#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "storage.rs"] mod storage;
#[path = "../kernel/ui/mod.rs"] mod ui;
mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey { Character(u8), Backspace, Delete, Left, Right, Home, End, Enter }
}
use runtime::{identity::*, console_execution::{self as execution, Diagnostic}, console_terminal::Terminal};
// ------------------------=
// FUNC: output_text
// DESC: Keeps diagnostics out of behavioral assertions.
// ------------------=
fn output_text(_: &[u8]) {}
// ------------------------=
// FUNC: type_input
// DESC: Exercises terminal input while acknowledging echoed bytes as a consuming client would.
// ------------------=
fn type_input(terminal: &mut Terminal, bytes: &[u8]) {
    for byte in bytes { let n = terminal.echo().len(); terminal.consume_echo(n); terminal.input_byte(*byte, 10); }
}
// ------------------------=
// FUNC: main
// DESC: Tests production native identity isolation, shared IOP execution, failure status, resource cleanup and independent terminal state.
// ------------------=
fn main() {
    runtime::initialize(false);
    runtime::with_runtime(|r| {
        r.start_all(1);
        r.console_environment = execution::Environment { architecture: b"fixture", ready_devices: 3 };
        r.identity.begin_onboarding().unwrap();
        r.identity.create_machine(b"SSH console", 0xaa64, 1, 1).unwrap();
        let user = r.identity.create_user(b"operator", b"Operator", 2).unwrap();
        r.identity.create_password(user.id, b"fixture password", 3).unwrap();
        r.identity.complete_onboarding().unwrap();
        let local = r.identity.create_session(user.id, b"fixture password", 4).unwrap();
        let key = ed25519_dalek::SigningKey::from_bytes(&[7;32]).verifying_key().to_bytes();
        r.identity.manage_ssh_key_durable(local.id, user.id, key, false, 5, |_| Ok(())).unwrap();
        let remote = r.identity.create_ssh_session(user.id, key, 6).unwrap();
        let other = r.identity.create_ssh_session(user.id, key, 7).unwrap();
        assert_eq!(remote.origin, SessionOrigin::Ssh);
        assert_eq!(remote.capabilities, SESSION_REMOTE_QUERY);
        assert_eq!(remote.capabilities & ORDINARY_SESSION_CAPABILITIES, 0);
        assert_eq!(remote.personal_space, StableId::zero());
        assert_eq!(remote.ai_context, StableId::zero());
        let baseline = r.capabilities.count();
        for command in [b"help".as_slice(), b"system status", b"node list", b"ai status"] {
            let result = execution::execute(r, user.id, remote.id, command, 8);
            assert_eq!(result.diagnostic, Diagnostic::Success);
            assert_eq!(result.status, 0);
            assert!(!result.stdout.bytes().is_empty());
            assert!(result.stderr.bytes().is_empty());
            assert_eq!(r.capabilities.count(), baseline);
            let local_result = execution::execute(r, user.id, local.id, command, 8);
            assert_eq!(local_result.diagnostic, result.diagnostic);
            assert_eq!(local_result.operation, result.operation);
            assert_eq!(local_result.status, result.status);
        }
        for _ in 0..100 {
            let result = execution::execute(r, user.id, remote.id, b"system status", 8);
            assert_eq!(result.status, 0);
            assert_eq!(r.capabilities.count(), baseline);
            assert_eq!(r.iop.queue_depth(0xe501), Some(0));
        }
        let denied = execution::execute(r, user.id, remote.id, b"voice status", 8);
        assert_eq!(denied.status, 126); assert_eq!(denied.diagnostic, Diagnostic::Denied);
        assert!(denied.stdout.bytes().is_empty()); assert!(!denied.stderr.bytes().is_empty());
        assert_eq!(execution::execute(r, user.id, remote.id, b"/bin/sh", 8).diagnostic, Diagnostic::Syntax);
        assert_eq!(execution::execute(r, StableId::zero(), remote.id, b"system status", 8).diagnostic, Diagnostic::Denied);
        r.identity.end_session(remote.id, user.id).unwrap();
        assert_eq!(execution::execute(r, user.id, remote.id, b"help", 8).diagnostic, Diagnostic::Denied);
        assert!(r.identity.console_session_authorized(other.id, user.id));
        assert!(r.identity.console_session_authorized(local.id, user.id));
        r.identity.manage_ssh_key_durable(local.id, user.id, key, true, 9, |_| Ok(())).unwrap();
        assert!(!r.identity.console_session_authorized(other.id, user.id));
        assert!(r.identity.console_session_authorized(local.id, user.id));
        r.identity.end_session(other.id, user.id).unwrap();
        assert_eq!(r.capabilities.count(), baseline);
    }).unwrap();
    let mut first = Terminal::new(); let mut second = Terminal::new();
    first.pty = true; second.pty = true;
    assert!(first.resize(132, 40)); assert!(!first.resize(0, 2000));
    assert!(first.start(None, 1)); assert!(second.start(None, 1));
    type_input(&mut first, b"system statuX\x7fs\r");
    type_input(&mut second, b"node list");
    assert!(first.pending); assert_eq!(&first.input[..first.length], b"system status");
    assert_eq!(&second.input[..second.length], b"node list");
    first.interrupt(); assert!(!first.pending); assert_eq!(first.status, 130);
    assert_eq!(second.length, 9); assert!(!second.closing);
    first.output_complete(); type_input(&mut first, b"\n\x1b[A");
    assert_eq!(&first.input[..first.length], b"system status");
    type_input(&mut first, b"\x15\x04"); assert!(first.closing); assert!(!second.closing);
    let mut exec = Terminal::new(); assert!(exec.start(Some(b"system status"), 1));
    assert!(!exec.interactive); assert!(exec.pending);
    let mut outcome = execution::Outcome::new(); outcome.stdout.write_line(b"a"); outcome.stderr.write_line(b"b");
    exec.pty = true; exec.complete(outcome); exec.stage_output();
    assert_eq!(exec.wire_length, 3); assert!(!exec.wire_stderr);
    exec.wire_offset = 1; exec.stage_output(); assert_eq!(exec.wire_offset, 1);
    exec.wire_offset = exec.wire_length; exec.stage_output(); assert_eq!(exec.wire_length, 3);
    exec.wire_offset = exec.wire_length; exec.stage_output(); assert_eq!(exec.wire_length, 0);
    exec.output_complete(); assert!(exec.closing);
    let mut full = Terminal::new(); full.pty = true; full.start(None, 1);
    type_input(&mut full, &[b'a'; 512]); type_input(&mut full, b"\x01"); assert!(!full.closing);
    type_input(&mut full, b"b"); assert!(!full.pending); assert_eq!(full.status, 75);
}
