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
        assert_eq!(remote.capabilities,local.capabilities);
        assert_eq!(remote.personal_space,user.personal_space_ref);
        assert!(!remote.ai_context.is_zero());assert_ne!(remote.ai_context,local.ai_context);
        assert_ne!(remote.id,local.id);assert_ne!(remote.ai_context,other.ai_context);
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
        assert_eq!(denied.status, 126); assert_eq!(denied.diagnostic, Diagnostic::Unsupported);
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
    mailbox_contract();
    private_terminal();
    raw_terminal();
    exec_input();
    output_boundaries();
}

// ------------------------=
// FUNC: mailbox_contract
// DESC: Exercises real channel isolation, ordered output backpressure, cancellation and stale-session rejection without parsing display text.
// ------------------=
fn mailbox_contract(){
    use runtime::console_remote::{Mailbox,Binding,JobKind,Reply};
    let mut mailbox=Mailbox::new();
    let first=Binding{slot:0,user:StableId([1;16]),session:StableId([2;16])};
    let second=Binding{slot:1,user:StableId([3;16]),session:StableId([4;16])};
    assert!(mailbox.bind(first));assert!(mailbox.bind(second));
    assert!(!mailbox.bind(Binding{session:StableId([5;16]),..first}));
    let sequence=mailbox.submit(first,b"service list",true).unwrap();
    assert!(mailbox.submit(first,b"help",true).is_none());
    mailbox.submit(second,b"llm ask hello",false).unwrap();
    let one=mailbox.take_job().unwrap();let two=mailbox.take_job().unwrap();
    assert_eq!(one.binding,first);assert_eq!(two.binding,second);assert_eq!(one.kind,JobKind::Command);
    assert_eq!(&one.input[..one.length],b"service list");assert!(!two.interactive);
    for status in [10,11] {let mut result=execution::Outcome::new();result.status=status;
        assert!(mailbox.reply(first,Reply{sequence,outcome:result,more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false}).is_ok());}
    assert!(mailbox.reply(first,Reply{sequence,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false}).is_err());
    assert!(mailbox.take_reply(second).is_none());
    assert_eq!(mailbox.take_reply(first).unwrap().outcome.status,10);
    assert_eq!(mailbox.take_reply(first).unwrap().outcome.status,11);
    mailbox.interrupt(first,true);let cancel=mailbox.take_job().unwrap();assert_eq!(cancel.kind,JobKind::Interrupt);
    assert!(mailbox.reply(first,Reply{sequence,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false}).is_err());
    assert!(mailbox.active(second));
    mailbox.retire(first);assert!(!mailbox.active(first));assert!(mailbox.active(second));
    assert!(!mailbox.bind(Binding{session:StableId([9;16]),..first}));
    assert_eq!(mailbox.take_job().unwrap().kind,JobKind::Closed);
    let fresh=Binding{session:StableId([9;16]),..first};assert!(mailbox.bind(fresh));
    assert!(mailbox.reply(first,Reply{sequence,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false}).is_err());
    mailbox.resize(fresh,132,40);mailbox.input(fresh,b"\x1b[A\x03\x00",true).unwrap();
    let raw=mailbox.take_job().unwrap();assert_eq!(raw.kind,JobKind::Input);
    assert_eq!((raw.columns,raw.rows),(132,40));assert_eq!(&raw.input[..raw.length],b"\x1b[A\x03\x00");
    mailbox.eof(fresh,false).unwrap();assert!(mailbox.take_ready_job([false;4]).is_none());
    let eof=mailbox.take_ready_job([true;4]).unwrap();assert_eq!(eof.kind,JobKind::Eof);assert_eq!(eof.length,0);
    mailbox.submit(fresh,b"blocked",true).unwrap();mailbox.submit(second,b"ready",true).unwrap();
    assert_eq!(mailbox.take_ready_job([false,true,false,false]).unwrap().binding,second);
    assert!(mailbox.take_ready_job([false;4]).is_none());
    mailbox.interrupt(fresh,true);assert_eq!(mailbox.take_ready_job([false;4]).unwrap().kind,JobKind::Interrupt);
    mailbox.submit(fresh,b"still bounded",true).unwrap();assert!(mailbox.submit(fresh,b"overwrite",true).is_none());
    mailbox.retire(fresh);assert_eq!(mailbox.take_ready_job([false;4]).unwrap().kind,JobKind::Closed);
}

