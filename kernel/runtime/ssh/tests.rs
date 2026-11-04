extern crate std;
use super::*;
use core::cell::Cell;
use sunset::{CliEvent, Client};

// ------------------------=
// FUNC: generated_account_key_is_openssh_compatible
// DESC: Verifies fail-closed entropy, independent keys and actual macOS OpenSSH decoding of generated private material.
// ------------------=
#[test]
fn generated_account_key_is_openssh_compatible() {
    use std::{fs, os::unix::fs::OpenOptionsExt, io::Write, process::Command};
    use base64ct::{Base64, Encoding};
    assert!(matches!(userkey::UserKeyPair::generate(|_| false), Err(Failure::EntropyUnavailable)));
    let keys = userkey::UserKeyPair::generate(|bytes| { bytes.fill(41); true }).unwrap();
    let other = userkey::UserKeyPair::generate(|bytes| { bytes.fill(42); true }).unwrap();
    assert_ne!(keys.public_key(), other.public_key());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../build/ssh-key-format-test");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("id_ed25519");
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&path).unwrap();
    file.write_all(keys.private_file()).unwrap(); drop(file);
    let output = Command::new("/usr/bin/ssh-keygen").arg("-y").arg("-f").arg(&path).output().unwrap();
    fs::remove_file(&path).unwrap();
    assert!(output.status.success());
    let text = core::str::from_utf8(&output.stdout).unwrap();
    let mut blob = [0; 51];
    let decoded = Base64::decode(text.split_ascii_whitespace().nth(1).unwrap(), &mut blob).unwrap();
    assert_eq!(decoded.len(), 51);
    assert_eq!(decoded[19..], keys.public_key());
    let text = core::str::from_utf8(keys.public_file()).unwrap();
    let mut file_blob = [0; 51];
    assert_eq!(Base64::decode(text.split_ascii_whitespace().nth(1).unwrap(), &mut file_blob).unwrap(), decoded);
}