// ------------------------=
// FUNC: private_terminal
// DESC: Verifies secret and private modes do not retain history, asynchronous replies preserve drafts, and deferred exec waits for its final receipt.
// ------------------=
fn private_terminal(){
    use runtime::console_remote::Reply;
    let mut terminal=Terminal::new();terminal.pty=true;terminal.start(None,0);
    let n=terminal.echo().len();terminal.consume_echo(n);
    terminal.secret=true;type_input(&mut terminal,b"private-password");
    assert!(terminal.echo().is_empty());assert_eq!(terminal.history_count(),0);
    type_input(&mut terminal,b"\r");assert!(terminal.pending);assert_eq!(terminal.history_count(),0);
    terminal.submitted(10);assert!(terminal.waiting);assert!(!terminal.can_input());
    terminal.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:true,raw:false,read_input:false,clear_history:false,close:false},false);
    terminal.output_complete();type_input(&mut terminal,b"help\r");assert_eq!(terminal.history_count(),0);
    terminal.complete(execution::Outcome::new());terminal.output_complete();terminal.private=false;
    type_input(&mut terminal,b"draft");let before=terminal.input;let length=terminal.length;
    terminal.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:true,private:true,raw:false,read_input:false,clear_history:false,close:false},true);
    assert_eq!(terminal.input,before);assert_eq!(terminal.length,length);assert!(!terminal.secret);assert!(!terminal.private);
    terminal.output_complete();assert_eq!(terminal.length,length);
    type_input(&mut terminal,b"\r");assert_ne!(terminal.history_count(),0);terminal.submitted(10);
    terminal.deliver(Reply{sequence:2,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:true,close:false},false);
    assert_eq!(terminal.history_count(),0);terminal.output_complete();
    type_input(&mut terminal,b"retained\r");assert_eq!(terminal.history_count(),1);terminal.submitted(11);
    terminal.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:true,close:false},true);
    assert_eq!(terminal.history_count(),1);
    let mut exec=Terminal::new();exec.start(Some(b"ai ask hello"),0);exec.submitted(1);
    exec.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    exec.output_complete();assert!(!exec.closing);assert!(exec.waiting);assert_ne!(exec.deadline,0);
    exec.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    exec.output_complete();assert!(exec.closing);assert!(!exec.waiting);
}

// ------------------------=
// FUNC: raw_terminal
// DESC: Exercises fullscreen-controller raw input, resize notifications, no line discipline and restored shell mode after exit.
// ------------------=
fn raw_terminal(){
    use runtime::console_remote::Reply;
    let mut terminal=Terminal::new();terminal.pty=true;terminal.start(Some(b"edit /home/test.py"),0);terminal.submitted(1);
    terminal.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:true,read_input:false,clear_history:false,close:false},false);
    terminal.output_complete();assert!(terminal.raw);assert!(!terminal.closing);assert!(terminal.can_input());assert_eq!(terminal.deadline,0);
    assert!(terminal.echo().is_empty());assert!(terminal.input_raw(b"\x1b[A\x03\x00\n",2));
    assert_eq!(&terminal.input[..terminal.length],b"\x1b[A\x03\x00\n");assert_eq!(terminal.history_count(),0);
    assert_eq!(terminal.status,0);assert!(terminal.echo().is_empty());terminal.submitted(2);
    terminal.deliver(Reply{sequence:2,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:true,read_input:false,clear_history:false,close:false},false);
    terminal.output_complete();assert!(terminal.input_raw(b"",3));assert!(terminal.pending);terminal.submitted(3);
    terminal.deliver(Reply{sequence:3,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    terminal.output_complete();assert!(!terminal.raw);assert!(terminal.closing);
    let mut piped=Terminal::new();piped.start(None,0);let n=piped.echo().len();piped.consume_echo(n);
    type_input(&mut piped,b"service list\n");piped.closing=true;assert!(!piped.ready_to_close());
    piped.submitted(1);assert!(!piped.ready_to_close());
    piped.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    assert!(!piped.ready_to_close());piped.output_complete();assert!(piped.ready_to_close());
    let mut exit=Terminal::new();exit.start(None,0);let n=exit.echo().len();exit.consume_echo(n);
    exit.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:true},true);
    assert!(!exit.closing);exit.output_complete();let n=exit.echo().len();exit.consume_echo(n);
    let mut result=execution::Outcome::new();result.stdout.write(&[65,66,67]);
    exit.deliver(Reply{sequence:2,outcome:result,more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:true},false);
    assert!(exit.closing);assert!(!exit.ready_to_close());assert_eq!(exit.status,0);assert!(!exit.waiting);
    exit.stage_output();assert_eq!(exit.wire_length,3);exit.wire_offset=1;assert!(!exit.ready_to_close());
    exit.wire_offset=3;exit.stage_output();exit.output_complete();assert!(exit.ready_to_close());
}