struct Authority { enabled: Cell<bool>, key: [u8; 32] }
impl IdentityAuthority for Authority {
    // ------------------------=
    // FUNC: authorized_key
    // DESC: Provides a revocable native identity boundary for protocol tests.
    // ------------------=
    fn authorized_key(&self, name: &[u8], key: &[u8; 32]) -> Option<Principal> {
        (self.enabled.get() && name == b"alice" && key == &self.key)
            .then_some(Principal { user: [7; 16], public_key: *key })
    }
    // ------------------------=
    // FUNC: still_authorized
    // DESC: Simulates key revocation without altering the transport or its signed packets.
    // ------------------=
    fn still_authorized(&self, principal: Principal) -> bool {
        self.enabled.get() && principal.user == [7; 16] && principal.public_key == self.key
    }
}
// ------------------------=
// FUNC: key
// DESC: Uses fixed test-only signing seeds; production keys must come from validated native entropy.
// ------------------=
fn key(value: u8) -> SignKey { SignKey::Ed25519(ed25519_dalek::SigningKey::from_bytes(&[value; 32])) }
// ------------------------=
// FUNC: public
// DESC: Extracts the raw Ed25519 verification key for native identity registration.
// ------------------=
fn public(key: &SignKey) -> [u8; 32] {
    match key.pubkey() { PubKey::Ed25519(k) => k.key.0, _ => panic!() }
}
// ------------------------=
// FUNC: exchange
// DESC: Moves bounded byte prefixes between two real SSH engines, preserving partial consumption.
// ------------------=
fn exchange(client: &mut Runner<'_, Client>, server: &mut Transport<'_>, authority: &Authority, now: u64, wire: &mut [std::vec::Vec<u8>; 2]) {
    let bytes = client.output_buf();
    let n = bytes.len();
    wire[0].extend_from_slice(bytes);
    client.consume_output(n);
    let bytes = server.output(authority, now).unwrap();
    let n = bytes.len();
    wire[1].extend_from_slice(bytes);
    server.consume_output(n).unwrap();
    assert!(wire.iter().all(|queue| queue.len() <= 2 * PACKET_BUFFER));
    let n = server.input(&wire[0], authority, now).unwrap();
    wire[0].drain(..n);
    let n = client.input(&wire[1]).unwrap();
    wire[1].drain(..n);
}
// ------------------------=
// FUNC: authenticated_transport_rejects_channels_and_observes_revocation
// DESC: Exercises signed auth, encrypted transport, denied session requests, and immediate key revocation.
// ------------------=
#[test]
fn authenticated_transport_rejects_channels_and_observes_revocation() {
    let host = key(1);
    let user = key(2);
    let authority = Authority { enabled: Cell::new(true), key: public(&user) };
    let (mut si, mut so, mut ci, mut co) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    let mut client = Runner::new_client(&mut ci, &mut co);
    let mut wire = [std::vec::Vec::new(), std::vec::Vec::new()];
    let mut client_authenticated = false;
    for now in 0..1000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        server.progress(&authority, &host, now).unwrap();
        match client.progress().unwrap() {
            Event::Cli(CliEvent::Hostkey(e)) => {
                assert_eq!(public(&host), match e.hostkey().unwrap() { PubKey::Ed25519(k) => k.key.0, _ => panic!() });
                e.accept().unwrap();
            }
            Event::Cli(CliEvent::Username(e)) => e.username("alice").unwrap(),
            Event::Cli(CliEvent::Pubkey(e)) => e.pubkey(user.clone()).unwrap(),
            Event::Cli(CliEvent::Password(e)) => e.skip().unwrap(),
            Event::Cli(CliEvent::Authenticated) => client_authenticated = true,
            Event::None | Event::Progressed | Event::Cli(CliEvent::PollAgain) => (),
            _ => panic!("unexpected client transition"),
        }
        if client_authenticated { break; }
    }
    assert!(client_authenticated);
    assert_eq!(server.phase(), Phase::Authenticated);
    assert_eq!(server.principal(), Some(Principal { user: [7; 16], public_key: authority.key }));
    let channel = client.open_client_session().unwrap();
    let mut denied = false;
    for now in 1000..1100 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        let event = server.progress(&authority, &host, now).unwrap();
        denied |= event == Diagnostic::ChannelDenied;
        client.progress().unwrap();
    }
    assert!(denied);
    assert_eq!(client.write_channel_ready(&channel, sunset::ChanData::Normal).unwrap(), None);
    authority.enabled.set(false);
    assert_eq!(server.progress(&authority, &host, 1100), Err(Failure::Revoked));
    assert_eq!(server.principal(), None);
    assert_eq!(server.output(&authority, 1101), Err(Failure::Revoked));
}
// ------------------------=
// FUNC: deadlines_configuration_and_bad_input_are_terminal
// DESC: Proves default-on policy, quota validation, hard handshake expiry and terminal parser failure.
// ------------------=
#[test]
fn deadlines_configuration_and_bad_input_are_terminal() {
    assert_eq!(Configuration::default(), Configuration { enabled: true, port: 22, max_connections: 2 });
    assert!(Configuration { enabled: true, port: 0, max_connections: 1 }.validate().is_err());
    assert!(Configuration { enabled: true, port: 22, max_connections: 5 }.validate().is_err());
    let host = key(1);
    let authority = Authority { enabled: Cell::new(false), key: [0; 32] };
    let (mut si, mut so) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    assert_eq!(server.progress(&authority, &host, HANDSHAKE_MS), Err(Failure::Deadline));
    assert_eq!(server.input(b"SSH-2.0-test\r\n", &authority, HANDSHAKE_MS + 1), Err(Failure::Deadline));
    drop(server);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    server.progress(&authority, &host, 0).unwrap();
    let pending = server.output(&authority, 0).unwrap().len();
    server.consume_output(pending).unwrap();
    assert!(server.input(b"SSH-1.0-invalid\r\n", &authority, 1).is_err());
    assert_eq!(server.principal(), None);
    assert!(matches!(server.phase(), Phase::Closed(_)));
}