// ------------------------=
// FUNC: exec_input
// DESC: Exercises exec password and native stdin continuation admission, empty lines, privacy, asynchronous metadata and final output closure.
// ------------------=
fn exec_input(){
    use runtime::console_remote::Reply;
    let mut secret=Terminal::new();secret.pty=true;secret.start(Some(b"user create member"),0);secret.submitted(1);
    secret.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:true,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    assert!(!secret.can_input());secret.output_complete();assert!(secret.can_input());assert!(!secret.waiting);assert!(!secret.closing);assert_eq!(secret.deadline,0);
    type_input(&mut secret,b"private password");assert!(secret.echo().is_empty());assert_eq!(secret.history_count(),0);
    type_input(&mut secret,b"\n");assert!(secret.pending);assert_eq!(&secret.input[..secret.length],b"private password");assert_eq!(secret.history_count(),0);
    let n=secret.echo().len();secret.consume_echo(n);secret.submitted(2);
    secret.deliver(Reply{sequence:2,outcome:execution::Outcome::new(),more:true,secret:true,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    secret.output_complete();type_input(&mut secret,b"\n");assert!(secret.pending);assert_eq!(secret.length,0);assert_eq!(secret.history_count(),0);
    let n=secret.echo().len();secret.consume_echo(n);secret.submitted(3);
    secret.deliver(Reply{sequence:3,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    secret.output_complete();assert!(secret.ready_to_close());assert_eq!(secret.status,0);
    let mut native=Terminal::new();native.start(Some(b"python /home/input.py"),0);native.submitted(1);
    native.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    native.output_complete();assert!(!native.can_input());type_input(&mut native,b"held in transport");assert_eq!(native.length,0);
    native.update(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:true,clear_history:false,close:false});
    native.output_complete();assert!(native.can_input());assert_eq!(native.deadline,0);type_input(&mut native,b"line one\n");
    assert!(native.pending);assert_eq!(&native.input[..native.length],b"line one");assert!(native.echo().is_empty());assert_eq!(native.history_count(),0);native.submitted(2);
    native.deliver(Reply{sequence:2,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    native.output_complete();assert!(!native.can_input());assert!(!native.closing);
    native.update(Reply{sequence:2,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:true,clear_history:false,close:false});
    native.output_complete();type_input(&mut native,b"\n");assert!(native.pending);assert_eq!(native.length,0);assert_eq!(native.history_count(),0);native.submitted(3);
    native.deliver(Reply{sequence:3,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    native.output_complete();assert!(native.ready_to_close());
    let mut shell=Terminal::new();shell.pty=true;shell.start(None,0);let n=shell.echo().len();shell.consume_echo(n);
    shell.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:true,clear_history:false,close:false},false);
    shell.output_complete();assert!(shell.echo().is_empty());shell.input_byte(b'x',1);assert_eq!(shell.echo(),b"\x1b[Kx");
    let n=shell.echo().len();shell.consume_echo(n);
    shell.update(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:true,clear_history:false,close:false});
    shell.output_complete();assert_eq!(&shell.input[..shell.length],b"x");assert!(shell.echo().is_empty());
    shell.deliver(Reply{sequence:0,outcome:execution::Outcome::new(),more:false,secret:true,private:true,raw:true,read_input:false,clear_history:true,close:true},true);
    shell.output_complete();assert!(shell.read_input);assert!(!shell.secret);assert!(!shell.raw);assert!(!shell.private);assert!(!shell.closing);assert_eq!(&shell.input[..shell.length],b"x");
}

// ------------------------=
// FUNC: output_boundaries
// DESC: Verifies terminal EOF waits for native completion and PTY output preserves existing CRLF even across output chunks.
// ------------------=
fn output_boundaries(){
    use runtime::console_remote::Reply;
    let mut terminal=Terminal::new();terminal.start(None,0);let n=terminal.echo().len();terminal.consume_echo(n);terminal.closing=true;
    terminal.deliver(Reply{sequence:1,outcome:execution::Outcome::new(),more:true,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false},false);
    terminal.output_complete();assert!(!terminal.waiting);assert!(!terminal.ready_to_close());
    terminal.update(Reply{sequence:1,outcome:execution::Outcome::new(),more:false,secret:false,private:false,raw:false,read_input:false,clear_history:false,close:false});
    terminal.output_complete();assert!(terminal.ready_to_close());
    let mut pty=Terminal::new();pty.pty=true;pty.start(Some(b"edit file"),0);
    let mut result=execution::Outcome::new();result.stdout.write(&[b'x';510]);result.stdout.write(b"\r\n\n");pty.complete(result);
    pty.stage_output();assert_eq!(pty.wire_length,511);assert_eq!(pty.wire[510],13);pty.wire_offset=pty.wire_length;
    pty.stage_output();assert_eq!(&pty.wire[..pty.wire_length],&[10,13,10]);
}