struct Store { bytes: Option<[u8; hostkey::HOST_KEY_BYTES]>, protected: bool, writes: usize }
impl hostkey::HostKeyStorage for Store {
    // ------------------------=
    // FUNC: read_protected
    // DESC: Simulates durable storage independently of transport lifetime and reboot.
    // ------------------=
    fn read_protected(&mut self, out: &mut [u8; hostkey::HOST_KEY_BYTES]) -> Result<bool, Failure> {
        if !self.protected { return Err(Failure::HostKeyUnavailable); }
        if let Some(bytes) = self.bytes { *out = bytes; Ok(true) } else { Ok(false) }
    }
    // ------------------------=
    // FUNC: create_protected
    // DESC: Enforces create-only key persistence and records durable writes for lifecycle assertions.
    // ------------------=
    fn create_protected(&mut self, content: &[u8; hostkey::HOST_KEY_BYTES]) -> Result<(), Failure> {
        if !self.protected || self.bytes.is_some() { return Err(Failure::HostKeyUnavailable); }
        self.bytes = Some(*content);
        self.writes += 1;
        Ok(())
    }
}
// ------------------------=
// FUNC: protected_key_persists_and_storage_or_entropy_failures_never_rotate_it
// DESC: Verifies reload without entropy, create-once persistence, corruption rejection and fail-closed protection.
// ------------------=
#[test]
fn protected_key_persists_and_storage_or_entropy_failures_never_rotate_it() {
    let mut store = Store { bytes: None, protected: true, writes: 0 };
    assert!(matches!(hostkey::load_or_create(&mut store, |_| false), Err(Failure::EntropyUnavailable)));
    assert_eq!(store.writes, 0);
    let first = hostkey::load_or_create(&mut store, |bytes| { bytes.fill(9); true }).unwrap();
    let second = hostkey::load_or_create(&mut store, |_| panic!("must not rotate")).unwrap();
    assert_eq!(public(&first), public(&second));
    assert_eq!(store.writes, 1);
    store.protected = false;
    assert!(matches!(hostkey::load_or_create(&mut store, |_| panic!()), Err(Failure::HostKeyUnavailable)));
    store.protected = true;
    store.bytes.as_mut().unwrap()[10] ^= 1;
    assert!(matches!(hostkey::load_or_create(&mut store, |_| panic!()), Err(Failure::HostKeyUnavailable)));
    assert_eq!(store.writes, 1);
}

// ------------------------=
// FUNC: unregistered_public_key_never_authenticates
// DESC: Runs a real key exchange and signed client attempt using a key absent from native authorization.
// ------------------=
#[test]
fn unregistered_public_key_never_authenticates() {
    let host = key(1);
    let authority = Authority { enabled: Cell::new(true), key: public(&key(2)) };
    let (mut si, mut so, mut ci, mut co) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    let mut client = Runner::new_client(&mut ci, &mut co);
    let mut wire = [std::vec::Vec::new(), std::vec::Vec::new()];
    let mut rejected = false;
    let mut offered = false;
    for now in 0..1000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        rejected |= server.progress(&authority, &host, now).unwrap() == Diagnostic::AuthenticationRejected;
        match client.progress() {
            Ok(Event::Cli(CliEvent::Hostkey(e))) => e.accept().unwrap(),
            Ok(Event::Cli(CliEvent::Username(e))) => e.username("alice").unwrap(),
            Ok(Event::Cli(CliEvent::Pubkey(e))) if !offered => { e.pubkey(key(3)).unwrap(); offered = true; }
            Ok(Event::Cli(CliEvent::Pubkey(e))) => { let _ = e.skip(); },
            Ok(Event::Cli(CliEvent::Password(e))) => { let _ = e.skip(); },
            Ok(Event::Cli(CliEvent::Authenticated)) => panic!("invalid authentication"),
            Err(_) => break,
            _ => (),
        }
    }
    assert!(offered && rejected);
    assert_eq!(server.principal(), None);
    assert_eq!(server.phase(), Phase::Handshake);
}

// ------------------------=
// FUNC: local_management_parsing_is_bounded
// DESC: Checks typed operations, malformed public-key blobs, ephemeral-port exclusion and quota bounds.
// ------------------=
#[test]
fn local_management_parsing_is_bounded() {
    use management::{parse, Command};
    assert_eq!(parse(b"ssh status"), Ok(Command::Status));
    assert_eq!(parse(b"ssh enable 2222 4"), Ok(Command::Configure(Configuration { enabled: true, port: 2222, max_connections: 4 })));
    assert_eq!(parse(b"ssh disable"), Ok(Command::Configure(Configuration { enabled: false, port: 22, max_connections: 2 })));
    for bad in [b"ssh enable 49152".as_slice(), b"ssh enable 0", b"ssh enable 2222 5", b"ssh enable 65536",
        b"ssh status extra", b"ssh key add alice ssh-rsa AAAA", b"ssh key add alice ssh-ed25519 AAAA", b"ssh enable 22 -1"] {
        assert_eq!(parse(bad), Err(Failure::Configuration));
    }
    let public = public(&key(2));
    let mut blob = [0; 51];
    blob[..19].copy_from_slice(b"\0\0\0\x0bssh-ed25519\0\0\0\x20");
    blob[19..].copy_from_slice(&public);
    let mut encoded = [0; 68];
    use base64ct::Encoding;
    let text = base64ct::Base64::encode(&blob, &mut encoded).unwrap();
    let command = std::format!("ssh key add alice ssh-ed25519 {text}");
    assert_eq!(parse(command.as_bytes()), Ok(Command::Key { user: b"alice", key: public, revoke: false }));
}

// ------------------------=
// FUNC: authorized_exec_keeps_streams_and_exit_status_distinct
// DESC: Uses real encrypted channel packets to prove exec request delivery, stdout/stderr separation and actual numeric exit status.
// ------------------=
#[test]
fn authorized_exec_keeps_streams_and_exit_status_distinct() {
    let host = key(1); let user = key(2);
    let authority = Authority { enabled: Cell::new(true), key: public(&user) };
    let (mut si, mut so, mut ci, mut co) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    let mut client = Runner::new_client(&mut ci, &mut co);
    let mut wire = [std::vec::Vec::new(), std::vec::Vec::new()];
    let mut authenticated = false;
    for now in 0..1000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        server.progress(&authority, &host, now).unwrap();
        match client.progress().unwrap() {
            Event::Cli(CliEvent::Hostkey(e)) => e.accept().unwrap(),
            Event::Cli(CliEvent::Username(e)) => e.username("alice").unwrap(),
            Event::Cli(CliEvent::Pubkey(e)) => e.pubkey(user.clone()).unwrap(),
            Event::Cli(CliEvent::Authenticated) => authenticated = true,
            _ => (),
        }
        if authenticated { break; }
    }
    assert!(authenticated); server.authorize_console().unwrap();
    let channel = client.open_client_session().unwrap();
    let (mut stdout_sent, mut stderr_sent) = (0, 0);
    let mut stdout = std::vec::Vec::new(); let mut stderr = std::vec::Vec::new();
    let (mut requested, mut finished, mut status) = (false, false, None);
    for now in 1000..2000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        server.progress(&authority, &host, now).unwrap();
        if let Some(request) = server.take_console_request() {
            match request {
                ConsoleRequest::Exec { bytes, length } => { assert_eq!(&bytes[..length], b"system status"); requested = true; }
                _ => panic!("unexpected request type"),
            }
        }
        if requested && !finished {
            if stdout_sent < 3 { stdout_sent += server.write_console(&[1,2,3][stdout_sent..], false, &authority, now).unwrap(); }
            else if stderr_sent < 2 { stderr_sent += server.write_console(&[4,5][stderr_sent..], true, &authority, now).unwrap(); }
            else { finished = server.finish_console(37).unwrap(); }
        }
        match client.progress().unwrap() {
            Event::Cli(CliEvent::SessionOpened(mut event)) => event.exec("system status").unwrap(),
            Event::Cli(CliEvent::SessionExit(sunset::CliSessionExit::Status(value))) => status = Some(value),
            _ => (),
        }
        for (kind, output) in [(sunset::ChanData::Normal, &mut stdout), (sunset::ChanData::Stderr, &mut stderr)] {
            let mut byte = [0];
            match client.read_channel(&channel, kind, &mut byte) {
                Ok(1) => output.push(byte[0]), Ok(0) | Err(sunset::Error::ChannelEOF) => (),
                result => panic!("unexpected read result {result:?}"),
            }
        }
        if status.is_some() && stdout.len() == 3 && stderr.len() == 2 && client.is_channel_closed(&channel) { break; }
    }
    assert!(requested && finished); assert_eq!(stdout, [1,2,3]); assert_eq!(stderr, [4,5]); assert_eq!(status, Some(37));
    assert!(client.is_channel_eof(&channel));
    assert!(client.is_channel_closed(&channel));
}

// ------------------------=
// FUNC: authorized_shell_delivers_pty_resize_and_input
// DESC: Verifies encrypted PTY and resize requests and literal terminal interrupt bytes reach the native adapter.
// ------------------=
#[test]
fn authorized_shell_delivers_pty_resize_and_input() {
    let host = key(1); let user = key(2);
    let authority = Authority { enabled: Cell::new(true), key: public(&user) };
    let (mut si, mut so, mut ci, mut co) = ([0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER], [0; PACKET_BUFFER]);
    let mut server = Transport::new(&mut si, &mut so, 0).unwrap();
    let mut client = Runner::new_client(&mut ci, &mut co);
    let mut wire = [std::vec::Vec::new(), std::vec::Vec::new()];
    let mut authenticated = false;
    for now in 0..1000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        server.progress(&authority, &host, now).unwrap();
        match client.progress().unwrap() {
            Event::Cli(CliEvent::Hostkey(e)) => e.accept().unwrap(),
            Event::Cli(CliEvent::Username(e)) => e.username("alice").unwrap(),
            Event::Cli(CliEvent::Pubkey(e)) => e.pubkey(user.clone()).unwrap(),
            Event::Cli(CliEvent::Authenticated) => authenticated = true,
            _ => (),
        }
        if authenticated { break; }
    }
    assert!(authenticated); server.authorize_console().unwrap();
    let channel = client.open_client_session().unwrap();

    let (mut pty, mut shell, mut resized, mut sent_resize, mut sent_input) = (false, false, false, false, false);
    let mut received = std::vec::Vec::new();
    for now in 1000..2000 {
        exchange(&mut client, &mut server, &authority, now, &mut wire);
        server.progress(&authority, &host, now).unwrap();
        if let Some(request) = server.take_console_request() {
            match request {
                ConsoleRequest::Pty { columns, rows } => { assert_eq!((columns, rows), (80, 24)); pty = true; }
                ConsoleRequest::Shell => { assert!(pty); shell = true; }
                ConsoleRequest::Resize { columns, rows } => { assert_eq!((columns, rows), (132, 43)); resized = true; }
                _ => panic!("unexpected request type"),
            }
        }
        match client.progress().unwrap() {
            Event::Cli(CliEvent::SessionOpened(mut event)) => {
                event.pty(sunset::Pty { term: "xterm".try_into().unwrap(), cols: 80, rows: 24,
                    width: 0, height: 0, modes: Default::default() }).unwrap();
                event.shell().unwrap();
            }
            _ => (),
        }
        if shell && !sent_resize {
            client.term_window_change(&channel, &sunset::packets::WinChange { cols: 132, rows: 43, width: 0, height: 0 }).unwrap();
            sent_resize = true;
        }
        if resized && !sent_input {
            assert_eq!(client.write_channel(&channel, sunset::ChanData::Normal, b"help\r\x03").unwrap(), 6);
            sent_input = true;
        }
        let mut bytes = [0; 2];
        let n = server.read_console(&mut bytes, &authority, now).unwrap();
        received.extend_from_slice(&bytes[..n]);
        if received.len() == 6 { break; }
    }
    assert!(pty && shell && resized && sent_input);
    assert_eq!(received, b"help\r\x03");
}
